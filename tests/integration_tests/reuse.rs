use assertables::{assert_contains, assert_contains_as_result};
use svgdx::transform_str_default;

#[test]
fn test_reuse_simple() {
    let input = r##"
<specs>
 <rect id="target" xy="0" wh="1 2"/>
</specs>
<reuse href="#target"/>
"##;
    let expected = r#"
<rect x="0" y="0" width="1" height="2" class="target"/>
"#;
    let output = transform_str_default(input).unwrap();
    // exact equality check: ensure that <specs> doesn't appear in the output.
    assert_eq!(output, expected);
}

#[test]
fn test_reuse_attr_locals() {
    let input = r##"
<specs>
 <rect id="square" width="$size" height="$size" xy="$x $y"/>
</specs>
<reuse href="#square" size="10" x="3" y="4"/>
"##;
    let expected = r#"<rect width="10" height="10" x="3" y="4" class="square"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r##"
<specs>
  <rect id="square" x="$x" y="$y" width="$size" height="$size"/>
</specs>
<reuse id="base" href="#square" x="0" y="0" size="10" class="thing"/>
"##;
    let expected = r#"<rect x="0" y="0" width="10" height="10" id="base" class="thing square"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}
