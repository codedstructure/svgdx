use assertables::assert_contains;
use svgdx::transform_str_default;

#[test]
fn test_defaults_simple() {
    let input = r##"
<defaults><rect wh="5"/></defaults>
<rect />
"##;
    let expected = r#"<rect width="5" height="5"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_defaults_empty() {
    for input in vec![
        r##"<defaults wh="37"/><defaults xy="10"/><rect />"##,
        r##"<defaults wh="37" xy="10"/><rect />"##,
        r##"<defaults wh="37"><_ xy="10"/></defaults><rect />"##,
    ] {
        let expected = r#"<rect x="10" y="10" width="37" height="37"/>"#;
        let output = transform_str_default(input).unwrap();
        assert_contains!(output, expected, "input: {}", input);
    }
}

#[test]
fn test_defaults_accumulate() {
    // Attributes get replaced
    let input = r##"
<defaults>
<rect fill="blue"/>
<rect fill="red"/>
<rect fill="green"/>
</defaults>
<rect />
"##;
    let expected = r#"<rect fill="green"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    // Classes get appended
    let input = r##"
<defaults>
<rect class="a"/>
<rect class="b"/>
<rect class="c"/>
</defaults>
<rect />
"##;
    let expected = r#"<rect class="a b c"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_defaults_element_match() {
    let input = r##"
<defaults>
<rect rx="5"/>
<ellipse rx="7"/>
<_ ry="9"/>
</defaults>
<rect/>
<ellipse/>
"##;
    let expected1 = r#"<rect rx="5" ry="9"/>"#;
    let expected2 = r#"<ellipse rx="7" ry="9"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
}

#[test]
fn test_defaults_class_match() {
    let input = r##"
<defaults>
<rect match=".first" fill="blue"/>
<rect match=".second" fill="red"/>
<rect match=".second, .first" stroke="black"/>
</defaults>
<rect class="first"/>
<rect class="second"/>
"##;
    let expected1 = r#"<rect fill="blue" stroke="black" class="first"/>"#;
    let expected2 = r#"<rect fill="red" stroke="black" class="second"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
}

#[test]
fn test_defaults_class_append() {
    // Classes get appended to any existing classes
    let input = r##"
<defaults>
<rect class="d-blue"/>
</defaults>
<rect id="z1" class="d-fill-red"/>
<rect id="z2"/>
"##;
    let expected1 = r#"<rect id="z1" class="d-fill-red d-blue"/>"#;
    let expected2 = r#"<rect id="z2" class="d-blue"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
}

#[test]
fn test_defaults_scope() {
    let input = r##"
<rect id="r1"/>
<defaults>
<rect fill="blue" width="4"/>
</defaults>
<rect id="r2"/>
<g>
 <defaults>
 <rect fill="red" stroke="black"/>
 </defaults>
 <rect id="r3"/>
</g>
<rect id="r4"/>
"##;
    let expected1 = r#"<rect id="r1"/>"#;
    let expected2 = r#"<rect id="r2" width="4" fill="blue"/>"#;
    let expected3 = r#"<rect id="r3" width="4" fill="red" stroke="black"/>"#;
    let expected4 = r#"<rect id="r4" width="4" fill="blue"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
    assert_contains!(output, expected3);
    assert_contains!(output, expected4);
}

#[test]
fn test_defaults_instance_priority() {
    let input = r##"
<defaults>
<rect fill="blue"/>
</defaults>
<rect fill="red"/>
"##;
    let expected = r#"<rect fill="red"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r##"
<defaults>
<rect wh="20"/>
</defaults>
<rect width="5"/>
"##;
    let expected = r#"<rect width="5" height="20"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_defaults_final() {
    let input = r##"
<defaults>
<rect fill="blue"/>
<rect match="final" fill="red"/>
<rect fill="green"/>
</defaults>
<rect />
"##;
    let expected = r#"<rect fill="red"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r##"
<defaults>
<rect class="a"/>
<rect match="final" class="b"/>
<rect class="c"/>
</defaults>
<rect />
"##;
    let expected = r#"<rect class="a b"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_defaults_init() {
    let input = r##"
<defaults>
<rect fill="blue"/>
<rect match="init" fill="red"/>
<rect fill="green"/>
</defaults>
<rect />
"##;
    let expected = r#"<rect fill="green"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r##"
<defaults>
<rect class="a"/>
<rect match="init" class="b"/>
<rect class="c"/>
</defaults>
<rect />
"##;
    let expected = r#"<rect class="b c"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_defaults_init_scope() {
    let input = r##"
<defaults>
<rect fill="blue"/>
</defaults>
<g>
  <defaults>
  <rect stroke="black"/>
  </defaults>
  <rect id="r1"/>
</g>
<g>
  <defaults>
  <_ match="init"/>
  <rect stroke="black"/>
  </defaults>
  <rect id="r2"/>
</g>
<rect id="r3"/>
"##;
    let expected1 = r#"<rect id="r1" fill="blue" stroke="black"/>"#;
    let expected2 = r#"<rect id="r2" stroke="black"/>"#;
    let expected3 = r#"<rect id="r3" fill="blue"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
    assert_contains!(output, expected3);
}

#[test]
fn test_defaults_init_reset() {
    let input = r##"
<defaults>
<rect class="a"/>
<rect match="init final" class="b"/>
<rect class="c"/>
</defaults>
<rect />
"##;
    let expected = r#"<rect class="b"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_defaults_style() {
    let input = r##"
<defaults>
<rect style="fill: blue; stroke: red"/>
<_ style="stroke-width: 0.2"/>
</defaults>
<rect style="fill: green" />
"##;
    let expected = r#"<rect style="fill: green; stroke: red; stroke-width: 0.2;"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_defaults_text_style() {
    let input = r##"
<defaults>
<rect wh="20" text-style="fill: blue; stroke: red"/>
<_ text-style="stroke-width: 0.2"/>
</defaults>
<rect text-style="fill: green" text="hi"/>
"##;
    let expected = r#"<text x="10" y="10" style="fill: green; stroke: red; stroke-width: 0.2;" class="d-text">hi</text>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_defaults_transform() {
    let input = r##"
<defaults>
<rect wh="20" transform="rotate(90)"/>
<_ transform="scale(2)"/>
</defaults>
<rect transform="translate(10, 10)"/>
"##;
    let expected =
        r#"<rect width="20" height="20" transform="rotate(90) scale(2) translate(10, 10)"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}
