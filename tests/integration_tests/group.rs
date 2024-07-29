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
