use assertables::{assert_contains, assert_contains_as_result};
use svgdx::transform_str_default;

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
