use assertables::assert_contains;
use svgdx::{TransformConfig, transform_str, transform_str_default};

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
        <rect x="1" y="1" width="9" height="5"/>


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

#[test]
fn test_roundtrip_style() {
    // tests the minimal-escaping used in text events;
    // neither quotes nor '>' should be escaped.
    // CSS styles are just one use of this.
    let inner = r##"<style>
    text { font-family: "Times New Roman", serif; font-size: 12px; }
    text tspan { fill: #0000ff; }
    .red { fill: #ff0000; }
  </style>"##;
    let input = format!(
        r##"
<svg>
  {inner}
</svg>
"##
    );
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, inner);

    // '>' combinator is not escaped.
    let inner = r##"<style>
    text { font-family: "Times New Roman", serif; font-size: 12px; }
    text > tspan { fill: #0000ff; }
    .red { fill: #ff0000; }
  </style>"##;
    let input = format!(
        r##"
<svg>
  {inner}
</svg>
"##
    );
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, inner);

    // CDATA is essential if using '<' in CSS
    let inner = r##"<style>
    <![CDATA[
    text { font-family: "Times New Roman", serif; font-size: 12px; }
    text > tspan { fill: #0000ff; }
    [data-x<10] { color: red; }
    .red { fill: #ff0000; }
    ]]>
  </style>"##;
    let input = format!(
        r##"
<svg>
  {inner}
</svg>
"##
    );
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, inner);
}

#[test]
fn test_roundtrip_default_vars() {
    let input = r##"
<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="100" height="50" viewBox="0 0 100 50">
  <g x="2" y="3">
    <rect x="10" y="$VALUE" width="20" height="30"/>
  </g>
</svg>
"##;
    // variable substitution should be ignored, but check the addition
    // of a default variable doesn't defeat roundtrip detection
    let config = TransformConfig {
        vars: vec![("VALUE".parse().unwrap(), "3".to_string())]
            .into_iter()
            .collect(),
        ..Default::default()
    };
    let output = transform_str(input, &config).unwrap();
    assert_eq!(output, input);
}
