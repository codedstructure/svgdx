use svgdx::transform_str;

#[test]
fn test_style_stroke_colour() {
    let colour_input = r#"<svg><rect xy="0" wh="20" class="d-red" /></svg>"#;
    let expected_style = r#".d-red { stroke: red; }"#;
    assert!(transform_str(colour_input)
        .unwrap()
        .contains(expected_style));
    let expected_style = r#"text.d-red, text.d-red * { stroke: none; }"#;
    assert!(transform_str(colour_input)
        .unwrap()
        .contains(expected_style));

    let colour_input = r#"<svg><rect xy="0" wh="20" class="d-black" /></svg>"#;
    let expected_style = r#".d-black { stroke: black; }"#;
    assert!(transform_str(colour_input)
        .unwrap()
        .contains(expected_style));
    let expected_style = r#"text.d-black, text.d-black * { stroke: none; }"#;
    assert!(transform_str(colour_input)
        .unwrap()
        .contains(expected_style));
}

#[test]
fn test_style_fill_colour() {
    let colour_input = r#"<svg><rect xy="0" wh="20" class="d-fill-red" /></svg>"#;
    let expected_style = r#".d-fill-red { fill: red; }"#;
    assert!(transform_str(colour_input)
        .unwrap()
        .contains(expected_style));
    let expected_style = r#"text.d-fill-red, text.d-fill-red * { fill: white; }"#;
    assert!(transform_str(colour_input)
        .unwrap()
        .contains(expected_style));

    let colour_input = r#"<svg><rect xy="0" wh="20" class="d-fill-lightgrey" /></svg>"#;
    let expected_style = r#".d-fill-lightgrey { fill: lightgrey; }"#;
    assert!(transform_str(colour_input)
        .unwrap()
        .contains(expected_style));
    let expected_style = r#"text.d-fill-lightgrey, text.d-fill-lightgrey * { fill: black; }"#;
    assert!(transform_str(colour_input)
        .unwrap()
        .contains(expected_style));
}

#[test]
fn test_style_arrow() {
    let colour_input = r#"<svg><line xy1="0" xy2="10" class="d-arrow" /></svg>"#;
    let expected_style = r#".d-arrow { marker-end: url(#d-arrow); }"#;
    assert!(transform_str(colour_input)
        .unwrap()
        .contains(expected_style));
    let expected_defs = r#"<marker id="d-arrow" "#;
    assert!(transform_str(colour_input).unwrap().contains(expected_defs));
}
