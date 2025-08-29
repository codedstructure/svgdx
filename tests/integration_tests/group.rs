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
<rect xy="#a|h 5" wh="10" id="z"/>
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
 <rect xy="^|h" wh="10"/>
 <rect xy="^|h" wh="10"/>
</g>
<rect xy="#a|h 5" wh="10" id="z"/>
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
 <rect xy="^|h" wh="10"/>
 <g id="b">
  <rect x="20" wh="10"/>
  <rect xy="^|v" wh="10"/>
 </g>
</g>
<rect xy="#a|h 5" wh="10" id="z"/>
"##;
    let expected_rect = r#"<rect id="z" x="35" y="5" width="10" height="10"/>"#;
    let output = transform_str_default(rel_h_input).unwrap();
    assert_contains!(output, expected_rect);
}

#[test]
fn test_group_transform_bbox() {
    let input = r##"
<svg>
  <config border="0"/>
  <g>
   <rect wh="10"/>
  </g>
  <g transform="translate(15)">
    <rect wh="10"/>
  </g>
</svg>
"##;
    let expected = r#"viewBox="0 0 25 10""#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_group_transform_prev() {
    let input = r##"
<g>
 <rect wh="10"/>
 <rect xy="^|V" wh="10"/>
</g>
<text id="z1" xy="^|v">Hello</text>
<g transform="translate(15)">
  <rect wh="10"/>
  <rect xy="^|V" wh="10"/>
</g>
<text id="z2" xy="^|v">World</text>
"##;
    let expected1 = r#"id="z1" x="5" y="11""#;
    let expected2 = r#"id="z2" x="20" y="11""#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
}

#[test]
fn test_g_previous() {
    let input = r##"
<g>
  <rect xy="10" wh="3"/>
  <rect xy="^|h 2" wh="3"/>
</g>
<rect xy="^|v" wh="5"/>
"##;
    let expected = r##"
<g>
  <rect x="10" y="10" width="3" height="3"/>
  <rect x="15" y="10" width="3" height="3"/>
</g>
<rect x="11.5" y="13" width="5" height="5"/>
"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r##"
<g>
  <rect xy="10" wh="3"/>
  <rect xy="^|h 2" wh="3"/>
</g>
<rect cxy="^@b" wh="2"/>
"##;
    let expected = r##"
<g>
  <rect x="10" y="10" width="3" height="3"/>
  <rect x="15" y="10" width="3" height="3"/>
</g>
<rect x="13" y="12" width="2" height="2"/>
"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_prev_el_from_group() {
    let input = r##"
<rect wh="1"/>
<g>
  <rect xy="^|h 1" wh="1"/>
</g>
"##;
    let expected = r#"<rect x="2" y="0" width="1" height="1"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_next_el_group() {
    let input = r##"
<svg>
  <circle xy="+|v" r="1"/>
  <g>
    <rect wh="1"/>
    <rect xy="^|h 1" wh="1"/>
  </g>
</svg>
"##;
    let expected = r#"<circle cx="1.5" cy="2" r="1"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_group_position() {
    let input = r#"
<g xy="1"><rect wh="1"/></g>
"#;
    let expected1 = r#"<g transform="translate(1, 1)">"#;
    let expected2 = r#"<rect width="1" height="1"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);

    let input = r#"
<g xy="1 3"><rect xy="10" wh="1"/></g>
"#;
    let expected1 = r#"<g transform="translate(1, 3)">"#;
    let expected2 = r#"<rect x="10" y="10" width="1" height="1"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
}
