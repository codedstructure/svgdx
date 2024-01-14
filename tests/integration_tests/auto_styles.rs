use assertables::{assert_contains, assert_contains_as_result};
use svgdx::transform_str_default;

#[test]
fn test_style_stroke_colour() {
    let colour_input = r#"<svg><rect xy="0" wh="20" class="d-red" /></svg>"#;
    let output = transform_str_default(colour_input).unwrap();
    let expected_style = r#".d-red { stroke: red; }"#;
    assert_contains!(output, expected_style);
    let expected_style = r#"text.d-red, text.d-red * { stroke: none; }"#;
    assert_contains!(output, expected_style);

    let colour_input = r#"<svg><rect xy="0" wh="20" class="d-black" /></svg>"#;
    let output = transform_str_default(colour_input).unwrap();
    let expected_style = r#".d-black { stroke: black; }"#;
    assert_contains!(output, expected_style);
    let expected_style = r#"text.d-black, text.d-black * { stroke: none; }"#;
    assert_contains!(output, expected_style);
}

#[test]
fn test_style_fill_colour() {
    let colour_input = r#"<svg><rect xy="0" wh="20" class="d-fill-red" /></svg>"#;
    let output = transform_str_default(colour_input).unwrap();
    let expected_style = r#".d-fill-red { fill: red; }"#;
    assert_contains!(output, expected_style);
    let expected_style = r#"text.d-fill-red, text.d-fill-red * { fill: white; }"#;
    assert_contains!(output, expected_style);

    let colour_input = r#"<svg><rect xy="0" wh="20" class="d-fill-lightgrey" /></svg>"#;
    let output = transform_str_default(colour_input).unwrap();
    let expected_style = r#".d-fill-lightgrey { fill: lightgrey; }"#;
    assert_contains!(output, expected_style);
    let expected_style = r#"text.d-fill-lightgrey, text.d-fill-lightgrey * { fill: black; }"#;
    assert_contains!(output, expected_style);
}

#[test]
fn test_style_arrow() {
    let input = r#"<svg><line xy1="0" xy2="10" class="d-arrow" /></svg>"#;
    let expected_style = r#".d-arrow { marker-end: url(#d-arrow); }"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_style);
    let expected_defs = r#"<marker id="d-arrow" "#;
    assert_contains!(output, expected_defs);

    let input = r#"<svg><line xy1="0" xy2="10" class="d-biarrow" /></svg>"#;
    let expected_style =
        r#".d-biarrow { marker-start: url(#d-arrow); marker-end: url(#d-arrow); }"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_style);
    let expected_defs = r#"<marker id="d-arrow" "#;
    assert_contains!(output, expected_defs);
}

#[test]
fn test_style_shadow() {
    let input = r#"<svg><rect wh="10" class="d-hardshadow" /></svg>"#;
    let expected_style = r#".d-hardshadow { filter: url(#d-hardshadow); }"#;
    let expected_defs = r#"<filter id="d-hardshadow""#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_style);
    assert_contains!(output, expected_defs);
    let expected_defs = r#"<feGaussianBlur in="SourceAlpha" stdDeviation="0.2"/>"#;
    assert_contains!(output, expected_defs);

    let input = r#"<svg><rect wh="10" class="d-softshadow" /></svg>"#;
    let expected_style = r#".d-softshadow { filter: url(#d-softshadow); }"#;
    let expected_defs = r#"<filter id="d-softshadow""#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_style);
    assert_contains!(output, expected_defs);
    let expected_defs = r#"<feGaussianBlur in="SourceAlpha" stdDeviation="0.7"/>"#;
    assert_contains!(output, expected_defs);
}
