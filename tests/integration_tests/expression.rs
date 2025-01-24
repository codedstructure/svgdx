use svgdx::transform_str_default;

#[test]
fn test_scalarspec() {
    let input = r#"
  <rect id="a" wh="20"/>
  <text xy="^" text="{{#a~w}}"/>
"#;
    let expected = r#"
  <rect id="a" width="20" height="20"/>
  <text x="10" y="10" class="d-text">20</text>
"#;

    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    // As above but use '^' for scalar elref
    let input = r#"
  <rect wh="20"/>
  <text xy="^" text="{{^~w}}"/>
"#;
    let expected = r#"
  <rect width="20" height="20"/>
  <text x="10" y="10" class="d-text">20</text>
"#;

    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    // As above but use an id containing '-'
    let input = r#"
  <rect id="a-1" wh="20"/>
  <text xy="^" text="{{#a-1~w}}"/>
"#;
    let expected = r#"
  <rect id="a-1" width="20" height="20"/>
  <text x="10" y="10" class="d-text">20</text>
"#;

    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}
