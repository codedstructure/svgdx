use crate::geometry::{BoundingBox, Length};

use super::bbox::PathParser;
use super::syntax::{PathSyntax, SvgPathSyntax};
use std::num::NonZeroU32;

#[test]
fn test_ps_number() {
    let mut ps = SvgPathSyntax::new("123 4.5  -9.25");
    ps.skip_whitespace();
    assert_eq!(ps.read_number().unwrap(), 123.);
    ps.skip_whitespace();
    assert_eq!(ps.read_number().unwrap(), 4.5);
    ps.skip_whitespace();
    assert_eq!(ps.read_number().unwrap(), -9.25);

    // should read as little as needed to allow valid parsing,
    // so numbers can be squished together providing the result
    // is unambiguous. See https://www.w3.org/TR/SVG11/paths.html#PathDataBNF
    let mut ps = SvgPathSyntax::new("123-4.5.25+5");
    assert_eq!(ps.read_number().unwrap(), 123.);
    assert_eq!(ps.read_number().unwrap(), -4.5);
    assert_eq!(ps.read_number().unwrap(), 0.25);
    assert_eq!(ps.read_number().unwrap(), 5.);

    // should support exponents
    let mut ps = SvgPathSyntax::new("1e3 -2E-2 +3.5e+2");
    assert_eq!(ps.read_number().unwrap(), 1e3);
    assert_eq!(ps.read_number().unwrap(), -2e-2);
    assert_eq!(ps.read_number().unwrap(), 3.5e+2);
    // ... and without spaces; '1e3.5' is '1e3' followed by '.5'
    let mut ps = SvgPathSyntax::new("1e3.5-2E-2+3.5e+2");
    assert_eq!(ps.read_number().unwrap(), 1e3);
    assert_eq!(ps.read_number().unwrap(), 0.5);
    assert_eq!(ps.read_number().unwrap(), -2e-2);
    assert_eq!(ps.read_number().unwrap(), 3.5e+2);
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
    let mut ps = SvgPathSyntax::new("123 456");
    assert_eq!(ps.read_coord().unwrap(), (123., 456.));

    let mut ps = SvgPathSyntax::new("123,456");
    assert_eq!(ps.read_coord().unwrap(), (123., 456.));

    let mut ps = SvgPathSyntax::new("123 ,   456");
    assert_eq!(ps.read_coord().unwrap(), (123., 456.));

    // Example from https://www.w3.org/TR/SVG11/paths.html#PathDataBNF
    // 'for the string "M 0.6.5" … the first coordinate will be "0.6" and
    // the second coordinate will be ".5".'
    let mut ps = SvgPathSyntax::new("0.6.5");
    assert_eq!(ps.read_coord().unwrap(), (0.6, 0.5));
}

#[test]
fn test_pp_move() {
    let mut pp = PathParser::new("M10 20");
    pp.evaluate().unwrap();
    assert_eq!(pp.position(), Some((10., 20.)));

    // if the first command is 'm' (relative moveto) it is treated
    // as an absolute moveto.
    let mut pp = PathParser::new("m10 20");
    pp.evaluate().unwrap();
    assert_eq!(pp.position(), Some((10., 20.)));

    // There can be multiple coordinates, in which case subsequent ones
    // are implicit 'line-to' coordinates
    let mut pp = PathParser::new("M10 20 100 200");
    pp.evaluate().unwrap();
    assert_eq!(pp.position(), Some((100., 200.)));
    assert!(pp.at_end());

    // There can be multiple coordinates, in which case subsequent ones
    // are implicit 'line-to' coordinates
    let mut pp = PathParser::new("m10 20 100 200");
    pp.evaluate().unwrap();
    assert_eq!(pp.position(), Some((110., 220.)));
    assert!(pp.at_end());

    // Example from spec - grammar section.
    let mut pp = PathParser::new("M 0.6.5");
    pp.evaluate().unwrap();
    assert_eq!(pp.position(), Some((0.6, 0.5)));
    assert!(pp.at_end());

    //
    // Same again as above, but with lineto (L / l) this time.
    //
    let mut pp = PathParser::new("L10 20");
    pp.evaluate().unwrap();
    assert_eq!(pp.position(), Some((10., 20.)));

    // if the first command is 'm' (relative moveto) it is treated
    // as an absolute moveto.
    let mut pp = PathParser::new("l10 20");
    pp.evaluate().unwrap();
    assert_eq!(pp.position(), Some((10., 20.)));

    // There can be multiple coordinates, in which case subsequent ones
    // are implicit 'line-to' coordinates
    let mut pp = PathParser::new("L10 20 100 200");
    pp.evaluate().unwrap();
    assert_eq!(pp.position(), Some((100., 200.)));
    assert!(pp.at_end());

    // There can be multiple coordinates, in which case subsequent ones
    // are implicit 'line-to' coordinates
    let mut pp = PathParser::new("l10 20 100 200");
    pp.evaluate().unwrap();
    assert_eq!(pp.position(), Some((110., 220.)));
    assert!(pp.at_end());

    //
    // Horizontal lines
    //
    let mut pp = PathParser::new("H 10");
    pp.evaluate().unwrap();
    assert_eq!(pp.position(), Some((10., 0.)));
    assert!(pp.at_end());

    let mut pp = PathParser::new("H 10 80 30");
    pp.evaluate().unwrap();
    assert_eq!(pp.position(), Some((30., 0.)));
    assert!(pp.at_end());

    let mut pp = PathParser::new("h 10");
    pp.evaluate().unwrap();
    assert_eq!(pp.position(), Some((10., 0.)));
    assert!(pp.at_end());

    let mut pp = PathParser::new("h 10 80 30");
    pp.evaluate().unwrap();
    assert_eq!(pp.position(), Some((120., 0.)));
    assert!(pp.at_end());

    //
    // Vertical lines
    //
    let mut pp = PathParser::new("V 10");
    pp.evaluate().unwrap();
    assert_eq!(pp.position(), Some((0., 10.)));
    assert!(pp.at_end());

    let mut pp = PathParser::new("V 10 80 30");
    pp.evaluate().unwrap();
    assert_eq!(pp.position(), Some((0., 30.)));
    assert!(pp.at_end());

    let mut pp = PathParser::new("v 10");
    pp.evaluate().unwrap();
    assert_eq!(pp.position(), Some((0., 10.)));
    assert!(pp.at_end());

    let mut pp = PathParser::new("v 10 80 30");
    pp.evaluate().unwrap();
    assert_eq!(pp.position(), Some((0., 120.)));
    assert!(pp.at_end());
}

#[test]
fn test_pp_bbox() {
    let mut pp = PathParser::new("M10 20 100 200 200 150");
    pp.evaluate().unwrap();
    assert_eq!(pp.get_bbox(), Some(BoundingBox::new(10., 20., 200., 200.)));

    let mut pp = PathParser::new("M10 20 M100 200 M200 150");
    pp.evaluate().unwrap();
    assert_eq!(pp.get_bbox(), Some(BoundingBox::new(10., 20., 200., 200.)));

    let mut pp = PathParser::new("M10 20 m100 200 m-1000 150");
    pp.evaluate().unwrap();
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
        let mut pp = PathParser::new(pd);
        pp.evaluate().unwrap();
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
        let mut pp = PathParser::new(pd);
        pp.evaluate().unwrap();
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
        let mut pp = PathParser::new(pd);
        pp.evaluate().unwrap();
        let exp_bbox = BoundingBox::new(exp[0], exp[1], exp[2], exp[3]);
        assert_eq!(pp.get_bbox(), Some(exp_bbox), "Failed for path: {pd}");
    }
}

#[test]
fn test_multiple_subpath_bbox() {
    let mut pp = PathParser::new("m0 0 20 20h-10zm 5 -10h20v-10zm20 10l20 30");
    pp.evaluate().unwrap();
    assert_eq!(pp.get_bbox(), Some(BoundingBox::new(0., -20., 45., 30.)));
}

#[test]
fn test_path_length() {
    // simple linear segments
    let mut pp = PathParser::new("m0 0h10v10h-10");
    assert_eq!(pp.full_length().unwrap(), 30.);

    // include diagonal line
    let mut pp = PathParser::new("m0 0h10v10z");
    assert!((pp.full_length().unwrap() - (20. + 10. * (2f32).sqrt())).abs() < 1e-4);

    // multiple subpaths - should ignore jumps
    let mut pp = PathParser::new("m0 0h10m 20 0v10");
    assert_eq!(pp.full_length().unwrap(), 20.);

    // multiple subpaths, start off origin
    let mut pp = PathParser::new("m12 45h10m 20 0v10");
    assert_eq!(pp.full_length().unwrap(), 20.);
}

#[test]
fn test_point_at_offset_linear() {
    let mut pp = PathParser::new("M 0 0 h10v10h10");

    // at start
    assert_eq!(pp.point_at_offset(Length::Absolute(0.)).unwrap(), (0., 0.));
    // at end
    assert_eq!(
        pp.point_at_offset(Length::Absolute(30.)).unwrap(),
        (20., 10.)
    );
    // halfway along first segment
    assert_eq!(pp.point_at_offset(Length::Absolute(5.)).unwrap(), (5., 0.));
    // halfway along second segment
    assert_eq!(
        pp.point_at_offset(Length::Absolute(15.)).unwrap(),
        (10., 5.)
    );
    // halfway along third segment
    assert_eq!(
        pp.point_at_offset(Length::Absolute(25.)).unwrap(),
        (15., 10.)
    );
    // beyond end should clamp to end point
    assert_eq!(
        pp.point_at_offset(Length::Absolute(35.)).unwrap(),
        (20., 10.)
    );
    // negative offset should clamp to start point
    assert_eq!(pp.point_at_offset(Length::Absolute(-5.)).unwrap(), (0., 0.));

    // ratios
    assert_eq!(pp.point_at_offset(Length::Ratio(0.)).unwrap(), (0., 0.));
    assert_eq!(pp.point_at_offset(Length::Ratio(0.5)).unwrap(), (10., 5.));
    assert_eq!(pp.point_at_offset(Length::Ratio(1.)).unwrap(), (20., 10.));

    // rationals
    assert_eq!(
        pp.point_at_offset(Length::Rational(0, NonZeroU32::new(1).unwrap()))
            .unwrap(),
        (0., 0.)
    );
    assert_eq!(
        pp.point_at_offset(Length::Rational(1, NonZeroU32::new(3).unwrap()))
            .unwrap(),
        (10., 0.)
    );
    assert_eq!(
        pp.point_at_offset(Length::Rational(1, NonZeroU32::new(2).unwrap()))
            .unwrap(),
        (10., 5.)
    );
    assert_eq!(
        pp.point_at_offset(Length::Rational(2, NonZeroU32::new(3).unwrap()))
            .unwrap(),
        (10., 10.)
    );
    assert_eq!(
        pp.point_at_offset(Length::Rational(1, NonZeroU32::new(1).unwrap()))
            .unwrap(),
        (20., 10.)
    );

    // test 'z'
    assert_eq!(
        PathParser::new("m0 0h10v10h-10z")
            .point_at_offset(Length::Absolute(35.))
            .unwrap(),
        (0., 5.)
    );
}

#[test]
fn test_point_at_offset_curve() {
    // test quadratic bezier
    assert_eq!(
        PathParser::new("M 0 0 Q 20 40 40 0")
            .point_at_offset(Length::Ratio(0.5))
            .unwrap(),
        (20., 20.)
    );

    // test cubic bezier
    assert_eq!(
        PathParser::new("M 0 0 C 0 40 40 40 40 0")
            .point_at_offset(Length::Ratio(0.5))
            .unwrap(),
        (20., 30.)
    );

    // test arc (radius 40; center 50,50; start at 10,50 travel ccw)
    // Note a single arc command cannot represent a full circle.
    let mut pp = PathParser::new("M 10 50 A 40 40 0 1 0 90 50 A 40 40 0 1 0 10 50");
    for (offset, expected) in [
        (Length::Rational(0, NonZeroU32::new(4).unwrap()), (10., 50.)),
        (Length::Rational(1, NonZeroU32::new(4).unwrap()), (50., 90.)),
        (Length::Rational(2, NonZeroU32::new(4).unwrap()), (90., 50.)),
        (Length::Rational(3, NonZeroU32::new(4).unwrap()), (50., 10.)),
        (Length::Rational(4, NonZeroU32::new(4).unwrap()), (10., 50.)),
    ] {
        let point = pp.point_at_offset(offset).unwrap();
        assert!(
            (point.0 - expected.0).abs() < 1e-4 && (point.1 - expected.1).abs() < 1e-4,
            "Failed for offset {offset:?}: got {point:?}, expected {expected:?}"
        );
    }
}

#[test]
fn test_point_at_offset_smooth_curve() {
    fn assert_point_close(actual: (f32, f32), expected: (f32, f32)) {
        assert!(
            (actual.0 - expected.0).abs() < 1e-4 && (actual.1 - expected.1).abs() < 1e-4,
            "got {actual:?}, expected {expected:?}"
        );
    }

    fn point_at_command_ratio(path: &str, command_index: usize, ratio: f32) -> (f32, f32) {
        let mut measure = PathParser::new(path);
        measure.skip_whitespace();

        for _ in 0..command_index {
            measure.process_instruction().unwrap();
        }

        let old_length = measure.length_so_far();
        measure.process_instruction().unwrap();
        let contribution = measure.length_so_far() - old_length;

        PathParser::new(path)
            .point_at_offset(Length::Absolute(old_length + contribution * ratio))
            .unwrap()
    }

    assert_point_close(
        point_at_command_ratio("M 0 0 C 10 0 20 20 30 20 s 20 0 30 0", 2, 0.5),
        (45., 20.),
    );

    assert_point_close(
        point_at_command_ratio("M 0 0 C 10 0 20 20 30 20 S 50 20 60 20 80 20 90 20", 3, 0.5),
        (75., 20.),
    );

    assert_point_close(
        point_at_command_ratio("M 0 0 C 0 40 40 40 40 0 L 50 0 S 90 0 90 0", 3, 0.5),
        (70., 0.),
    );

    assert_point_close(
        point_at_command_ratio("M 0 0 Q 10 20 20 0 t 20 0", 2, 0.5),
        (30., -10.),
    );

    assert_point_close(
        point_at_command_ratio("M 0 0 Q 10 20 20 0 T 40 0 60 0", 3, 0.5),
        (50., 10.),
    );

    assert_point_close(
        point_at_command_ratio("M 0 0 Q 10 20 20 0 L 30 0 T 50 0", 3, 0.5),
        (35., 0.),
    );
}

#[test]
fn test_point_at_offset_arc_with_scaled_radii() {
    let mut pp = PathParser::new("M 0 0 A 13 10 0 1 0 27 1");

    assert_eq!(pp.point_at_offset(Length::Ratio(0.)).unwrap(), (0., 0.));

    let point = pp.point_at_offset(Length::Ratio(1.)).unwrap();
    assert!(
        (point.0 - 27.).abs() < 1e-4 && (point.1 - 1.).abs() < 1e-4,
        "Expected endpoint on scaled arc, got {point:?}"
    );
}
