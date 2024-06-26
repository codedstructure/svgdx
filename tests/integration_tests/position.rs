use svgdx::transform_str_default;

#[test]
fn test_position_trivial() {
    let input = r#"<rect x="1" y="2" width="3" height="4"/>"#;
    let expected = r#"<rect x="1" y="2" width="3" height="4"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    // Check x1/y1 are treated as alternatives to x/y
    let input = r#"<rect x1="1" y1="2" width="3" height="4"/>"#;
    let expected = r#"<rect width="3" height="4" x="1" y="2"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);
}

#[test]
fn test_position_x_spec() {
    // Specify cx and width
    let input = r#"<rect cx="1" y="2" width="3" height="4"/>"#;
    let expected = r#"<rect y="2" width="3" height="4" x="-0.5"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    // Specify x2 and width
    let input = r#"<rect x2="1" y="2" width="3" height="4"/>"#;
    let expected = r#"<rect y="2" width="3" height="4" x="-2"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    // Specify x1 and x2
    let input = r#"<rect x1="1" y="2" x2="3" height="4"/>"#;
    let expected = r#"<rect y="2" height="4" x="1" width="2"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    // Specify x2 and cx
    let input = r#"<rect x2="3" cx="1" y="2" height="4"/>"#;
    let expected = r#"<rect y="2" height="4" x="-1" width="4"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    // Specify x1 and cx
    let input = r#"<rect x1="1" cx="4" y="2" height="4"/>"#;
    let expected = r#"<rect y="2" height="4" x="1" width="6"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);
}

#[test]
fn test_position_y_spec() {
    // Specify cy and height
    let input = r#"<rect cy="1" x="2" height="3" width="4"/>"#;
    let expected = r#"<rect x="2" height="3" width="4" y="-0.5"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    // Specify y2 and height
    let input = r#"<rect y2="1" x="2" height="3" width="4"/>"#;
    let expected = r#"<rect x="2" height="3" width="4" y="-2"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    // Specify y1 and y2
    let input = r#"<rect y1="1" x="2" y2="3" width="4"/>"#;
    let expected = r#"<rect x="2" width="4" y="1" height="2"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    // Specify y2 and cy
    let input = r#"<rect y2="3" cy="1" x="2" width="4"/>"#;
    let expected = r#"<rect x="2" width="4" y="-1" height="4"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    // Specify y1 and cy
    let input = r#"<rect y1="1" cy="4" x="2" width="4"/>"#;
    let expected = r#"<rect x="2" width="4" y="1" height="6"/>"#;
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
    let expected = r#"<circle cy="3" cx="4" r="3"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);
}

// TODO: xy-loc handling

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
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r##"
<rect id="a" x="1" y="2" width="3" height="4"/>
<rect x="1" y="#a" width="3" height="4"/>
"##;
    let expected = r#"
<rect id="a" x="1" y="2" width="3" height="4"/>
<rect x="1" y="2" width="3" height="4"/>
"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r##"
<rect id="a" x="1" y="2" width="3" height="4"/>
<rect x="#a" y="#a" width="#a" height="#a"/>
"##;
    let expected = r#"
<rect id="a" x="1" y="2" width="3" height="4"/>
<rect x="1" y="2" width="3" height="4"/>
"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);
}
