use assertables::{assert_contains, assert_contains_as_result};
use svgdx::transform_str_default;

#[test]
fn test_var_simple() {
    let input = r##"
<var x="1" y="2"/>
<var z="3"/>
<rect wh="10" text="$x-$y-$z"/>
"##;
    let expected = r#">1-2-3</text>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_var_update() {
    let input = r##"
<var x="1" y="2"/>
<var z="3" x="4"/>
<rect wh="10" text="$x-$y-$z"/>
"##;
    let expected = r#">4-2-3</text>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_var_missing() {
    let input = r##"
<var x="1" y="2"/>
<rect wh="10" text="$x-$y-$z"/>
"##;
    // missing var reference `$z` remains as-is
    let expected = r#">1-2-$z</text>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

/// Test that attribute-lookup variables are scoped to child elements
#[test]
fn test_var_nested() {
    let input = r##"
<var y="4"/>
<rect wh="10" text="r1-$x-$y-$z-$a-$b-$c"/>
<g x="7" y="8" z="9">
<g a="4" b="5" c="6">
<rect wh="10" text="r2-$x-$y-$z-$a-$b-$c"/>
</g>
<rect wh="10" text="r3-$x-$y-$z-$a-$b-$c"/>
</g>
<rect wh="10" text="r4-$x-$y-$z-$a-$b-$c"/>
"##;
    // Only existing `<var>` values and reuse attrs are used
    let expected1 = r#">r1-$x-4-$z-$a-$b-$c</text>"#;
    let expected2 = r#">r2-7-8-9-4-5-6</text>"#;
    let expected3 = r#">r3-7-8-9-$a-$b-$c</text>"#;
    let expected4 = r#">r4-$x-4-$z-$a-$b-$c</text>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
    assert_contains!(output, expected3);
    assert_contains!(output, expected4);
}

#[test]
fn test_var_reuse() {
    let input = r##"
<var x="4"/>
<g id="group1" y="8" z="9">
<rect wh="10" text="$x-$y-$z"/>
</g>
<reuse href="#group1" x="3"/>
"##;
    let expected1 = r#">4-8-9</text>"#; // original group
    let expected2 = r#">3-8-9</text>"#; // reuse group
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
}

#[test]
fn test_var_swap() {
    let input = r##"
<var x="1" y="2"/>
<var y="$x" x="$y"/>
<rect wh="10" text="$x-$y"/>
"##;
    let expected = r#">2-1</text>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_var_expr() {
    let input = r#"
<var r="0.7" g="0.3" b="0.5"/>
<rect wh="10" fill="rgb({{255 * $r, 255 * $g, 255 * $b}})"/>
"#;
    let expected = r#"
<rect width="10" height="10" fill="rgb(178.5, 76.5, 127.5)"/>
"#;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, expected);
}
