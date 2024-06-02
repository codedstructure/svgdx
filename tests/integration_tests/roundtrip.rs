use svgdx::transform_str_default;

#[test]
fn test_roundtrip_minimal() {
    // Tests Start + Text + End
    let input = r##"
<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="100" height="50" viewBox="0 0 100 50"></svg>
"##;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, input);

    let input = r##"
<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="100" height="50" viewBox="0 0 100 50">   </svg>
"##;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, input);

    let input = r##"
<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="100" height="50" viewBox="0 0 100 50">
</svg>
"##;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, input);

    let input = r##"
<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="100" height="50" viewBox="0 0 100 50">


</svg>
"##;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, input);
}

#[test]
fn test_roundtrip_shapes() {
    // Tests Start + Text + Empty + Text + End
    let input = r##"
<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="100" height="50" viewBox="0 0 100 50">
  <rect x="1" y="2" width="3" height="4"/>
</svg>
"##;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, input);

    let input = r##"
<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="100" height="50" viewBox="0 0 100 50">

        <rect x="1" y="2" width="3" height="4"/>
        <rect y="1" height="5" width="9" x="3"/>


    <circle cx="12" cy="34" r="56"/>

</svg>
"##;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, input);
}

#[test]
fn test_roundtrip_nested() {
    // Tests Start + Text + Start + Text + End + Text + End
    let input = r##"
<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="100" height="50" viewBox="0 0 100 50">
  <blob x="1" y="2">thing</blob>
</svg>
"##;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, input);

    // Tests Start + Text + Start + Text + End + Text + End
    let input = r##"
<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="100" height="50" viewBox="0 0 100 50">
  <blob x="1" y="2">
    thing
  </blob>
</svg>
"##;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, input);
}

#[test]
fn test_roundtrip_text() {
    // Tests Start + Text + End
    let input = r##"
<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="100" height="50" viewBox="0 0 100 50">
    <text x="1" y="2"><tspan>some</tspan><tspan dx="1em" dy="1em">thing</tspan></text>
    <text x="3" y="4">other thing</text>
</svg>
"##;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, input);
}

#[test]
fn test_roundtrip_cdata() {
    // Tests Start + Empty + Text + CData + Text + End
    let input = r##"
<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="100" height="50" viewBox="0 0 100 50">
    <rect width="2" height="4"/>
    <![CDATA[
        cdata stuff
    ]]>
</svg>
"##;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, input);
}

#[test]
fn test_roundtrip_svg_units() {
    // Tests units on width / height don't break things
    let input = r##"
<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="100cm" height="50cm">
</svg>
"##;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, input);
}

#[test]
fn test_roundtrip_svg_percent() {
    // Tests %age on width / height don't break things
    let input = r##"
<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="100cm" height="50cm">
  <rect x="10%" y="10%" width="50%" height="50%"/>
</svg>
"##;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, input);
}
