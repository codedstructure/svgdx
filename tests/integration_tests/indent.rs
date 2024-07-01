use svgdx::transform_str_default;

#[test]
fn test_indent_ws_only() {
    let input = r#"    "#;
    let expected = r#"    "#;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, expected);
}

#[test]
fn test_indent_none() {
    let input = r#"
<rect xy="10 20" wh="20 60" />
<rect id="z" xy="25 35" wh="^" />
"#;
    let expected = r#"
<rect x="10" y="20" width="20" height="60"/>
<rect id="z" x="25" y="35" width="20" height="60"/>
"#;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, expected);
}

#[test]
fn test_indent_constant() {
    // OOPS: looks like any leading indent / text event is dropped...
    let input = r#"
  <rect xy="10 20" wh="20 60" />
  <rect id="z" xy="25 35" wh="^" />
"#;
    let expected = r#"
  <rect x="10" y="20" width="20" height="60"/>
  <rect id="z" x="25" y="35" width="20" height="60"/>
"#;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, expected);

    let input = r#"
        <rect xy="10 20" wh="20 60" />
        <rect id="z" xy="25 35" wh="^" />
"#;
    let expected = r#"
        <rect x="10" y="20" width="20" height="60"/>
        <rect id="z" x="25" y="35" width="20" height="60"/>
"#;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, expected);
}

#[test]
fn test_indent_out_of_order() {
    let input = r##"
  <line start="#a@l" end="#a@r" text="a"/>
  <rect id="a" xy="1" wh="1"/>
"##;

    let expected = r##"
  <line x1="1" y1="1.5" x2="2" y2="1.5"/>
  <text x="1.5" y="1.5" class="d-tbox">a</text>
  <rect id="a" x="1" y="1" width="1" height="1"/>
"##;

    let output = transform_str_default(input).unwrap();
    assert_eq!(output, expected);
}

#[test]
fn test_indent_ooo_varying() {
    let input = r##"
     <line start="#a@l" end="#a@r" text="a"/>
   <rect id="a" xy="1" wh="1"/>
"##;

    let expected = r##"
     <line x1="1" y1="1.5" x2="2" y2="1.5"/>
     <text x="1.5" y="1.5" class="d-tbox">a</text>
   <rect id="a" x="1" y="1" width="1" height="1"/>
"##;

    let output = transform_str_default(input).unwrap();
    assert_eq!(output, expected);
}
