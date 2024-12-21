use assertables::{assert_contains, assert_not_contains};
use svgdx::transform_str_default;

#[test]
fn test_box_simple() {
    let input = r##"
<box id="b0" xy="0" wh="10"/>
<rect id="r0" surround="#b0"/>
"##;
    let expected = r#"<rect id="r0" x="0" y="0" width="10" height="10" class="d-surround"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
    assert_not_contains!(output, "<box");
}

#[test]
fn test_box_refspec() {
    let input = r##"
<rect id="r0" wh="10"/>
<box id="b0" xy="#r0:h 5" wh="10"/>
<rect id="r1" cxy="#b0@c" wh="2"/>
"##;
    let expected = r#"<rect id="r1" x="19" y="4" width="2" height="2"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_box_included_at_toplevel() {
    let input = r##"
<svg>
<config border="0"/>
<box id="b0" xy="0" wh="1024, 768"/>
<rect id="r0" cxy="#b0" wh="100"/>
</svg>
"##;
    let expected = r#"viewBox="0 0 1024 768"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}
