use svgdx::transform_str_default;

/// Check comments on elements are preserved and evaluated
#[test]
fn test_comment() {
    let input = r#"<rect xy="0" wh="5" _="A comment"/>"#;
    let expected = r#"<!--A comment-->
<rect x="0" y="0" width="5" height="5"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r#"<rect xy="0" wh="5" _="5 + 2 = {{5+2}}"/>"#;
    let expected = r#"<!--5 + 2 = 7-->
<rect x="0" y="0" width="5" height="5"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);
}

/// Check raw comments are preserved and *not* evaluated
#[test]
fn test_raw_comment() {
    let input = r#"<rect xy="0" wh="5" __="A comment"/>"#;
    let expected = r#"<!--A comment-->
<rect x="0" y="0" width="5" height="5"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r#"<rect xy="0" wh="5" __="5 + 2 = {{5+2}}"/>"#;
    let expected = r#"<!--5 + 2 = {{5+2}}-->
<rect x="0" y="0" width="5" height="5"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);
}

/// Check comments in var elements don't get propagated
#[test]
fn test_var_comment() {
    let input = r#"<var size="5" _="Size of rect"/>
<rect xy="0" wh="$size" />"#;
    let expected = r#"<rect x="0" y="0" width="5" height="5"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);

    let input = r#"<var size="5" __="Size of rect"/>
<rect xy="0" wh="$size" />"#;
    let expected = r#"<rect x="0" y="0" width="5" height="5"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);
}
