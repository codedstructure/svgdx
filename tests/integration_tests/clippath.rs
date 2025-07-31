use assertables::assert_contains;
use svgdx::transform_str_default;

#[test]
fn test_clippath_simple() {
    let input = r##"
<defs>
 <clipPath id="cp"><rect wh="10"/></clipPath>
</defs>
<rect id="r1" wh="100" clip-path="url(#cp)"/>
<rect id="r2" surround="#r1"/>
"##;
    let expected = r#"<rect id="r2" x="0" y="0" width="10" height="10" class="d-surround"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r##"
<defs>
 <clipPath id="cp"><rect xy="5" wh="10"/></clipPath>
</defs>
<rect id="r1" wh="100" clip-path="url(#cp)"/>
<rect id="r2" surround="#r1"/>
"##;
    let expected = r#"<rect id="r2" x="5" y="5" width="10" height="10" class="d-surround"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_clippath_partial() {
    // The clippath is not entirely within the clipped element
    let input = r##"
<defs>
 <clipPath id="cp"><rect xy="95" wh="10"/></clipPath>
</defs>
<rect id="r1" wh="100" clip-path="url(#cp)"/>
<rect id="r2" surround="#r1"/>
"##;
    let expected = r#"<rect id="r2" x="95" y="95" width="5" height="5" class="d-surround"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_clippath_composite() {
    let input = r##"
<defs>
 <clipPath id="cp">
  <rect wh="10 50"/>
  <rect xy="10" wh="50 10"/>
 </clipPath>
</defs>
<rect id="r1" wh="100" clip-path="url(#cp)"/>
<rect id="r2" surround="#r1"/>
"##;
    let expected = r#"<rect id="r2" x="0" y="0" width="60" height="50" class="d-surround"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_clippath_previous() {
    let input = r##"
<defs>
 <clipPath id="cp">
  <rect wh="50"/>
 </clipPath>
</defs>
<rect id="r1" wh="100" clip-path="url(#cp)"/>
<rect id="r2" surround="^"/>
"##;
    let expected = r#"<rect id="r2" x="0" y="0" width="50" height="50" class="d-surround"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}
