use assertables::assert_contains;
use svgdx::transform_str_default;

#[test]
fn test_rel_null_xfrm() {
    let rel_wh_input = r#"
<g transform="translate(0,0)">
<rect xy="5" wh="2"/>
<rect id="z" xy="^|h" wh="^"/>
</g>
"#;
    let expected_rect = r#"<rect id="z" x="7" y="5" width="2" height="2"/>"#;
    let output = transform_str_default(rel_wh_input).unwrap();
    assert_contains!(output, expected_rect);
}

#[test]
fn test_rel_xlat_internal() {
    let rel_wh_input = r#"
<g transform="translate(3,5)">
<rect xy="5" wh="2"/>
<rect id="z" xy="^|h" wh="^"/>
</g>
"#;
    let expected_rect = r#"<rect id="z" x="7" y="5" width="2" height="2"/>"#;
    let output = transform_str_default(rel_wh_input).unwrap();
    assert_contains!(output, expected_rect);
}

#[test]
fn test_rel_xlat_external() {
    let rel_wh_input = r##"
<g transform="translate(3,5)">
<rect xy="5" wh="2"/>
<rect id="z" xy="^|h" wh="^"/>
</g>
<rect id="a" surround="#z"/>
"##;
    let expected_rect = r#"<rect id="a" x="10" y="10" width="2" height="2""#;
    let output = transform_str_default(rel_wh_input).unwrap();
    assert_contains!(output, expected_rect);
}

#[test]
fn test_rel_xlat_nested() {
    let rel_wh_input = r##"
<svg>
<g transform="translate(2,3)">
<g transform="translate(5,7)">
<rect id="s1" xy="20" wh="2"/>
<rect id="s2" xy="^|h" wh="2"/>
</g>
<rect id="s3" xy="#s2|h" wh="2"/>
</g>
<rect id="a" surround="#s2"/>
<rect id="b" surround="#s3"/>
</svg>
"##;
    let expected1 = r#"<rect id="a" x="29" y="30""#;
    let expected2 = r#"<rect id="s3" x="29" y="27""#;
    let expected3 = r#"<rect id="b" x="31" y="30""#;
    let output = transform_str_default(rel_wh_input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
    assert_contains!(output, expected3);
}
