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
<box id="b0" xy="#r0|h 5" wh="10"/>
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

#[test]
fn test_box_prev_element() {
    // box should affect prev_element, so #z should be
    // centered horizontally between #r0 and #r1
    let input = r##"
<rect id="r0" wh="10"/>
<rect id="r1" xy="^|h 20" wh="10"/>
<box id="b0" surround="#r0 #r1"/>
<rect id="z" xy="^|v" wh="10"/>
"##;
    let expected = r#"<rect id="z" x="15" y="10" width="10" height="10"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_box_text() {
    let input = r##"
<rect id="a" wh="10"/>
<rect id="b" xy="^|v 5" wh="10"/>
<box surround="#a #b" text="hello"/>
"##;
    let expected = r#"<text x="5" y="12.5" class="d-text">hello</text>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}
