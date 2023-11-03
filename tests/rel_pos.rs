pub mod utils;
use utils::contains;

#[test]
fn test_rel_prev() {
    let rel_h_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="@tr" wh="20 60" id="z"/>
"#;
    let expected_rect = r#"<rect x="30" y="20" width="20" height="60" id="z"/>"#;
    contains(rel_h_input, expected_rect);

    let rel_v_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="@bl -1 1" wh="20 60" id="z"/>
"#;
    let expected_rect = r#"<rect x="9" y="81" width="20" height="60" id="z"/>"#;
    contains(rel_v_input, expected_rect);
}

#[test]
fn test_rel_refid() {
    let rel_h_input = r##"
<rect xy="10 20" wh="20 60" id="abc"/>
<rect xy="98 99" wh="123 321" />
<rect xy="22 23" wh="234 654" />
<rect xy="#abc@tr" wh="20 60" id="z"/>
"##;
    let expected_rect = r#"<rect x="30" y="20" width="20" height="60" id="z"/>"#;
    contains(rel_h_input, expected_rect);
}

#[test]
fn test_relh() {
    let rel_h_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="relh" wh="20 60" id="z"/>
"#;
    let expected_rect = r#"<rect x="30" y="20" width="20" height="60" id="z"/>"#;
    contains(rel_h_input, expected_rect);
}

#[test]
fn test_relv() {
    let rel_v_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="relv" wh="20 60" id="z"/>
"#;
    let expected_rect = r#"<rect x="10" y="80" width="20" height="60" id="z"/>"#;
    contains(rel_v_input, expected_rect);
}

#[test]
fn test_rel_dx_dy() {
    let rel_h_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="relh -1.23 4.56" wh="20 60" id="z"/>
"#;
    let expected_rect = r#"<rect x="28.77" y="24.56" width="20" height="60" id="z"/>"#;
    contains(rel_h_input, expected_rect);

    let rel_input = r##"
<rect xy="10 20" wh="20 60" id="abc"/>
<rect xy="98 99" wh="123 321" />
<rect xy="22 23" wh="234 654" />
<rect xy="#abc@tr 10 100" wh="20 60" id="z"/>
"##;
    let expected_rect = r#"<rect x="40" y="120" width="20" height="60" id="z"/>"#;
    contains(rel_input, expected_rect);
}
