use crate::utils::contains;

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
    let rel_refid_input = r##"
<rect xy="10 20" wh="20 60" id="abc"/>
<rect xy="98 99" wh="123 321" />
<rect xy="22 23" wh="234 654" />
<rect xy="#abc@tr" wh="20 60" id="z"/>
"##;
    let expected_rect = r#"<rect x="30" y="20" width="20" height="60" id="z"/>"#;
    contains(rel_refid_input, expected_rect);
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

#[test]
fn test_rel_recursive() {
    // Ensure a relative position can be derived from a referenced
    // (not just previous) element which is also relatively positioned
    let rel_refid_input = r##"
<rect xy="10 20" wh="20 60" id="abc"/>
<rect xy="@tr 12 0" wh="10" id="x"/>
<rect xy="@tl 1 1" wh="2"/>
<rect xy="@tl 1 1" wh="2"/>
<rect xy="@tl 1 1" wh="2"/>
<rect xy="#x@tr 12 0" wh="7" id="y"/>
<rect xy="@tl 1 1" wh="2"/>
<rect xy="@tl 1 1" wh="2"/>
<rect xy="@tl 1 1" wh="2" id="z"/>
"##;
    let expected_rect = r#"<rect x="42" y="20" width="10" height="10" id="x"/>"#;
    contains(rel_refid_input, expected_rect);
    let expected_rect = r#"<rect x="64" y="20" width="7" height="7" id="y"/>"#;
    contains(rel_refid_input, expected_rect);
    let expected_rect = r#"<rect x="67" y="23" width="2" height="2" id="z"/>"#;
    contains(rel_refid_input, expected_rect);
}
