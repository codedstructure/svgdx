use svgdx::transform_str;

#[test]
fn test_repeat_simple() {
    let rep_input = r#"
<rect xy="0" wh="2"/>
<rect xy="@tr" wh="2" repeat="3"/>
"#;
    let expected = r#"
<rect x="0" y="0" width="2" height="2"/>
<rect x="2" y="0" width="2" height="2"/>
<rect x="4" y="0" width="2" height="2"/>
<rect x="6" y="0" width="2" height="2"/>
"#;
    assert_eq!(transform_str(rep_input).unwrap().trim(), expected.trim());
}

#[test]
fn test_repeat_zero() {
    let rep_input = r#"
<rect xy="0" wh="2"/>
<rect xy="@tr" wh="2" repeat="0"/>
"#;
    let expected = r#"
<rect x="0" y="0" width="2" height="2"/>
"#;
    assert_eq!(transform_str(rep_input).unwrap().trim(), expected.trim());
}
