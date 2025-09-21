use assertables::assert_contains;
use svgdx::transform_str_default;

#[test]
fn test_rotate_simple() {
    let input = r##"
<rect wh="10 5" rotate="90"/>
<rect id="z" surround="^"/>
"##;
    let expected1 = r#"<rect width="10" height="5" transform="rotate(90, 5, 2.5)"/>"#;
    let expected2 = r#"<rect id="z" x="2.5" y="-2.5" width="5" height="10" class="d-surround"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
}

#[test]
fn test_rotate_text() {
    // text-rotate only
    let input = r##"<rect xy="10" wh="10" text-rotate="45" text="hello"/>"##;
    let expected1 = r#"<rect x="10" y="10" width="10" height="10"/>"#;
    let expected2 =
        r#"<text x="15" y="15" transform="rotate(45, 15, 15)" class="d-text">hello</text>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);

    // text-rotate and rotate
    let input = r##"<rect xy="10" wh="10" rotate="60" text-rotate="45" text="hello"/>"##;
    let expected1 =
        r#"<rect x="10" y="10" width="10" height="10" transform="rotate(60, 15, 15)"/>"#;
    let expected2 =
        r#"<text x="15" y="15" transform="rotate(45, 15, 15)" class="d-text">hello</text>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
}

#[test]
fn test_rotate_group() {
    let input = r##"
<g rotate="90">
  <rect wh="10 5"/>
  <circle cxy="5 2.5" r="1"/>
</g>
"##;
    let expected1 = r#"<g transform="rotate(90, 5, 2.5)">"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
}

#[test]
fn test_rotate_use() {
    let input = r##"
<symbol id="a">
  <rect wh="10 5"/>
  <circle cxy="5 2.5" r="1"/>
</symbol>
<use href="#a" x="10" y="4" rotate="90"/>
"##;
    let expected1 = r##"<use href="#a" x="10" y="4" transform="rotate(90, 15, 6.5)"/>"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
}

#[test]
fn test_rotate_reuse() {
    let input = r##"
<symbol id="a">
  <rect wh="10 5"/>
  <circle cxy="5 2.5" r="1"/>
</symbol>
<reuse href="#a" x="10" y="4" rotate="90"/>
"##;
    let expected1 = r##"<g transform="rotate(90, 15, 6.5) translate(10, 4)" class="a">"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
}

#[test]
fn test_rotate_loc() {
    // standard locspec on an element
    let input = r##"
<rect xy="3 7" wh="10 5" rotate="270" rotate-loc="br"/>
"##;
    let expected1 = r#"<rect x="3" y="7" width="10" height="5" transform="rotate(270, 13, 12)"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);

    // locspec on a group
    let input = r##"
<g rotate="90" rotate-loc="t">
  <rect wh="10 5"/>
</g>
"##;
    let expected1 = r#"<g transform="rotate(90, 5, 0)">"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);

    // edgespec
    let input = r##"
<rect xy="3 7" wh="10 5" rotate="-10" rotate-loc="b:40%"/>
"##;
    let expected1 = r#"<rect x="3" y="7" width="10" height="5" transform="rotate(-10, 7, 12)"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
}
