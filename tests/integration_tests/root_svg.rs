use crate::assert_contains;
use svgdx::transform_str_default;

#[test]
fn test_root_svg_no_wh() {
    let input = r##"
<svg>
  <config border="0"/>
  <rect x="10" y="10" width="50" height="25"/>
</svg>
"##;
    let expected = r##"<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="50mm" height="25mm" viewBox="10 10 50 25">"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_root_svg_wh() {
    // Tests provided width and height (including units) are preserved
    let input = r##"
<svg width="50cm" height="25cm">
  <config border="0"/>
  <rect x="10" y="10" width="50" height="25"/>
</svg>
"##;
    let expected = r##"<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="50cm" height="25cm" viewBox="10 10 50 25">"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_root_svg_no_width() {
    // Tests width is calculated from given height, including any units
    let input = r##"
<svg height="30in">
  <config border="0"/>
  <rect x="10" y="10" width="50" height="25"/>
</svg>
"##;
    let expected = r##"<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="60in" height="30in" viewBox="10 10 50 25">"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    // Same but unitless
    let input = r##"
<svg height="30">
  <config border="0"/>
  <rect x="10" y="10" width="50" height="25"/>
</svg>
"##;
    let expected = r##"<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="60" height="30" viewBox="10 10 50 25">"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    // And as %age
    let input = r##"
<svg height="40%">
  <config border="0"/>
  <rect x="10" y="10" width="50" height="25"/>
</svg>
"##;
    let expected = r##"<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="80%" height="40%" viewBox="10 10 50 25">"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_root_svg_no_height() {
    // Tests height is calculated from given width, including any units
    let input = r##"
<svg width="30in">
  <config border="0"/>
  <rect x="10" y="10" width="50" height="25"/>
</svg>
"##;
    let expected = r##"<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="30in" height="15in" viewBox="10 10 50 25">"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    // Same but unitless
    let input = r##"
<svg width="30">
  <config border="0"/>
  <rect x="10" y="10" width="50" height="25"/>
</svg>
"##;
    let expected = r##"<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="30" height="15" viewBox="10 10 50 25">"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    // And as %age
    let input = r##"
<svg width="40%">
  <config border="0"/>
  <rect x="10" y="10" width="50" height="25"/>
</svg>
"##;
    let expected = r##"<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="40%" height="20%" viewBox="10 10 50 25">"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_internal_svg() {
    // Tests that an SVG element inside another SVG element is not modified
    // if the xmlns attribute is present
    let input = r##"
<svg>
  <config auto-style-mode="none"/>
  <rect xy="10" wh="50"/>
  <svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="50cm" height="25cm">
    <rect x="0" y="0" width="50" height="25" text="blob"/>
  </svg>
</svg>
"##;
    let expected = r##"
<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="60mm" height="60mm" viewBox="5 5 60 60">
  <rect x="10" y="10" width="50" height="50"/>
  <svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="50cm" height="25cm">
    <rect x="0" y="0" width="50" height="25" text="blob"/>
  </svg>
</svg>
"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output.trim(), expected.trim());
}

#[test]
fn test_root_svg_bbox_in_order() {
    let input = r##"
<svg>
  <config border="0"/>
  <rect id="abc" xy="10" wh="20"/>
  <rect xy="#abc|h" wh="20"/>
</svg>
"##;
    let expected = r##"<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="40mm" height="20mm" viewBox="10 10 40 20">"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_root_svg_bbox_out_of_order() {
    let input = r##"
<svg>
  <config border="0"/>
  <rect xy="#abc|h" wh="20"/>
  <rect id="abc" xy="10" wh="20"/>
</svg>
"##;
    let expected = r##"<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="40mm" height="20mm" viewBox="10 10 40 20">"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_root_svg_bbox_isolated_nested_elements() {
    let input = r##"
<svg>
  <config border="0"/>
  <defs>
    <clipPath id="clip"><rect xy="10 5" wh="10"/></clipPath>
    <rect xy="-100" wh="1000"/>
  </defs>
  <specs><rect id="unused" xy="-200" wh="1000"/></specs>
  <g transform="translate(10 5)" clip-path="url(#clip)">
    <rect wh="100"/>
  </g>
  <rect xy="40 5" wh="10"/>
</svg>
"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, r#"viewBox="10 5 40 10""#);
}

#[test]
fn test_root_svg_bbox_ignores_failed_group() {
    let input = r##"
<svg>
  <config border="0" error-mode="warn"/>
  <g rotate="invalid"><rect xy="-100" wh="10"/></g>
  <rect xy="20" wh="10"/>
</svg>
"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, r#"viewBox="20 20 10 10""#);
    assert_contains!(output, "Warning: error processing element");
}

#[test]
fn test_root_svg_bbox_control_flow() {
    let input = r##"
<svg>
  <config border="0"/>
  <loop count="3" var="i">
    <if test="1"><rect xy="{{$i * 20}} 10" wh="10"/></if>
  </loop>
  <for var="x" data="100, 120"><rect xy="$x 10" wh="10"/></for>
</svg>
"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, r#"viewBox="0 10 130 10""#);
}

#[test]
fn test_root_svg_bbox_reuse_clipped_group() {
    let input = r##"
<svg>
  <config border="0"/>
  <defs><clipPath id="clip"><rect xy="10" wh="10"/></clipPath></defs>
  <specs><g id="shape"><rect wh="100"/></g></specs>
  <reuse href="#shape" clip-path="url(#clip)"/>
  <rect xy="30" wh="10"/>
</svg>
"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, r#"viewBox="10 10 30 30""#);
}

#[test]
fn test_root_svg_bbox_failed_loop_does_not_contribute() {
    let input = r##"
<svg>
  <config border="0" error-mode="warn" loop-limit="1"/>
  <loop count="3"><rect xy="-100" wh="10"/></loop>
  <rect xy="20" wh="10"/>
</svg>
"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, r#"viewBox="20 20 10 10""#);
    assert_contains!(output, "Warning: error processing element");
}

#[test]
fn test_root_svg_bbox_clip_scope_preserves_siblings() {
    let input = r##"
<svg>
  <config border="0"/>
  <defs><clipPath id="clip"><rect xy="50" wh="10"/></clipPath></defs>
  <rect wh="10"/>
  <g clip-path="url(#clip)"><rect xy="100" wh="10"/></g>
  <g clip-path="url(#clip)"><rect wh="100"/></g>
  <rect xy="80" wh="10"/>
</svg>
"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, r#"viewBox="0 0 90 90""#);
}

#[test]
fn test_root_svg_bbox_clipped_group_forward_reference() {
    let input = r##"
<svg>
  <config border="0"/>
  <g clip-path="url(#clip)"><rect xy="-100" wh="200"/></g>
  <defs><clipPath id="clip"><rect xy="10" wh="10"/></clipPath></defs>
  <rect xy="30" wh="10"/>
</svg>
"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, r#"viewBox="10 10 30 30""#);
}

#[test]
fn test_root_svg_preserve_attrs() {
    let input = r##"
<svg id="abc123" class="svgdx-a blob" data-attr="arbitrary" style="border: 1px solid red;">
  <config border="0"/>
  <rect x="10" y="10" width="50" height="25"/>
</svg>
"##;
    let expected = r##"<svg id="abc123" version="1.1" xmlns="http://www.w3.org/2000/svg" width="50mm" height="25mm" data-attr="arbitrary" viewBox="10 10 50 25" style="border: 1px solid red;" class="svgdx-a blob">"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_root_svg_style_combo() {
    let input = r##"
<svg style="border: 1px solid red;">
  <config border="0" svg-style="background: silver;"/>
  <rect x="10" y="10" width="50" height="25"/>
</svg>
"##;
    let expected = r##"<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="50mm" height="25mm" viewBox="10 10 50 25" style="border: 1px solid red; background: silver;">"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}
