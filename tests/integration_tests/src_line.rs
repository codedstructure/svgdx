use svgdx::{TransformConfig, transform_str};

fn meta_config() -> TransformConfig {
    TransformConfig {
        add_metadata: true,
        ..Default::default()
    }
}

#[test]
fn test_src_line_simple() {
    let input = r#"
<rect x="10" y="20" width="30" height="40"/>
<rect x="50" y="60" width="70" height="80"/>
"#;
    // note leading newline; numbering is 1-based
    let expected = r#"<rect x="10" y="20" width="30" height="40" data-src-line="2"/>
<rect x="50" y="60" width="70" height="80" data-src-line="3"/>
"#;
    let result = transform_str(input, &meta_config()).unwrap();
    assert_eq!(result, expected);

    // And again with some blank lines
    let input = r#"

<rect x="10" y="20" width="30" height="40"/>


<rect x="50" y="60" width="70" height="80"/>
"#;
    let expected = r#"<rect x="10" y="20" width="30" height="40" data-src-line="3"/>

<rect x="50" y="60" width="70" height="80" data-src-line="6"/>
"#;
    let result = transform_str(input, &meta_config()).unwrap();
    assert_eq!(result, expected);
}

#[test]
fn test_src_line_multiline() {
    let input = r#"
<rect x="10" y="20" width="30" height="40">
    <set attributeName="width" to="50" begin="0s" dur="1s"/>
</rect>
<path d="
    M 10 10
    L 20 20
"/>
"#;
    let expected = r#"<rect x="10" y="20" width="30" height="40" data-src-line="2">
  <set attributeName="width" to="50" begin="0s" dur="1s" data-src-line="3"/>
</rect>
<path d="M 10 10 L 20 20" data-src-line="5"/>
"#;
    let result = transform_str(input, &meta_config()).unwrap();
    assert_eq!(result, expected);
}

#[test]
fn test_src_line_text() {
    // Generated text elements retain the original element's source line
    let input = r#"
<rect x="0" y="0" width="30" height="40" text="Hello world!"/>"#;
    let expected = r#"<rect x="0" y="0" width="30" height="40" data-src-line="2"/>
<text x="15" y="20" class="d-text" data-src-line="2">Hello world!</text>"#;
    let result = transform_str(input, &meta_config()).unwrap();
    assert_eq!(result, expected);
}

#[test]
fn test_reuse_src_line_propagates() {
    let input = r##"
<specs>
  <g id="outer">
    <rect wh="4 5"/>
    <reuse href="#inner"/>
  </g>
  <rect id="inner" wh="2 3"/>
</specs>
<reuse href="#outer"/>
"##;
    let result = transform_str(input, &meta_config()).unwrap();
    assert_eq!(result.matches("data-src-line=").count(), 2);
    assert_eq!(result.matches(r#"data-src-line="9""#).count(), 2);
}

#[test]
fn test_reuse_stdlib_src_line() {
    let input = r##"<svg><reuse href="#d:store" wh="20"/></svg>"##;
    let result = transform_str(input, &meta_config().with_stdlib()).unwrap();
    assert_eq!(result.matches("data-src-line=").count(), 2);
    assert_eq!(result.matches(r#"data-src-line="1""#).count(), 2);
}
