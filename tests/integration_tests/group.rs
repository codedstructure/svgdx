use assertables::assert_contains;
use svgdx::transform_str_default;

#[test]
fn test_group_simple() {
    let input = r##"
<g id="a">
 <rect xy="0" wh="1 2"/>
</g>
<g id="b">
 <rect xy="10 0" wh="1 2"/>
</g>
"##;
    let expected = r#"
<g id="a">
 <rect x="0" y="0" width="1" height="2"/>
</g>
<g id="b">
 <rect x="10" y="0" width="1" height="2"/>
</g>
"#;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, expected);
}

#[test]
fn test_group_empty() {
    // Not useful, but valid. Should be passed through and closed.
    // At one point this failed as it generated a bare (unclosed) <g> element.
    let input = r##"
<g id="a"/>
"##;
    let expected = r#"
<g id="a"/>
"#;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, expected);
}

#[test]
fn test_group_nested() {
    let input = r##"
<g id="a">
 <g id="b">
  <rect xy="0" wh="1 2"/>
 </g>
 <rect xy="10 0" wh="1 2"/>
</g>
"##;
    let expected = r#"
<g id="a">
 <g id="b">
  <rect x="0" y="0" width="1" height="2"/>
 </g>
 <rect x="10" y="0" width="1" height="2"/>
</g>
"#;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, expected);
}

#[test]
fn test_group_reuse() {
    let input = r##"
<specs>
<g id="a">
 <rect xy="0" wh="1 2"/>
</g>
</specs>
<reuse href="#a"/>
"##;
    let expected = r#"
<g class="a">
 <rect x="0" y="0" width="1" height="2"/>
</g>
"#;
    let output = transform_str_default(input).unwrap();
    // exact equality check: ensure that <specs> doesn't appear in the output.
    assert_eq!(output, expected);
}

#[test]
fn test_group_rel_pos() {
    let rel_h_input = r##"
<g id="a"><rect x="0" y="0" width="10" height="10" /></g>
<rect xy="#a:h 5" wh="10" id="z"/>
"##;
    let expected_rect = r#"<rect id="z" x="15" y="0" width="10" height="10"/>"#;
    let output = transform_str_default(rel_h_input).unwrap();
    assert_contains!(output, expected_rect);
}

#[test]
fn test_group_bbox() {
    let rel_h_input = r##"
<g id="a">
 <rect wh="10"/>
 <rect xy="^:h" wh="10"/>
 <rect xy="^:h" wh="10"/>
</g>
<rect xy="#a:h 5" wh="10" id="z"/>
"##;
    let expected_rect = r#"<rect id="z" x="35" y="0" width="10" height="10"/>"#;
    let output = transform_str_default(rel_h_input).unwrap();
    assert_contains!(output, expected_rect);
}

#[test]
fn test_group_nested_bbox() {
    let rel_h_input = r##"
<g id="a">
 <rect wh="10"/>
 <rect xy="^:h" wh="10"/>
 <g id="b">
  <rect x="20" wh="10"/>
  <rect xy="^:v" wh="10"/>
 </g>
</g>
<rect xy="#a:h 5" wh="10" id="z"/>
"##;
    let expected_rect = r#"<rect id="z" x="35" y="5" width="10" height="10"/>"#;
    let output = transform_str_default(rel_h_input).unwrap();
    assert_contains!(output, expected_rect);
}
