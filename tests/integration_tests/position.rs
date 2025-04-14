use assertables::assert_contains;
use svgdx::transform_str_default;

#[test]
fn test_position_trivial() {
    let input = r#"<rect x="1" y="2" width="3" height="4"/>"#;
    let expected = r#"<rect x="1" y="2" width="3" height="4"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    // Check x1/y1 are treated as alternatives to x/y
    let input = r#"<rect x1="1" y1="2" width="3" height="4"/>"#;
    let expected = r#"<rect x="1" y="2" width="3" height="4"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);
}

#[test]
fn test_position_x_spec() {
    // Specify cx and width
    let input = r#"<rect cx="1" y="2" width="3" height="4"/>"#;
    let expected = r#"<rect x="-0.5" y="2" width="3" height="4"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    // Specify x2 and width
    let input = r#"<rect x2="1" y="2" width="3" height="4"/>"#;
    let expected = r#"<rect x="-2" y="2" width="3" height="4"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    // Specify x1 and x2
    let input = r#"<rect x1="1" y="2" x2="3" height="4"/>"#;
    let expected = r#"<rect x="1" y="2" width="2" height="4"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    // Specify x2 and cx
    let input = r#"<rect x2="3" cx="1" y="2" height="4"/>"#;
    let expected = r#"<rect x="-1" y="2" width="4" height="4"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    // Specify x1 and cx
    let input = r#"<rect x1="1" cx="4" y="2" height="4"/>"#;
    let expected = r#"<rect x="1" y="2" width="6" height="4"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);
}

#[test]
fn test_position_y_spec() {
    // Specify cy and height
    let input = r#"<rect cy="1" x="2" height="3" width="4"/>"#;
    let expected = r#"<rect x="2" y="-0.5" width="4" height="3"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    // Specify y2 and height
    let input = r#"<rect y2="1" x="2" height="3" width="4"/>"#;
    let expected = r#"<rect x="2" y="-2" width="4" height="3"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    // Specify y1 and y2
    let input = r#"<rect y1="1" x="2" y2="3" width="4"/>"#;
    let expected = r#"<rect x="2" y="1" width="4" height="2"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    // Specify y2 and cy
    let input = r#"<rect y2="3" cy="1" x="2" width="4"/>"#;
    let expected = r#"<rect x="2" y="-1" width="4" height="4"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    // Specify y1 and cy
    let input = r#"<rect y1="1" cy="4" x="2" width="4"/>"#;
    let expected = r#"<rect x="2" y="1" width="4" height="6"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);
}

#[test]
fn test_position_circle() {
    // Specify y1/y2 and cx for circle
    let input = r#"<circle y1="1" y2="3" cx="4" />"#;
    let expected = r#"<circle cx="4" cy="2" r="1"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    // Specify x1/cx and cy for circle
    let input = r#"<circle x1="1" y2="6" cy="3" />"#;
    let expected = r#"<circle cx="4" cy="3" r="3"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);
}

#[test]
fn test_position_relspec() {
    let input = r##"
<rect id="a" x="1" y="2" width="3" height="4"/>
<rect x="#a" y="2" width="3" height="4"/>
"##;
    let expected = r#"
<rect id="a" x="1" y="2" width="3" height="4"/>
<rect x="1" y="2" width="3" height="4"/>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    let input = r##"
<rect id="a" x="1" y="2" width="3" height="4"/>
<rect x="1" y="#a" width="3" height="4"/>
"##;
    let expected = r#"
<rect id="a" x="1" y="2" width="3" height="4"/>
<rect x="1" y="2" width="3" height="4"/>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    let input = r##"
<rect id="a" x="1" y="2" width="3" height="4"/>
<rect x="#a" y="#a" width="#a" height="#a"/>
"##;
    let expected = r#"
<rect id="a" x="1" y="2" width="3" height="4"/>
<rect x="1" y="2" width="3" height="4"/>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_position_scalar_locspec() {
    let input = r##"
<rect id="a" x="1" y="2" width="3" height="4"/>
<rect x="#a@b 3" cy="#a@bl 1" width="3" height="4"/>
"##;
    let expected = r#"
<rect id="a" x="1" y="2" width="3" height="4"/>
<rect x="5.5" y="5" width="3" height="4"/>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_position_scalar_edge() {
    let input = r##"
<rect id="a" x="1" y="2" width="3" height="4"/>
<rect cx="#a@b:25%" y2="#a@r:75%" width="3" height="4"/>
"##;
    let expected = r#"
<rect id="a" x="1" y="2" width="3" height="4"/>
<rect x="0.25" y="1" width="3" height="4"/>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_position_inferred_line() {
    let input = r#"<line cx="10" y1="0" y2="4"/>"#;
    let expected = r#"<line x1="10" y1="0" x2="10" y2="4"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r#"<line cy="10" x1="0" x2="4"/>"#;
    let expected = r#"<line x1="0" y1="10" x2="4" y2="10"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r#"<line x2="3" cx="10" cy="10" y2="20"/>"#;
    let expected = r#"<line x1="17" y1="0" x2="3" y2="20"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r#"<line x2="3" cx="10" cy="10" y2="20" dxy="2"/>"#;
    let expected = r#"<line x1="19" y1="2" x2="5" y2="22"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);
}

#[test]
fn test_position_deferred() {
    let input = r##"
<rect x="#a" y="2" width="#a 50%" height="4"/>
<rect id="a" x="1" y="2" width="3" height="4"/>
"##;
    let expected = r#"
<rect x="1" y="2" width="1.5" height="4"/>
<rect id="a" x="1" y="2" width="3" height="4"/>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_position_deferred_inferred() {
    // This checks both deferred values and inference from y1/y2 => r
    let input = r##"
<circle cx="#a" y1="#a" y2="#a"/>
<rect id="a" x="1" y="2" width="3" height="4"/>
"##;
    let expected = r#"
<circle cx="2.5" cy="4" r="2"/>
<rect id="a" x="1" y="2" width="3" height="4"/>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    // check inferred cx/x2/y2
    let input = r##"
<circle cx="#a" x2="#a" y2="#a"/>
<rect id="a" x="1" y="2" width="3" height="4"/>
"##;
    let expected = r#"
<circle cx="2.5" cy="4.5" r="1.5"/>
<rect id="a" x="1" y="2" width="3" height="4"/>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_position_missing_attrs() {
    let input = r#"<rect wh="4"/>"#;
    let expected = r#"<rect width="4" height="4"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r#"<rect x="2" wh="4"/>"#;
    let expected = r#"<rect x="2" width="4" height="4"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r#"<rect y="2" wh="4"/>"#;
    let expected = r#"<rect y="2" width="4" height="4"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r#"<rect cx="1" wh="4"/>"#;
    let expected = r#"<rect x="-1" width="4" height="4"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r#"<rect cy="1" wh="4"/>"#;
    let expected = r#"<rect y="-1" width="4" height="4"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r#"<circle wh="4"/>"#;
    let expected = r#"<circle r="2"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r#"<circle cx="1" wh="4"/>"#;
    let expected = r#"<circle cx="1" r="2"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r#"<circle cy="1" wh="4"/>"#;
    let expected = r#"<circle cy="1" r="2"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);
}

#[test]
fn test_position_dxy() {
    let input = r#"<rect wh="4" dx="2"/>"#;
    let expected = r#"<rect x="2" width="4" height="4"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r#"<rect wh="4" dy="3"/>"#;
    let expected = r#"<rect y="3" width="4" height="4"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r#"<rect wh="4" dxy="-2 3"/>"#;
    let expected = r#"<rect x="-2" y="3" width="4" height="4"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r#"<circle cx="1" wh="4" dxy="5"/>"#;
    let expected = r#"<circle cx="6" cy="5" r="2"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r#"<circle cx="1" wh="4" dxy="2 5"/>"#;
    let expected = r#"<circle cx="3" cy="5" r="2"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r#"<circle cx="1" wh="4" dx="3"/>"#;
    let expected = r#"<circle cx="4" r="2"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r#"<line xy1="0" xy2="10" dxy="5"/>"#;
    let expected = r#"<line x1="5" y1="5" x2="15" y2="15"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r#"<line xy1="0" xy2="10" dxy="-2 5"/>"#;
    let expected = r#"<line x1="-2" y1="5" x2="8" y2="15"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);
}

#[test]
fn test_position_dxy_polyline() {
    let input = r#"<polyline points="1 1 2 1 2 2 3 2 3 1 4 1" dxy="-1 3"/>"#;
    let expected = r#"<polyline points="1 1 2 1 2 2 3 2 3 1 4 1" transform="translate(-1, 3)"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);
}

#[test]
fn test_use_bbox() {
    let input = r##"
<svg>
  <config border="0"/>
  <defs>
    <g id="a"><rect xy="0" wh="10"/></g>
  </defs>
  <use id="b" x="0" y="0" href="#a"/>
  <circle cxy="#b@br" r="1"/>
</svg>
"##;
    let output = transform_str_default(input).unwrap();
    let expected1 = r#"viewBox="0 0 11 11""#;
    let expected2 = r#"circle cx="10" cy="10" r="1""#;
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
}

#[test]
fn test_reuse_bbox() {
    let input = r##"
<svg>
  <config border="0"/>
  <defs>
    <g id="a"><rect xy="0" wh="10"/></g>
  </defs>
  <reuse id="b" href="#a"/>
  <circle xy="#b|h" r="2"/>
</svg>
"##;
    let output = transform_str_default(input).unwrap();
    let expected1 = r#"viewBox="0 0 14 10""#;
    let expected2 = r#"circle cx="12" cy="5" r="2""#;
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
}

#[test]
fn test_reuse_specs_bbox() {
    // Same again but <specs> rather than <def>
    let input = r##"
<svg>
  <config border="0"/>
  <specs>
    <g id="a"><rect xy="0" wh="10"/></g>
  </specs>
  <reuse id="b" href="#a"/>
  <circle xy="#b|h" r="2"/>
</svg>
"##;
    let output = transform_str_default(input).unwrap();
    let expected1 = r#"viewBox="0 0 14 10""#;
    let expected2 = r#"circle cx="12" cy="5" r="2""#;
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
}

#[test]
fn test_line_width_height() {
    // Fix x1, width
    let input1 = r#"<line x1="2" y1="3" width="10"/>"#;
    // y2 vs y1 shouldn't matter
    let input2 = r#"<line x1="2" y2="3" width="10"/>"#;
    let input3 = r#"<line xy1="2 3" width="10"/>"#;
    let expected = r#"<line x1="2" y1="3" x2="12" y2="3"/>"#;
    assert_eq!(transform_str_default(input1).unwrap(), expected);
    assert_eq!(transform_str_default(input2).unwrap(), expected);
    assert_eq!(transform_str_default(input3).unwrap(), expected);

    // Fix y1, height
    let input1 = r#"<line x1="2" y1="3" height="10"/>"#;
    // x2 vs x1 shouldn't matter
    let input2 = r#"<line x2="2" y1="3" height="10"/>"#;
    let input3 = r#"<line xy1="2 3" height="10"/>"#;
    let expected = r#"<line x1="2" y1="3" x2="2" y2="13"/>"#;
    assert_eq!(transform_str_default(input1).unwrap(), expected);
    assert_eq!(transform_str_default(input2).unwrap(), expected);
    assert_eq!(transform_str_default(input3).unwrap(), expected);

    // Fix x2, width
    let input1 = r#"<line x2="3" y2="5" width="10"/>"#;
    // y2 vs y1 shouldn't matter
    let input2 = r#"<line x2="3" y1="5" width="10"/>"#;
    let input3 = r#"<line xy2="3 5" width="10"/>"#;
    let expected = r#"<line x1="-7" y1="5" x2="3" y2="5"/>"#;
    assert_eq!(transform_str_default(input1).unwrap(), expected);
    assert_eq!(transform_str_default(input2).unwrap(), expected);
    assert_eq!(transform_str_default(input3).unwrap(), expected);

    // Fix y2, height
    let input1 = r#"<line x1="2" y2="3" height="10"/>"#;
    // x2 vs x1 shouldn't matter
    let input2 = r#"<line x2="2" y2="3" height="10"/>"#;
    let input3 = r#"<line xy2="2 3" height="10"/>"#;
    let expected = r#"<line x1="2" y1="-7" x2="2" y2="3"/>"#;
    assert_eq!(transform_str_default(input1).unwrap(), expected);
    assert_eq!(transform_str_default(input2).unwrap(), expected);
    assert_eq!(transform_str_default(input3).unwrap(), expected);
}

#[test]
fn test_non_rect_scalar() {
    let input = r##"<circle id="a" r="3"/><circle id="b" r="#a~r"/>"##;
    assert_contains!(
        transform_str_default(input).unwrap(),
        r#"<circle id="b" r="3"/>"#
    );

    // Convention: 'r' scalarspec is the larger of rx/ry
    let input =
        r##"<ellipse id="a" rxy="3 4"/><circle id="b" r="#a~r"/><circle id="c" r="#a~rx"/>"##;
    let expected1 = r#"<circle id="b" r="4"/>"#;
    let expected2 = r#"<circle id="c" r="3"/>"#;
    let result = transform_str_default(input).unwrap();
    assert_contains!(result, expected1);
    assert_contains!(result, expected2);
}
