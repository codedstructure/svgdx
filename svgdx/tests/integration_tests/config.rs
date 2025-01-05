use assertables::assert_contains;
use svgdx::transform_str_default;

#[test]
fn test_config_debug() {
    let input = r#"
<config debug="true"/>
<rect xy="0" wh="5"/>
"#;
    let expected = r#"<!-- rect xy=`0` wh=`5` -->
<rect x="0" y="0" width="5" height="5"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_config_border() {
    let input = r#"
<svg>
<config border="3"/>
<rect xy="0" wh="5"/>
</svg>
"#;
    let expected = r#"viewBox="-3 -3 11 11""#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r#"
<svg>
<config border="12"/>
<rect xy="2 3" wh="4 5"/>
</svg>
"#;
    let expected = r#"viewBox="-10 -9 28 29""#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_config_scale() {
    let input = r#"
<svg>
<config border="0" scale="1.5"/>
<rect xy="0" wh="5"/>
</svg>
"#;
    let expected = r#"width="7.5mm""#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_config_background() {
    let input = r#"
<svg>
<config background="papayawhip"/>
<rect xy="0" wh="5"/>
</svg>
"#;
    let expected = r#"svg { background: papayawhip; }"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_config_auto_style() {
    let input = r#"
<svg>
<config add-auto-styles="true"/>
<rect xy="0" wh="5"/>
</svg>
"#;
    let expected = r#"<style>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r#"
<svg>
<config add-auto-styles="false"/>
<rect xy="0" wh="5"/>
</svg>
"#;
    let expected = r#"
<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="15mm" height="15mm" viewBox="-5 -5 15 15">
<rect x="0" y="0" width="5" height="5"/>
</svg>
"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}
