use assertables::assert_contains;
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

/// Test the inner-most variable definition takes precedence
#[test]
fn test_var_priority() {
    let input = r##"
<g z="1">
 <g z="2">
  <g z="3">
   <text text="$z"/>
  </g>
 </g>
</g>
"##;
    let expected = r#">3</text>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

/// check that a variable can contain the name of another variable to be used
#[test]
fn test_var_indirect() {
    let input = r##"
<var v1="37" v2="42"/>
<var select="v1"/>
<text text="$$select"/>
"##;
    let expected = r#">37</text>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r##"
<var v1="37" v2="42"/>
<var select="v2"/>
<text text="$$select"/>
"##;
    let expected = r#">42</text>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_var_reuse() {
    let input = r##"
<var p="4"/>
<g id="group1" q="8" r="9">
<rect wh="10" text="$p-$q-$r"/>
</g>
<reuse href="#group1" p="3"/>
"##;
    let expected1 = r#">4-8-9</text>"#; // original group
    let expected2 = r#">3-8-9</text>"#; // reuse group
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
}

#[test]
fn test_var_reuse_recursive() {
    let input = r##"
<specs>
 <text id="txt" text="$k"/>
 <reuse id="a" href="#txt" k="3"/>
 <reuse id="b" href="#a" k="4"/>
</specs>
<reuse id="c" href="#b" k="5"/>
"##;
    let expected = r#">5</text>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
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
    assert_eq!(output.trim(), expected.trim());
}

#[test]
fn test_var_classes() {
    let input = r#"
<var cls_a="d-red d-text-bold" cls_b="d-fill-none"/>
<rect wh="10" class="$cls_a $cls_b"/>
"#;
    let expected = r#"
<rect width="10" height="10" class="d-red d-text-bold d-fill-none"/>
"#;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output.trim(), expected.trim());
}

#[test]
fn test_var_limit() {
    // 'Normal' case, using {{..}} for numeric evaluation
    let input = r#"
<var thing="1"/>
<loop count="10"><var thing="{{$thing + $thing}}"/></loop>
<rect wh="10" text="$thing"/>
"#;
    let expected = r#">1024</text>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    // Same thing without {{..}}, which will cause string concatenation
    // and consequent var-limit error
    let input = r#"
<var thing="1"/>
<loop count="10"><var thing="$thing + $thing"/></loop>
<rect wh="10" text="$thing"/>
"#;
    let output = transform_str_default(input);
    assert!(output.is_err());

    // Raising the var-limit should allow this to be transformed
    // with '1 + ' repeated 1024 times, 5000 should be plenty
    let input = r#"
<config var-limit="5000"/>
<var thing="1"/>
<loop count="10"><var thing="$thing + $thing"/></loop>
<rect wh="10" text="$thing"/>
"#;
    let output = transform_str_default(input);
    assert!(output.is_ok());

    // Test var-limit boundary condition
    let input = r#"
<config var-limit="10"/>
<var thing="0123456789"/>
"#;
    let output = transform_str_default(input);
    assert!(output.is_ok());

    let input = r#"
<config var-limit="10"/>
<var thing="01234567890"/>
"#;
    let output = transform_str_default(input);
    assert!(output.is_err());
}

#[test]
fn test_var_scopes() {
    // Check that variables are scoped to the element they're defined within
    let input = r#"
<var k="1"/>
<g k="2">
  <text text="1:$k"/>
  <g k="3">
    <text text="2:$k"/>
    <var k="4"/>
    <text text="3:$k"/>
  </g>
  <text text="4:$k"/>
</g>
<text text="5:$k"/>
"#;
    let expected1 = r#">1:2</text>"#;
    let expected2 = r#">2:3</text>"#;
    let expected3 = r#">3:4</text>"#;
    let expected4 = r#">4:2</text>"#;
    let expected5 = r#">5:1</text>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
    assert_contains!(output, expected3);
    assert_contains!(output, expected4);
    assert_contains!(output, expected5);
}

#[test]
fn test_var_closure() {
    // Check that deferred elements carry closure info
    let input = r##"
<svg>
<var k="word"/>
<rect xy="#later|h" wh="10" text="$k"/>
<rect id="later" xy="0" wh="10"/>
</svg>
"##;
    let expected = r#">word</text>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    // Check closures contain info which can be evaluated
    let input = r##"
<svg>
  <g>
    <var k="12"/>
    <rect xy="#later|h" wh="10" text="{{$k / 3}}"/>
  </g>
  <rect id="later" xy="0" wh="10"/>
</svg>
"##;
    let expected = r#">4</text>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}
