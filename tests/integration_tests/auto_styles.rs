use assertables::{assert_contains, assert_not_contains};
use svgdx::transform_str_default;

#[test]
fn test_style_stroke_colour() {
    let colour_input = r#"<svg><rect xy="0" wh="20" class="d-red" text="T"/></svg>"#;
    let output = transform_str_default(colour_input).unwrap();
    let expected_style = r#".d-red { stroke: red; }"#;
    assert_contains!(output, expected_style);
    let expected_style = r#"text.d-red, tspan.d-red, text.d-red * { fill: red; stroke: white; }"#;
    assert_contains!(output, expected_style);

    let colour_input = r#"<svg><rect xy="0" wh="20" class="d-black" text="T"/></svg>"#;
    let output = transform_str_default(colour_input).unwrap();
    let expected_style = r#".d-black { stroke: black; }"#;
    assert_contains!(output, expected_style);
    let expected_style =
        r#"text.d-black, tspan.d-black, text.d-black * { fill: black; stroke: white; }"#;
    assert_contains!(output, expected_style);

    // Check special case that d-none does not set text fill
    let colour_input = r#"<svg><rect xy="0" wh="20" class="d-none" text="T"/></svg>"#;
    let output = transform_str_default(colour_input).unwrap();
    let expected_style = r#".d-none { stroke: none; }"#;
    assert_contains!(output, expected_style);
    let unexpected_style = r#"text.d-none, tspan.d-none, text.d-none * { fill: none;"#;
    assert_not_contains!(output, unexpected_style);
}

#[test]
fn test_style_fill_colour() {
    let colour_input = r#"<svg><rect xy="0" wh="20" class="d-fill-red" text="test"/></svg>"#;
    let output = transform_str_default(colour_input).unwrap();
    let expected_style = r#".d-fill-red { fill: red; }"#;
    assert_contains!(output, expected_style);
    let expected_style =
        r#"text.d-fill-red, tspan.d-fill-red, text.d-fill-red * { fill: white; stroke: black; }"#;
    assert_contains!(output, expected_style);

    let colour_input = r#"<svg><rect xy="0" wh="20" class="d-fill-lightgrey" text="test"/></svg>"#;
    let output = transform_str_default(colour_input).unwrap();
    let expected_style = r#".d-fill-lightgrey { fill: lightgrey; }"#;
    assert_contains!(output, expected_style);
    let expected_style = r#"text.d-fill-lightgrey, tspan.d-fill-lightgrey, text.d-fill-lightgrey * { fill: black; stroke: white; }"#;
    assert_contains!(output, expected_style);
}

#[test]
fn test_style_text_colour() {
    let colour_input = r#"<svg><text xy="0" class="d-text-red">Hello!</text></svg>"#;
    let output = transform_str_default(colour_input).unwrap();
    let expected_style =
        r#"text.d-text-red, tspan.d-text-red, text.d-text-red * { fill: red; stroke: white; }"#;
    assert_contains!(output, expected_style);

    let colour_input = r#"<svg><text xy="0" class="d-text-black">Hello!</text></svg>"#;
    let output = transform_str_default(colour_input).unwrap();
    let expected_style = r#"text.d-text-black, tspan.d-text-black, text.d-text-black * { fill: black; stroke: white; }"#;
    assert_contains!(output, expected_style);
}

#[test]
fn test_style_text_attributes() {
    let input = r#"<svg><text xy="0" class="d-text-bold">Hello!</text></svg>"#;
    let output = transform_str_default(input).unwrap();
    let expected_style =
        r#"text.d-text-bold, tspan.d-text-bold, text.d-text-bold * { font-weight: bold; }"#;
    assert_contains!(output, expected_style);

    let input = r#"<svg><text xy="0" class="d-text-italic">Hello!</text></svg>"#;
    let output = transform_str_default(input).unwrap();
    let expected_style =
        r#"text.d-text-italic, tspan.d-text-italic, text.d-text-italic * { font-style: italic; }"#;
    assert_contains!(output, expected_style);

    let input = r#"<svg><text xy="0" class="d-text-monospace">Hello!</text></svg>"#;
    let output = transform_str_default(input).unwrap();
    let expected_style = r#"text.d-text-monospace, tspan.d-text-monospace, text.d-text-monospace * { font-family: monospace; }"#;
    assert_contains!(output, expected_style);

    let text_sizes = vec![
        ("d-text-smallest", 1.),
        ("d-text-smaller", 1.5),
        ("d-text-small", 2.),
        ("d-text-medium", 3.), // Default, but include explicitly for completeness
        ("d-text-large", 4.5),
        ("d-text-larger", 6.),
        ("d-text-largest", 9.),
    ];
    for (class, size) in text_sizes {
        let input = format!(r#"<svg><text xy="0" class="{class}">Hello!</text></svg>"#);
        let output = transform_str_default(&input).unwrap();
        let expected_style =
            format!("text.{class}, tspan.{class}, text.{class} * {{ font-size: {size}px; }}");
        assert_contains!(output, &expected_style);
    }
}

#[test]
fn test_style_text_outline() {
    let input = r#"<svg><text xy="0" class="d-text-ol">Hello!</text></svg>"#;
    let output = transform_str_default(input).unwrap();
    let expected_style =
        r#"text.d-text-ol, tspan.d-text-ol, text.d-text-ol * { stroke-width: 0.5; }"#;
    assert_contains!(output, expected_style);

    let input = r#"<svg><text xy="0" class="d-text-ol-red">Hello!</text></svg>"#;
    let output = transform_str_default(input).unwrap();
    let expected_style = r#"text.d-text-ol-red, tspan.d-text-ol-red, text.d-text-ol-red * { stroke: red; stroke-width: 0.5; }"#;
    assert_contains!(output, expected_style);

    let input = r#"<svg><text xy="0" class="d-text-ol-thicker">Hello!</text></svg>"#;
    let output = transform_str_default(input).unwrap();
    let expected_style = r#"text.d-text-ol-thicker, tspan.d-text-ol-thicker, text.d-text-ol-thicker * { stroke-width: 2; }"#;
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

#[test]
fn test_style_flow() {
    let input = r#"<svg><line xy1="0" xy2="0 10"/></svg>"#;
    let output = transform_str_default(input).unwrap();
    assert_not_contains!(output, "animation");

    let input = r#"<svg><line xy1="0" xy2="0 10" class="d-flow"/></svg>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, "animation");
    assert_not_contains!(output, "animation-direction: reverse");

    let input = r#"<svg><line xy1="0" xy2="0 10" class="d-flow d-flow-rev"/></svg>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, "animation-direction: reverse");
}

#[test]
fn test_style_grid() {
    let input = r#"<svg><rect wh="10" class="d-grid" /></svg>"#;
    let expected1 = r#".d-grid {"#;
    let expected2 = r#"url(#grid)"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);

    let input = r#"<svg><rect wh="10" class="d-grid-15" /></svg>"#;
    let expected1 = r#".d-grid-15 {"#;
    let expected2 = r#"url(#grid-15)"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);

    let input = r#"<svg><rect wh="10" class="d-grid-100" /></svg>"#;
    let expected1 = r#".d-grid-100 {"#;
    let expected2 = r#"url(#grid-100)"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);

    // Should only match values up to 100
    let input = r#"<svg><rect wh="10" class="d-grid-101" /></svg>"#;
    let expected1 = r#".d-grid-101 {"#;
    let expected2 = r#"url(#grid-101)"#;
    let output = transform_str_default(input).unwrap();
    assert_not_contains!(output, expected1);
    assert_not_contains!(output, expected2);
}

#[test]
fn test_style_vars() {
    let input = r#"
<var c="red"/>
<rect wh="10" style="fill: $c;stroke: green"/>
"#;
    let expected_style = r#"<rect width="10" height="10" style="fill: red; stroke: green;"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_style);
}
