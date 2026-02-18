use svgdx::transform_str_default;

#[test]
fn test_if_simple() {
    let input = r#"<if test="1"><rect xy="10 20" wh="20 60"/></if>"#;
    let expected = r#"<rect x="10" y="20" width="20" height="60"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, expected);

    let input = r#"<if test="0"><rect xy="10 20" wh="20 60"/></if>"#;
    let expected = r#""#;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, expected);
}

#[test]
fn test_if_loop() {
    let input = r#"
<loop count="4" var="i">
<if test="eq($i % 2, 0)">
<rect x="{{$i * 10}}" wh="5"/>
</if></loop>
"#;
    let expected = r#"<rect x="0" width="5" height="5"/>



<rect x="20" width="5" height="5"/>
"#;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output.trim(), expected.trim());
}

#[test]
fn test_if_nested() {
    let input = r#"<if test="1"><if test="1"><rect xy="10 20" wh="20 60"/></if></if>"#;
    let expected = r#"<rect x="10" y="20" width="20" height="60"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, expected);

    let input = r#"<if test="1"><if test="0"><rect xy="10 20" wh="20 60"/></if></if>"#;
    let expected = r#""#;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, expected);

    let input = r#"<if test="0"><if test="1"><rect xy="10 20" wh="20 60"/></if></if>"#;
    let expected = r#""#;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, expected);
}
