use crate::TransformConfig;
use crate::context::TransformerContext;
use crate::errors::Result;
use crate::geometry::{BoundingBox, Length};

use super::parser::PathParser;
use super::syntax::{PathSyntax, SvgPathSyntax};
use super::{Vec2, process_path_data};
use std::num::NonZeroU32;

impl PathParser {
    pub fn new_default(data: &str) -> Self {
        PathParser::new(data, &TransformConfig::default())
    }
}

fn assert_point_close(actual: Vec2, expected: Vec2, epsilon: f32) {
    assert!(
        actual.distance(expected) < epsilon,
        "got {actual:?}, expected {expected:?}"
    );
}

fn process_path_with_config(
    data: &str,
    cfg: &TransformConfig,
) -> Result<(String, Option<BoundingBox>)> {
    let ctx = TransformerContext::from_config(cfg);
    process_path_data(data, &ctx)
}

fn process_path_default(data: &str) -> Result<(String, Option<BoundingBox>)> {
    process_path_with_config(data, &TransformConfig::default())
}

fn process_path_with_limit(data: &str, limit: u32) -> Result<(String, Option<BoundingBox>)> {
    let cfg = TransformConfig {
        path_repeat_limit: limit,
        ..Default::default()
    };
    process_path_with_config(data, &cfg)
}

#[test]
fn test_ps_number() {
    let ctx = TransformerContext::default();
    let mut ps = SvgPathSyntax::new("123 4.5  -9.25");
    ps.skip_whitespace();
    assert_eq!(ps.read_number(&ctx).unwrap(), 123.);
    ps.skip_whitespace();
    assert_eq!(ps.read_number(&ctx).unwrap(), 4.5);
    ps.skip_whitespace();
    assert_eq!(ps.read_number(&ctx).unwrap(), -9.25);

    // should read as little as needed to allow valid parsing,
    // so numbers can be squished together providing the result
    // is unambiguous. See https://www.w3.org/TR/SVG11/paths.html#PathDataBNF
    let mut ps = SvgPathSyntax::new("123-4.5.25+5");
    assert_eq!(ps.read_number(&ctx).unwrap(), 123.);
    assert_eq!(ps.read_number(&ctx).unwrap(), -4.5);
    assert_eq!(ps.read_number(&ctx).unwrap(), 0.25);
    assert_eq!(ps.read_number(&ctx).unwrap(), 5.);

    // should support exponents
    let mut ps = SvgPathSyntax::new("1e3 -2E-2 +3.5e+2");
    assert_eq!(ps.read_number(&ctx).unwrap(), 1e3);
    assert_eq!(ps.read_number(&ctx).unwrap(), -2e-2);
    assert_eq!(ps.read_number(&ctx).unwrap(), 3.5e+2);
    // ... and without spaces; '1e3.5' is '1e3' followed by '.5'
    let mut ps = SvgPathSyntax::new("1e3.5-2E-2+3.5e+2");
    assert_eq!(ps.read_number(&ctx).unwrap(), 1e3);
    assert_eq!(ps.read_number(&ctx).unwrap(), 0.5);
    assert_eq!(ps.read_number(&ctx).unwrap(), -2e-2);
    assert_eq!(ps.read_number(&ctx).unwrap(), 3.5e+2);
}

#[test]
fn test_ps_flag() {
    let mut ps = SvgPathSyntax::new("0 1,1 0");
    assert_eq!(ps.read_flag().unwrap(), 0);
    assert_eq!(ps.read_flag().unwrap(), 1);
    assert_eq!(ps.read_flag().unwrap(), 1);
    assert_eq!(ps.read_flag().unwrap(), 0);

    // whitespace is not required around flags
    let mut ps = SvgPathSyntax::new("01");
    assert_eq!(ps.read_flag().unwrap(), 0);
    assert_eq!(ps.read_flag().unwrap(), 1);

    // only '0' and '1' are valid flags
    let mut ps = SvgPathSyntax::new("2");
    assert!(ps.read_flag().is_err());
    let mut ps = SvgPathSyntax::new("1.0");
    // will read the '1' as a flag, leaving '.0'
    assert_eq!(ps.read_flag().unwrap(), 1);
    assert!(ps.read_flag().is_err());
}

#[test]
fn test_ps_coord() {
    let ctx = TransformerContext::default();
    let mut ps = SvgPathSyntax::new("123 456");
    assert_eq!(ps.read_coord(&ctx).unwrap(), Vec2::new(123., 456.));

    let mut ps = SvgPathSyntax::new("123,456");
    assert_eq!(ps.read_coord(&ctx).unwrap(), Vec2::new(123., 456.));

    let mut ps = SvgPathSyntax::new("123 ,   456");
    assert_eq!(ps.read_coord(&ctx).unwrap(), Vec2::new(123., 456.));

    // Example from https://www.w3.org/TR/SVG11/paths.html#PathDataBNF
    // 'for the string "M 0.6.5" … the first coordinate will be "0.6" and
    // the second coordinate will be ".5".'
    let mut ps = SvgPathSyntax::new("0.6.5");
    assert_eq!(ps.read_coord(&ctx).unwrap(), Vec2::new(0.6, 0.5));
}

#[test]
fn test_ps_dynamic_scalars() {
    let mut ctx = TransformerContext::default();
    ctx.set_var("dx", "3.5");
    ctx.set_var("dy", "{{1 + 2}}");

    let mut ps = SvgPathSyntax::new("$dx ${dy} {{1 + 4}}");
    assert_eq!(ps.read_number(&ctx).unwrap(), 3.5);
    assert_eq!(ps.read_number(&ctx).unwrap(), 3.0);
    assert_eq!(ps.read_count(&ctx).unwrap(), 5);
}

#[test]
fn test_ps_dynamic_coord_expr() {
    let ctx = TransformerContext::default();
    let mut ps = SvgPathSyntax::new("{{p2r(10, 0)}}");
    assert_eq!(ps.read_coord(&ctx).unwrap(), Vec2::new(10., 0.));
}

#[test]
fn test_pp_move() {
    let mut pp = PathParser::new_default("M10 20");
    pp.evaluate_default().unwrap();
    assert_eq!(pp.position(), Some(Vec2::new(10., 20.)));

    // if the first command is 'm' (relative moveto) it is treated
    // as an absolute moveto.
    let mut pp = PathParser::new_default("m10 20");
    pp.evaluate_default().unwrap();
    assert_eq!(pp.position(), Some(Vec2::new(10., 20.)));

    // There can be multiple coordinates, in which case subsequent ones
    // are implicit 'line-to' coordinates
    let mut pp = PathParser::new_default("M10 20 100 200");
    pp.evaluate_default().unwrap();
    assert_eq!(pp.position(), Some(Vec2::new(100., 200.)));
    assert!(pp.at_end());

    // There can be multiple coordinates, in which case subsequent ones
    // are implicit 'line-to' coordinates
    let mut pp = PathParser::new_default("m10 20 100 200");
    pp.evaluate_default().unwrap();
    assert_eq!(pp.position(), Some(Vec2::new(110., 220.)));
    assert!(pp.at_end());

    // Example from spec - grammar section.
    let mut pp = PathParser::new_default("M 0.6.5");
    pp.evaluate_default().unwrap();
    assert_eq!(pp.position(), Some(Vec2::new(0.6, 0.5)));
    assert!(pp.at_end());

    //
    // Same again as above, but with lineto (L / l) this time.
    //
    let mut pp = PathParser::new_default("L10 20");
    pp.evaluate_default().unwrap();
    assert_eq!(pp.position(), Some(Vec2::new(10., 20.)));

    // if the first command is 'm' (relative moveto) it is treated
    // as an absolute moveto.
    let mut pp = PathParser::new_default("l10 20");
    pp.evaluate_default().unwrap();
    assert_eq!(pp.position(), Some(Vec2::new(10., 20.)));

    // There can be multiple coordinates, in which case subsequent ones
    // are implicit 'line-to' coordinates
    let mut pp = PathParser::new_default("L10 20 100 200");
    pp.evaluate_default().unwrap();
    assert_eq!(pp.position(), Some(Vec2::new(100., 200.)));
    assert!(pp.at_end());

    // There can be multiple coordinates, in which case subsequent ones
    // are implicit 'line-to' coordinates
    let mut pp = PathParser::new_default("l10 20 100 200");
    pp.evaluate_default().unwrap();
    assert_eq!(pp.position(), Some(Vec2::new(110., 220.)));
    assert!(pp.at_end());

    //
    // Horizontal lines
    //
    let mut pp = PathParser::new_default("H 10");
    pp.evaluate_default().unwrap();
    assert_eq!(pp.position(), Some(Vec2::new(10., 0.)));
    assert!(pp.at_end());

    let mut pp = PathParser::new_default("H 10 80 30");
    pp.evaluate_default().unwrap();
    assert_eq!(pp.position(), Some(Vec2::new(30., 0.)));
    assert!(pp.at_end());

    let mut pp = PathParser::new_default("h 10");
    pp.evaluate_default().unwrap();
    assert_eq!(pp.position(), Some(Vec2::new(10., 0.)));
    assert!(pp.at_end());

    let mut pp = PathParser::new_default("h 10 80 30");
    pp.evaluate_default().unwrap();
    assert_eq!(pp.position(), Some(Vec2::new(120., 0.)));
    assert!(pp.at_end());

    //
    // Vertical lines
    //
    let mut pp = PathParser::new_default("V 10");
    pp.evaluate_default().unwrap();
    assert_eq!(pp.position(), Some(Vec2::new(0., 10.)));
    assert!(pp.at_end());

    let mut pp = PathParser::new_default("V 10 80 30");
    pp.evaluate_default().unwrap();
    assert_eq!(pp.position(), Some(Vec2::new(0., 30.)));
    assert!(pp.at_end());

    let mut pp = PathParser::new_default("v 10");
    pp.evaluate_default().unwrap();
    assert_eq!(pp.position(), Some(Vec2::new(0., 10.)));
    assert!(pp.at_end());

    let mut pp = PathParser::new_default("v 10 80 30");
    pp.evaluate_default().unwrap();
    assert_eq!(pp.position(), Some(Vec2::new(0., 120.)));
    assert!(pp.at_end());
}

#[test]
fn test_pp_bearing() {
    // Absolute bearing on relative line.
    let mut pp = PathParser::new_default("M0 0B90l10 0");
    pp.evaluate_default().unwrap();
    assert_point_close(pp.position().unwrap(), Vec2::new(0., 10.), 1e-4);

    // Relative bearing updates should accumulate.
    let mut pp = PathParser::new_default("M0 0b60b30l10 0");
    pp.evaluate_default().unwrap();
    assert_point_close(pp.position().unwrap(), Vec2::new(0., 10.), 1e-4);

    // Bearing affects relative h/v.
    let mut pp = PathParser::new_default("M0 0B90h10v10");
    pp.evaluate_default().unwrap();
    assert_point_close(pp.position().unwrap(), Vec2::new(10., 10.), 1e-4);

    // Bearing affects relative m as well.
    let mut pp = PathParser::new_default("M0 0B45m10 0");
    pp.evaluate_default().unwrap();
    assert_point_close(pp.position().unwrap(), Vec2::new(7.071, 7.071), 1e-3);

    // Absolute commands are unaffected by bearing.
    let mut pp = PathParser::new_default("M0 0B90H10V20");
    pp.evaluate_default().unwrap();
    assert_eq!(pp.position(), Some(Vec2::new(10., 20.)));
}

#[test]
fn test_pp_repeat_updates_state_inline() {
    let mut pp = PathParser::new_default("M0 0 r2[ h10 v5 ]");
    pp.evaluate_default().unwrap();

    assert_eq!(pp.position(), Some(Vec2::new(20., 10.)));
    assert_eq!(pp.get_bbox(), Some(BoundingBox::new(0., 0., 20., 10.)));
}

#[test]
fn test_pp_bbox() {
    let mut pp = PathParser::new_default("M10 20 100 200 200 150");
    pp.evaluate_default().unwrap();
    assert_eq!(pp.get_bbox(), Some(BoundingBox::new(10., 20., 200., 200.)));

    let mut pp = PathParser::new_default("M10 20 M100 200 M200 150");
    pp.evaluate_default().unwrap();
    assert_eq!(pp.get_bbox(), Some(BoundingBox::new(10., 20., 200., 200.)));

    let mut pp = PathParser::new_default("M10 20 m100 200 m-1000 150");
    pp.evaluate_default().unwrap();
    assert_eq!(
        pp.get_bbox(),
        Some(BoundingBox::new(-890., 20., 110., 370.))
    );
}

#[test]
fn test_bezier_curve_bbox() {
    for (pd, exp) in [
        // Simple cubic curve with extrema
        ("M 0 0 c 10 20 30 20 40 0", [0., 0., 40., 15.]),
        // Multiple cubic curves - simple horizontal curves
        (
            "M 10 10 c 0 5 5 5 10 0 c 0 -5 5 -5 10 0",
            [10., 6.25, 30., 13.75],
        ),
        // Absolute cubic with clear extrema - symmetric arch
        ("M 0 0 C 0 40 40 40 40 0", [0., 0., 40., 30.]),
        // Simple S-curve
        ("M 0 0 C 20 0 20 20 40 20", [0., 0., 40., 20.]),
        // Smooth cubic with simple reflection
        ("M 0 0 C 10 0 20 20 30 20 s 20 0 30 0", [0., 0., 60., 20.]),
        // Smooth cubic without previous cubic (degenerate case)
        ("M 20 20 s 10 9 20 0", [20., 20., 40., 24.]),
        // Simple absolute smooth cubic
        (
            "M 0 0 C 0 20 20 20 20 0 S -15 -20 30 0",
            [0., -15., 30., 15.],
        ),
        // S command without previous cubic
        ("M 10 10 S 20 28 30 10", [10., 10., 30., 18.]),
        // Simple quadratic arch
        ("M 0 0 q 20 40 40 0", [0., 0., 40., 20.]),
        // Quadratic with no extrema (straight line case)
        ("M 10 10 q 10 10 20 20", [10., 10., 30., 30.]),
        // Absolute quadratic arch
        ("M 0 0 Q 20 40 40 0", [0., 0., 40., 20.]),
        // Quadratic dipping below
        ("M 0 20 Q 20 0 40 20", [0., 10., 40., 20.]),
        // Smooth quadratic following Q - symmetric waves
        ("M 0 0 Q 10 20 20 0 t 20 0", [0., -10., 40., 10.]),
        // t command without previous quadratic (degenerate)
        ("M 10 10 t 20 0", [10., 10., 30., 10.]),
        // Absolute smooth quadratic - symmetric arches
        ("M 0 0 Q 20 40 40 0 T 80 0", [0., -20., 80., 20.]),
        // T command without previous quadratic
        ("M 10 10 T 30 20", [10., 10., 30., 20.]),
        // oblique quadratic with different t values in x and y
        ("M 0 0 q 60 120 30 0", [0., 0., 40., 60.]),
    ] {
        let mut pp = PathParser::new_default(pd);
        pp.evaluate_default().unwrap();
        let exp_bbox = BoundingBox::new(exp[0], exp[1], exp[2], exp[3]);
        assert_eq!(pp.get_bbox(), Some(exp_bbox), "Failed for path: {pd}");
    }
}

#[test]
fn test_smooth_bezier_curve_bbox_state() {
    for (pd, exp) in [
        (
            "M 0 0 C 10 0 20 20 30 20 S 50 20 60 20 80 20 90 20",
            [0., 0., 90., 20.],
        ),
        (
            "M 0 0 C 0 40 40 40 40 0 L 50 0 S 90 0 90 0",
            [0., 0., 90., 30.],
        ),
        (
            "M 0 0 c 0 40 40 40 40 0 l 10 0 s 40 0 40 0",
            [0., 0., 90., 30.],
        ),
        ("M 0 0 Q 10 20 20 0 T 40 0 60 0", [0., -10., 60., 10.]),
        ("M 0 0 Q 10 20 20 0 L 30 0 T 50 0", [0., 0., 50., 10.]),
        ("M 0 0 q 10 20 20 0 l 10 0 t 20 0", [0., 0., 50., 10.]),
    ] {
        let mut pp = PathParser::new_default(pd);
        pp.evaluate_default().unwrap();
        let exp_bbox = BoundingBox::new(exp[0], exp[1], exp[2], exp[3]);
        assert_eq!(pp.get_bbox(), Some(exp_bbox), "Failed for path: {pd}");
    }
}

#[test]
fn test_arc_bbox() {
    for (pd, exp) in [
        // Simple semicircle arc - top half
        ("M 0 0 A 10 10 0 0 1 20 0", [0., -10., 20., 0.]),
        // Simple semicircle arc - bottom half
        ("M 0 0 A 10 10 0 0 0 20 0", [0., 0., 20., 10.]),
        // Quarter circle arc - first quadrant
        ("M 10 0 A 10 10 0 0 1 20 10", [10., 0., 20., 10.]),
        // Quarter circle arc - includes extrema at top
        ("M 0 10 A 10 10 0 0 1 10 0", [0., 0., 10., 10.]),
        // Small arc that doesn't include any extrema
        // ("M 5 0 A 10 10 0 0 1 15 0", [5., -1.34, 15., 0.]),
        // Elliptical arc - horizontal ellipse
        ("M 0 0 A 20 10 0 0 1 40 0", [0., -10., 40., 0.]),
        // Elliptical arc - vertical ellipse
        ("M 0 0 A 10 20 0 0 1 20 0", [0., -20., 20., 0.]),
        // Rotated ellipse - 45 degrees
        ("M 0 0 A 10 5 45 0 1 10 10", [0., 0., 10., 10.]),
        // Relative arc - simple quarter circle
        ("M 10 10 a 5 5 0 0 1 5 5", [10., 10., 15., 15.]),
        // Relative arc - semicircle
        ("M 5 5 a 10 10 0 0 1 20 0", [5., -5., 25., 5.]),
        // Large arc flag difference - same endpoints, different sweep
        ("M 0 0 A 10 10 0 1 1 20 0", [0., -10., 20., 0.]),
        // Sweep flag difference - clockwise vs counterclockwise
        ("M 0 0 A 10 10 0 0 0 20 0", [0., 0., 20., 10.]),
        // Full circle (start and end same) - should be degenerate
        ("M 10 10 A 5 5 0 0 1 10 10", [10., 10., 10., 10.]),
        // Nearly straight line arc
        // ("M 0 0 A 100 100 0 0 1 1 0", [0., 0., 1., 0.]),
        // Arc with very different radii
        ("M 0 0 A 50 2 0 0 1 100 0", [0., -2., 100., 0.]),
        // If the radii are too small they are scaled up;
        // check the bbox is too.
        ("M 0 0 A 10 5 45 1 1 100 0", [-12.5, -62.5, 100., 0.]),
    ] {
        let mut pp = PathParser::new_default(pd);
        pp.evaluate_default().unwrap();
        let exp_bbox = BoundingBox::new(exp[0], exp[1], exp[2], exp[3]);
        assert_eq!(pp.get_bbox(), Some(exp_bbox), "Failed for path: {pd}");
    }
}

#[test]
fn test_multiple_subpath_bbox() {
    let mut pp = PathParser::new_default("m0 0 20 20h-10zm 5 -10h20v-10zm20 10l20 30");
    pp.evaluate_default().unwrap();
    assert_eq!(pp.get_bbox(), Some(BoundingBox::new(0., -20., 45., 30.)));
}

#[test]
fn test_path_length() {
    // simple linear segments
    let mut pp = PathParser::new_default("m0 0h10v10h-10");
    assert_eq!(pp.full_length_default().unwrap(), 30.);

    // include diagonal line
    let mut pp = PathParser::new_default("m0 0h10v10z");
    assert!((pp.full_length_default().unwrap() - (20. + 10. * (2f32).sqrt())).abs() < 1e-4);

    // multiple subpaths - should ignore jumps
    let mut pp = PathParser::new_default("m0 0h10m 20 0v10");
    assert_eq!(pp.full_length_default().unwrap(), 20.);

    // multiple subpaths, start off origin
    let mut pp = PathParser::new_default("m12 45h10m 20 0v10");
    assert_eq!(pp.full_length_default().unwrap(), 20.);

    let mut pp = PathParser::new_default("M0 0 r2[ h10 v5 ]");
    assert_eq!(pp.full_length_default().unwrap(), 30.);
}

#[test]
fn test_point_at_offset_linear() {
    let mut pp = PathParser::new_default("M 0 0 h10v10h10");

    // at start
    assert_eq!(
        pp.point_at_offset(Length::Absolute(0.)).unwrap(),
        Vec2::new(0., 0.)
    );
    // at end
    assert_eq!(
        pp.point_at_offset(Length::Absolute(30.)).unwrap(),
        Vec2::new(20., 10.)
    );
    // halfway along first segment
    assert_eq!(
        pp.point_at_offset(Length::Absolute(5.)).unwrap(),
        Vec2::new(5., 0.)
    );
    // halfway along second segment
    assert_eq!(
        pp.point_at_offset(Length::Absolute(15.)).unwrap(),
        Vec2::new(10., 5.)
    );
    // halfway along third segment
    assert_eq!(
        pp.point_at_offset(Length::Absolute(25.)).unwrap(),
        Vec2::new(15., 10.)
    );
    // beyond end should clamp to end point
    assert_eq!(
        pp.point_at_offset(Length::Absolute(35.)).unwrap(),
        Vec2::new(20., 10.)
    );
    // negative offset should clamp to start point
    assert_eq!(
        pp.point_at_offset(Length::Absolute(-5.)).unwrap(),
        Vec2::new(0., 0.)
    );

    // ratios
    assert_eq!(
        pp.point_at_offset(Length::Ratio(0.)).unwrap(),
        Vec2::new(0., 0.)
    );
    assert_eq!(
        pp.point_at_offset(Length::Ratio(0.5)).unwrap(),
        Vec2::new(10., 5.)
    );
    assert_eq!(
        pp.point_at_offset(Length::Ratio(1.)).unwrap(),
        Vec2::new(20., 10.)
    );

    // rationals
    assert_eq!(
        pp.point_at_offset(Length::Rational(0, NonZeroU32::new(1).unwrap()))
            .unwrap(),
        Vec2::new(0., 0.)
    );
    assert_eq!(
        pp.point_at_offset(Length::Rational(1, NonZeroU32::new(3).unwrap()))
            .unwrap(),
        Vec2::new(10., 0.)
    );
    assert_eq!(
        pp.point_at_offset(Length::Rational(1, NonZeroU32::new(2).unwrap()))
            .unwrap(),
        Vec2::new(10., 5.)
    );
    assert_eq!(
        pp.point_at_offset(Length::Rational(2, NonZeroU32::new(3).unwrap()))
            .unwrap(),
        Vec2::new(10., 10.)
    );
    assert_eq!(
        pp.point_at_offset(Length::Rational(1, NonZeroU32::new(1).unwrap()))
            .unwrap(),
        Vec2::new(20., 10.)
    );

    // test 'z'
    assert_eq!(
        PathParser::new_default("m0 0h10v10h-10z")
            .point_at_offset(Length::Absolute(35.))
            .unwrap(),
        Vec2::new(0., 5.)
    );
}

#[test]
fn test_point_at_offset_with_bearing_command() {
    // Bearing is a zero-length command and should not affect interpolation.
    let mut pp = PathParser::new_default("M 0 0 B 45 l 10 0");
    assert_point_close(
        pp.point_at_offset(Length::Ratio(0.5)).unwrap(),
        Vec2::new(3.536, 3.536),
        1e-3,
    );
}

#[test]
fn test_point_at_offset_with_repeat() {
    let mut pp = PathParser::new_default("M0 0 r2[ h10 v5 ]");
    assert_eq!(
        pp.point_at_offset(Length::Absolute(25.)).unwrap(),
        Vec2::new(20., 5.)
    );
}

#[test]
fn test_process_path_data_with_bearing() {
    let input = "M0 0 b-45 h2 b90 h2 b90 h2 z";
    let (output, bbox) = process_path_default(input).unwrap();
    assert_eq!(output, "M0 0 l1.414 -1.414l1.414 1.414l-1.414 1.414z");
    assert!(bbox.is_some());
}

#[test]
fn test_process_path_data_passthrough_when_unaffected() {
    let input = "M0 0L1 1";
    let (output, bbox) = process_path_default(input).unwrap();
    assert_eq!(output, input);
    assert!(bbox.is_some());
}

#[test]
fn test_process_path_data_with_dynamic_scalars() {
    let mut ctx = TransformerContext::default();
    ctx.set_var("dx", "3");
    ctx.set_var("dy", "{{1 + 2}}");

    let (output, bbox) = process_path_data("M0 0 l $dx ${dy}", &ctx).unwrap();
    assert_eq!(output, "M0 0 l 3 3");
    assert_eq!(bbox, Some(BoundingBox::new(0., 0., 3., 3.)));
}

#[test]
fn test_process_path_data_with_set_var() {
    let ctx = TransformerContext::default();
    let (output, bbox) = process_path_data("M0 0 :i 1 r3[l $i 0 :i {{$i + 1}}]", &ctx).unwrap();
    assert_eq!(output, "M0 0 l 1 0 l 2 0 l 3 0");
    assert_eq!(bbox, Some(BoundingBox::new(0., 0., 6., 0.)));
}

#[test]
fn test_process_path_data_with_repeat() {
    let input = "M0 0 r3[ l10 0 ] l5 0";
    let (output, bbox) = process_path_with_limit(input, 100).unwrap();
    assert_eq!(output, "M0 0 l10 0 l10 0 l10 0 l5 0");
    assert_eq!(bbox, Some(BoundingBox::new(0., 0., 35., 0.)));
}

#[test]
fn test_process_path_data_with_nested_repeat() {
    let input = "M0 0 r3[ h3 r2[ l10 0 ] ] l5 0";
    let (output, bbox) = process_path_with_limit(input, 100).unwrap();
    assert_eq!(
        output,
        "M0 0 h3 l10 0 l10 0 h3 l10 0 l10 0 h3 l10 0 l10 0 l5 0"
    );
    assert_eq!(bbox, Some(BoundingBox::new(0., 0., 74., 0.)));
}

#[test]
fn test_process_path_data_repeat_limit() {
    let input = "M0 0 r1000[ l10 0 ]";
    assert!(process_path_with_limit(input, 999).is_err());
    assert!(process_path_with_limit(input, 1000).is_ok());

    let input = "M0 0 r10[ r100[ l10 0 ] ]";
    assert!(process_path_with_limit(input, 500).is_err());

    let input = "M0 0 r999[r999[r999[r999[r999[h1]]]]]";
    assert!(process_path_with_limit(input, 1_000_000).is_err());

    let input = "M0 0 r10[h1] r10[v1]";
    assert!(process_path_with_limit(input, 10).is_ok());

    let input = "M0 0 r10[h1] r10[r2[v1]] r0[h1] h1 r3[v1 h1]";
    assert!(process_path_with_limit(input, 19).is_err());
    assert!(process_path_with_limit(input, 20).is_ok());
}

#[test]
fn test_process_path_data_repeat_re_evaluates_expressions() {
    let cfg = TransformConfig {
        seed: 1234,
        path_repeat_limit: 10,
        ..Default::default()
    };
    let input = "M0 0 r5[ l {{random()}} {{random()}} ]";

    let (output1, bbox1) = process_path_with_config(input, &cfg).unwrap();
    let (output2, bbox2) = process_path_with_config(input, &cfg).unwrap();

    assert_eq!(output1, output2);
    assert_eq!(bbox1, bbox2);
    assert!(!output1.contains("{{"));

    let steps: Vec<_> = output1.split('l').skip(1).collect();
    assert_eq!(steps.len(), 5);
    assert_ne!(steps[0], steps[1]);
}

#[test]
fn test_process_path_data_repeat_re_evaluates_variables() {
    let cfg = TransformConfig {
        seed: 1234,
        path_repeat_limit: 10,
        ..Default::default()
    };
    let input = "M0 0 r5[ l $step 1 ]";

    let mut ctx1 = TransformerContext::from_config(&cfg);
    ctx1.set_var("step", "{{random()}}");
    let (output1, bbox1) = process_path_data(input, &ctx1).unwrap();

    let mut ctx2 = TransformerContext::from_config(&cfg);
    ctx2.set_var("step", "{{random()}}");
    let (output2, bbox2) = process_path_data(input, &ctx2).unwrap();

    assert_eq!(output1, output2);
    assert_eq!(bbox1, bbox2);

    let steps: Vec<_> = output1.split('l').skip(1).collect();
    assert_eq!(steps.len(), 5);
    assert_ne!(steps[0], steps[1]);
}

#[test]
fn test_point_at_offset_curve() {
    // test quadratic bezier
    assert_point_close(
        PathParser::new_default("M 0 0 Q 20 40 40 0")
            .point_at_offset(Length::Ratio(0.5))
            .unwrap(),
        Vec2::new(20., 20.),
        1e-3,
    );

    // test cubic bezier
    assert_point_close(
        PathParser::new_default("M 0 0 C 0 40 40 40 40 0")
            .point_at_offset(Length::Ratio(0.5))
            .unwrap(),
        Vec2::new(20., 30.),
        1e-3,
    );

    // test arc (radius 40; center 50,50; start at 10,50 travel ccw)
    // Note a single arc command cannot represent a full circle.
    let mut pp = PathParser::new_default("M 10 50 A 40 40 0 1 0 90 50 A 40 40 0 1 0 10 50");
    for (offset, expected) in [
        (
            Length::Rational(0, NonZeroU32::new(4).unwrap()),
            Vec2::new(10., 50.),
        ),
        (
            Length::Rational(1, NonZeroU32::new(4).unwrap()),
            Vec2::new(50., 90.),
        ),
        (
            Length::Rational(2, NonZeroU32::new(4).unwrap()),
            Vec2::new(90., 50.),
        ),
        (
            Length::Rational(3, NonZeroU32::new(4).unwrap()),
            Vec2::new(50., 10.),
        ),
        (
            Length::Rational(4, NonZeroU32::new(4).unwrap()),
            Vec2::new(10., 50.),
        ),
    ] {
        let point = pp.point_at_offset(offset).unwrap();
        assert_point_close(point, expected, 1e-4);
    }
}

#[test]
fn test_point_at_offset_smooth_curve() {
    fn point_at_command_ratio(path: &str, command_index: usize, ratio: f32) -> Vec2 {
        let mut measure = PathParser::new_default(path);
        measure.skip_whitespace();

        for _ in 0..command_index {
            measure.process_instruction_default().unwrap();
        }

        let old_length = measure.length_so_far();
        measure.process_instruction_default().unwrap();
        let contribution = measure.length_so_far() - old_length;

        PathParser::new_default(path)
            .point_at_offset(Length::Absolute(old_length + contribution * ratio))
            .unwrap()
    }

    assert_point_close(
        point_at_command_ratio("M 0 0 C 10 0 20 20 30 20 s 20 0 30 0", 2, 0.5),
        Vec2::new(45., 20.),
        6.0,
    );

    assert_point_close(
        point_at_command_ratio("M 0 0 C 10 0 20 20 30 20 S 50 20 60 20 80 20 90 20", 3, 0.5),
        Vec2::new(75., 20.),
        6.0,
    );

    assert_point_close(
        point_at_command_ratio("M 0 0 C 0 40 40 40 40 0 L 50 0 S 90 0 90 0", 3, 0.5),
        Vec2::new(70., 0.),
        6.0,
    );

    assert_point_close(
        point_at_command_ratio("M 0 0 Q 10 20 20 0 t 20 0", 2, 0.5),
        Vec2::new(30., -10.),
        3.0,
    );

    assert_point_close(
        point_at_command_ratio("M 0 0 Q 10 20 20 0 T 40 0 60 0", 3, 0.5),
        Vec2::new(50., 10.),
        3.0,
    );

    assert_point_close(
        point_at_command_ratio("M 0 0 Q 10 20 20 0 L 30 0 T 50 0", 3, 0.5),
        Vec2::new(35., 0.),
        6.0,
    );
}

#[test]
fn test_path_length_curve_approximation() {
    for (pd, expected, epsilon) in [
        ("M 0 0 Q 20 40 40 0", 59.16, 1.0),
        ("M 0 0 C 0 40 40 40 40 0", 80.0, 1.0),
        ("M 0 0 A 10 10 0 0 1 20 0", 31.42, 1.0),
        ("M 0 0 A 20 10 0 0 1 40 0", 48.44, 2.0),
    ] {
        let mut pp = PathParser::new_default(pd);
        let actual = pp.full_length_default().unwrap();
        assert!(
            (actual - expected).abs() < epsilon,
            "Failed for path: {pd}; got {actual}, expected about {expected}"
        );
    }
}

#[test]
fn test_point_at_offset_arc_with_scaled_radii() {
    let mut pp = PathParser::new_default("M 0 0 A 13 10 0 1 0 27 1");

    let point = pp.point_at_offset(Length::Ratio(0.)).unwrap();
    assert_point_close(point, Vec2::new(0., 0.), 1e-4);

    let point = pp.point_at_offset(Length::Ratio(1.)).unwrap();
    assert_point_close(point, Vec2::new(27., 1.), 1e-4);
}
