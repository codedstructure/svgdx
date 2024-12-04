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
    let expected = r##"<svg width="50cm" height="25cm" version="1.1" xmlns="http://www.w3.org/2000/svg" viewBox="10 10 50 25">"##;
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
    let expected = r##"<svg height="30in" version="1.1" xmlns="http://www.w3.org/2000/svg" width="60in" viewBox="10 10 50 25">"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    // Same but unitless
    let input = r##"
<svg height="30">
  <config border="0"/>
  <rect x="10" y="10" width="50" height="25"/>
</svg>
"##;
    let expected = r##"<svg height="30" version="1.1" xmlns="http://www.w3.org/2000/svg" width="60" viewBox="10 10 50 25">"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    // And as %age
    let input = r##"
<svg height="40%">
  <config border="0"/>
  <rect x="10" y="10" width="50" height="25"/>
</svg>
"##;
    let expected = r##"<svg height="40%" version="1.1" xmlns="http://www.w3.org/2000/svg" width="80%" viewBox="10 10 50 25">"##;
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
    let expected = r##"<svg width="30in" version="1.1" xmlns="http://www.w3.org/2000/svg" height="15in" viewBox="10 10 50 25">"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    // Same but unitless
    let input = r##"
<svg width="30">
  <config border="0"/>
  <rect x="10" y="10" width="50" height="25"/>
</svg>
"##;
    let expected = r##"<svg width="30" version="1.1" xmlns="http://www.w3.org/2000/svg" height="15" viewBox="10 10 50 25">"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    // And as %age
    let input = r##"
<svg width="40%">
  <config border="0"/>
  <rect x="10" y="10" width="50" height="25"/>
</svg>
"##;
    let expected = r##"<svg width="40%" version="1.1" xmlns="http://www.w3.org/2000/svg" height="20%" viewBox="10 10 50 25">"##;
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
