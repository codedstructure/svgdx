use assertables::assert_contains;
use svgdx::transform_str_default;

#[test]
fn test_loop_trivial() {
    let input = r##"<loop count="0"><rect xy="2"/></loop>"##;
    let expected = r#""#;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, expected);

    let input = r##"<loop count="1"><rect xy="3"/></loop>"##;
    let expected = r#"<rect x="3" y="3"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, expected);

    let input = r##"<loop count="2"><rect xy="4"/></loop>"##;
    let expected = r#"<rect x="4" y="4"/><rect x="4" y="4"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, expected);
}

#[test]
fn test_loop_simple() {
    let input = r##"
<rect xy="0" wh="2"/>
<loop count="3"><rect xy="^|h" wh="2"/>
</loop>
"##;
    let expected = r#"
<rect x="0" y="0" width="2" height="2"/>
<rect x="2" y="0" width="2" height="2"/>
<rect x="4" y="0" width="2" height="2"/>
<rect x="6" y="0" width="2" height="2"/>
"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output.trim(), expected.trim());
}

#[test]
fn test_loop_variables() {
    let input = r##"
<var i="10"/>
<loop count="5"><circle cxy="$i" r="$i"/>
<var i="{{$i-2}}"/>
</loop>
"##;
    let expected = r#"
<circle cx="10" cy="10" r="10"/>
<circle cx="8" cy="8" r="8"/>
<circle cx="6" cy="6" r="6"/>
<circle cx="4" cy="4" r="4"/>
<circle cx="2" cy="2" r="2"/>
"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output.trim(), expected.trim());
}

#[test]
fn test_loop_while() {
    let input = r##"
<var i="10"/>
<loop while="{{gt($i,0)}}"><circle cxy="$i" r="$i"/>
<var i="{{$i-2}}"/>
</loop>
"##;
    let expected = r#"
<circle cx="10" cy="10" r="10"/>
<circle cx="8" cy="8" r="8"/>
<circle cx="6" cy="6" r="6"/>
<circle cx="4" cy="4" r="4"/>
<circle cx="2" cy="2" r="2"/>
"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output.trim(), expected.trim());

    let input = r##"<var i="0"/><loop while="gt($i, 0)"><rect xy="2"/></loop>"##;
    let expected = r#""#;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output.trim(), expected.trim());
}

#[test]
fn test_loop_until() {
    let input = r##"
<var i="10"/>
<loop until="{{lt($i,0)}}"><circle cxy="$i" r="$i"/>
<var i="{{$i-2}}"/>
</loop>
"##;
    let expected = r#"
<circle cx="10" cy="10" r="10"/>
<circle cx="8" cy="8" r="8"/>
<circle cx="6" cy="6" r="6"/>
<circle cx="4" cy="4" r="4"/>
<circle cx="2" cy="2" r="2"/>
<circle cx="0" cy="0" r="0"/>
"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output.trim(), expected.trim());
}

#[test]
fn test_loop_nested() {
    let input = r#"
<var i="3"/>
<loop while="{{gt($i, 0)}}">
<var j="3"/>
<loop while="{{gt($j, 0)}}"><rect wh="1" xy="{{$j, $i}}"/>
<var j="{{$j - 1}}"/>
</loop>
<var i="{{$i - 1}}"/>
</loop>
"#;
    let expected = r#"
<rect x="3" y="3" width="1" height="1"/>
<rect x="2" y="3" width="1" height="1"/>
<rect x="1" y="3" width="1" height="1"/>


<rect x="3" y="2" width="1" height="1"/>
<rect x="2" y="2" width="1" height="1"/>
<rect x="1" y="2" width="1" height="1"/>


<rect x="3" y="1" width="1" height="1"/>
<rect x="2" y="1" width="1" height="1"/>
<rect x="1" y="1" width="1" height="1"/>
"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output.trim(), expected.trim());
}

#[test]
fn test_loop_nested_deep() {
    let input = r#"
<var i="1"/>
<loop while="{{gt($i, 0)}}">
<var j="1" />
<loop while="{{gt($j, 0)}}">
<var k="3"/>
<loop while="{{gt($k, 0)}}">
<rect wh="1" xy="{{3 * $j + $k/3, 3 * $i + $k/3}}" class="d-thin"/>
<var k="{{$k - 1}}"/>
</loop>
<var j="{{$j - 1}}"/>
</loop>
<var i="{{$i - 1}}"/>
</loop>
"#;
    let expected = r#"
<rect x="4" y="4" width="1" height="1" class="d-thin"/>

<rect x="3.667" y="3.667" width="1" height="1" class="d-thin"/>

<rect x="3.333" y="3.333" width="1" height="1" class="d-thin"/>
"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_loop_count_loop_var() {
    let input = r#"
<loop count="2" var="i">
<loop count="3" var="j">
<rect wh="1" xy="{{3 * $j, 3 * $i}}"/>
</loop>
</loop>
"#;
    let expected = r#"
<rect x="0" y="0" width="1" height="1"/>

<rect x="3" y="0" width="1" height="1"/>

<rect x="6" y="0" width="1" height="1"/>



<rect x="0" y="3" width="1" height="1"/>

<rect x="3" y="3" width="1" height="1"/>

<rect x="6" y="3" width="1" height="1"/>
"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output.trim(), expected.trim());
}

#[test]
fn test_loop_count_loop_start_step() {
    let input = r#"
<loop count="3" var="i" start="-4.5" step="1.5">
<rect wh="1" xy="{{$i}} 10"/>
</loop>
"#;
    let expected = r#"
<rect x="-4.5" y="10" width="1" height="1"/>

<rect x="-3" y="10" width="1" height="1"/>

<rect x="-1.5" y="10" width="1" height="1"/>
"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output.trim(), expected.trim());
}

#[test]
fn test_loop_limit() {
    let input = r#"
<config loop-limit="100"/>
<loop count="1000"><rect wh="1" xy="0"/></loop>
"#;
    assert!(transform_str_default(input).is_err());

    let input = r#"
<config loop-limit="2000"/>
<loop count="1000"><rect wh="1" xy="0"/></loop>
"#;
    assert!(transform_str_default(input).is_ok());

    // test loop-limit boundary condition
    let input = r#"
<config loop-limit="100"/>
<loop count="100"><rect wh="1" xy="0"/></loop>
"#;
    assert!(transform_str_default(input).is_ok());

    let input = r#"
<config loop-limit="100"/>
<loop count="101"><rect wh="1" xy="0"/></loop>
"#;
    assert!(transform_str_default(input).is_err());
}

#[test]
fn test_for_loop() {
    let input = r#"
<for data="0, 1, 2" var="pos">
<rect wh="1" xy="$pos"/>
</for>
"#;
    let expected1 = r#"<rect x="0" y="0" width="1" height="1"/>"#;
    let expected2 = r#"<rect x="1" y="1" width="1" height="1"/>"#;
    let expected3 = r#"<rect x="2" y="2" width="1" height="1"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
    assert_contains!(output, expected3);

    let input = r#"
<for data="'a', 'b', 'c'" var="d" idx="idx">
<rect id="$d" wh="1" xy="$idx"/>
</for>
"#;
    let expected1 = r#"<rect id="a" x="0" y="0" width="1" height="1"/>"#;
    let expected2 = r#"<rect id="b" x="1" y="1" width="1" height="1"/>"#;
    let expected3 = r#"<rect id="c" x="2" y="2" width="1" height="1"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
    assert_contains!(output, expected3);
}

#[test]
fn test_for_loop_simple() {
    let input = r##"
<rect xy="0" wh="2"/>
<for data="0, 0, 0" var="_"><rect xy="^|h" wh="2"/>
</for>
"##;
    let expected = r#"
<rect x="0" y="0" width="2" height="2"/>
<rect x="2" y="0" width="2" height="2"/>
<rect x="4" y="0" width="2" height="2"/>
<rect x="6" y="0" width="2" height="2"/>
"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output.trim(), expected.trim());
}
