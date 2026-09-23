use assertables::assert_contains;
use svgdx::transform_str_default;

#[test]
fn test_auto_id_simple() {
    let input = r#"
<rect wh="20 10" text="thing" class="d-auto-id"/>
"#;
    let expected = r#"
<rect id="thing" width="20" height="10"/>
<text x="10" y="5" class="d-text">thing</text>
"#;

    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_auto_id_with_markdown() {
    let input = r#"
<rect wh="20 10" md="this *or* that" class="d-auto-id"/>
"#;
    let expected = r#"
<rect id="thisOrThat" width="20" height="10"/>
"#;

    assert_contains!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_auto_id_camel_case() {
    let input = r#"
<rect x="0" y="0" width="20" height="10" text="hello world" class="d-auto-id"/>
<rect x="0" y="15" width="20" height="10" text="Bonjour!" class="d-auto-id"/>
"#;
    let expected = r#"
<rect id="helloWorld" x="0" y="0" width="20" height="10"/>
<text x="10" y="5" class="d-text">hello world</text>
<rect id="bonjour" x="0" y="15" width="20" height="10"/>
<text x="10" y="20" class="d-text">Bonjour!</text>
"#;

    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_auto_id_class_is_removed_without_text() {
    let input = r#"
<rect wh="20 10" class="d-auto-id"/>
"#;
    let expected = r#"
<rect width="20" height="10"/>
"#;

    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_explicit_id_precedence() {
    let input = r#"
<rect id="fixed" wh="20 10" text="thing" class="d-auto-id"/>
"#;
    let expected = r#"
<rect id="fixed" width="20" height="10"/>
<text x="10" y="5" class="d-text">thing</text>
"#;

    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_auto_id_avoids_existing() {
    let input = r#"
<rect id="thing" width="20" height="10"/>
<rect x="0" y="15" width="20" height="10" text="thing" class="d-auto-id"/>
"#;
    let expected = r#"
<rect id="thing" width="20" height="10"/>
<rect x="0" y="15" width="20" height="10"/>
<text x="10" y="20" class="d-text">thing</text>
"#;

    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_auto_id_connector() {
    let input = r##"
<rect wh="10" text="abc" xy="0" class="d-auto-id"/>
<rect wh="10" text="def" xy="^|h 20" class="d-auto-id"/>
<line start="#abc" end="#def"/>
"##;
    let expected = r##"
<line x1="10" y1="5" x2="30" y2="5"/>
"##;
    assert_contains!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}
