use assertables::{assert_contains, assert_contains_as_result};
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
<loop count="3"><rect xy="^:h" wh="2"/>
</loop>
"##;
    let expected = r#"
<rect x="0" y="0" width="2" height="2"/>
<rect x="2" y="0" width="2" height="2"/>
<rect x="4" y="0" width="2" height="2"/>
<rect x="6" y="0" width="2" height="2"/>
"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
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
    assert_contains!(output, expected);
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
    assert_contains!(output, expected);

    let input = r##"<var i="0"/><loop while="gt($i, 0)"><rect xy="2"/></loop>"##;
    let expected = r#""#;
    let output = transform_str_default(input).unwrap();
    assert_eq!(output, expected);
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
    assert_contains!(output, expected);
}

#[test]
fn test_loop_nested() {
    let input = r#"
<var i="3"/>
<loop while="{{gt($i, 0)}}">
<var j="3"/>
<loop while="{{gt($j, 0)}}"><rect wh="1" xy="{{$j}} {{$i}}"/>
<var j="{{$j - 1}}"/>
</loop>
<var i="{{$i - 1}}"/>
</loop>
"#;
    let expected = r#"
<rect width="1" height="1" x="3" y="3"/>
<rect width="1" height="1" x="2" y="3"/>
<rect width="1" height="1" x="1" y="3"/>

<rect width="1" height="1" x="3" y="2"/>
<rect width="1" height="1" x="2" y="2"/>
<rect width="1" height="1" x="1" y="2"/>

<rect width="1" height="1" x="3" y="1"/>
<rect width="1" height="1" x="2" y="1"/>
<rect width="1" height="1" x="1" y="1"/>
"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
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
<rect wh="1" xy="{{3 * $j + $k/3}} {{3 * $i + $k/3}}" class="d-thin"/>
<var k="{{$k - 1}}"/>
</loop>
<var j="{{$j - 1}}"/>
</loop>
<var i="{{$i - 1}}"/>
</loop>
"#;
    let expected = r#"
<rect width="1" height="1" x="4" y="4" class="d-thin"/>

<rect width="1" height="1" x="3.667" y="3.667" class="d-thin"/>

<rect width="1" height="1" x="3.333" y="3.333" class="d-thin"/>
"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}
