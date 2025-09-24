use assertables::assert_contains;
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
  <config add-auto-styles="false"/>
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
    assert_contains!(output, expected);
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
