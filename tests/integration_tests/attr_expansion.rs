use svgdx::transform_str_default;

#[test]
fn test_expand_rect_xy_wh() {
    let input = r#"<rect xy="1 2" wh="3 4"/>"#;
    let expected = r#"<rect x="1" y="2" width="3" height="4"/>"#;

    assert_eq!(transform_str_default(input).unwrap(), expected);
}

#[test]
fn test_expand_xy1_xy2() {
    let input = r#"<line xy1="1 2" xy2="3 4"/>"#;
    let expected = r#"<line x1="1" y1="2" x2="3" y2="4"/>"#;

    assert_eq!(transform_str_default(input).unwrap(), expected);
}

#[test]
fn test_expand_rect_cxy_wh() {
    let input = r#"<rect cxy="5 7" wh="3 4"/>"#;
    let expected = r#"<rect x="3.5" y="5" width="3" height="4"/>"#;

    assert_eq!(transform_str_default(input).unwrap(), expected);
}

#[test]
fn test_expand_rect_xy_loc() {
    let input = r#"<rect xy="5 7" wh="3 4" xy-loc="br"/>"#;
    let expected = r#"<rect x="2" y="3" width="3" height="4"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r#"<rect xy="5 7" wh="3 4" xy-loc="t"/>"#;
    let expected = r#"<rect x="3.5" y="7" width="3" height="4"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r#"<rect xy="5 7" wh="4 6" xy-loc="c"/>"#;
    let expected = r#"<rect x="3" y="4" width="4" height="6"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);
}

#[test]
fn test_expand_cycle() {
    let input = r#"<rect xy="5.5" wh="2"/>"#;
    let expected = r#"<rect x="5.5" y="5.5" width="2" height="2"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);
}

#[test]
fn test_expand_circle() {
    let input = r#"<circle cxy="5" wh="3"/>"#;
    let expected = r#"<circle cx="5" cy="5" r="1.5"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r#"<circle xy="5" wh="3"/>"#;
    let expected = r#"<circle cx="6.5" cy="6.5" r="1.5"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);
}

#[test]
fn test_expand_ellipse() {
    let input = r#"<ellipse cxy="5" wh="3 2"/>"#;
    let expected = r#"<ellipse cx="5" cy="5" rx="1.5" ry="1"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r#"<ellipse xy="5" wh="3 2"/>"#;
    let expected = r#"<ellipse cx="6.5" cy="6" rx="1.5" ry="1"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);
}

#[test]
// Check that attributes derived from compound attrs are overridden
// by explicit attributes
fn test_attr_priority() {
    let input = r#"<rect y="2" xy="3 4"/>"#;
    let expected = r#"<rect y="2" x="3"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r#"<rect xy="3 4" x="1"/>"#;
    let expected = r#"<rect x="1" y="4"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);
}

#[test]
fn test_attr_priority_delta() {
    let input = r#"<rect xy="3 4" x="10" dx="2"/>"#;
    let expected = r#"<rect x="12" y="4"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r#"<rect wh="7 8" width="12" dw="-1"/>"#;
    let expected = r#"<rect height="8" width="11"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    // Note only size attributes can have ratio deltas
    let input = r#"<rect wh="7 8" width="12" dw="25%"/>"#;
    let expected = r#"<rect height="8" width="3"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);
}
