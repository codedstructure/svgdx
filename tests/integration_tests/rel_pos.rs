use svgdx::transform_str_default;

#[test]
fn test_rel_prev() {
    let rel_h_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="@tr" wh="20 60" id="z"/>
"#;
    let expected_rect = r#"<rect x="30" y="20" width="20" height="60" id="z"/>"#;
    assert!(transform_str_default(rel_h_input)
        .unwrap()
        .contains(expected_rect));

    let rel_v_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="@bl -1 1" wh="20 60" id="z"/>
"#;
    let expected_rect = r#"<rect x="9" y="81" width="20" height="60" id="z"/>"#;
    assert!(transform_str_default(rel_v_input)
        .unwrap()
        .contains(expected_rect));
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
    assert!(transform_str_default(rel_refid_input)
        .unwrap()
        .contains(expected_rect));
}

#[test]
fn test_relh() {
    // TO THE RIGHT
    let rel_h_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="^h" wh="20 60" id="z"/>
"#;
    let expected_rect = r#"<rect x="30" y="20" width="20" height="60" id="z"/>"#;
    assert!(transform_str_default(rel_h_input)
        .unwrap()
        .contains(expected_rect));

    // With a gap
    let rel_h_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="^h 3" wh="20 60" id="z"/>
"#;
    let expected_rect = r#"<rect x="33" y="20" width="20" height="60" id="z"/>"#;
    assert!(transform_str_default(rel_h_input)
        .unwrap()
        .contains(expected_rect));

    // TO THE LEFT
    let rel_h_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="^H" wh="20 60" id="z"/>
"#;
    let expected_rect = r#"<rect x="-10" y="20" width="20" height="60" id="z"/>"#;
    assert!(transform_str_default(rel_h_input)
        .unwrap()
        .contains(expected_rect));

    // With a gap
    let rel_h_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="^H 3" wh="20 60" id="z"/>
"#;
    let expected_rect = r#"<rect x="-13" y="20" width="20" height="60" id="z"/>"#;
    assert!(transform_str_default(rel_h_input)
        .unwrap()
        .contains(expected_rect));
}

#[test]
fn test_relv() {
    // VERTICAL BELOW
    let rel_v_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="^v" wh="20 60" id="z"/>
"#;
    let expected_rect = r#"<rect x="10" y="80" width="20" height="60" id="z"/>"#;
    assert!(transform_str_default(rel_v_input)
        .unwrap()
        .contains(expected_rect));

    // With a gap
    let rel_v_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="^v 5" wh="20 60" id="z"/>
"#;
    let expected_rect = r#"<rect x="10" y="85" width="20" height="60" id="z"/>"#;
    assert!(transform_str_default(rel_v_input)
        .unwrap()
        .contains(expected_rect));

    // VERTICAL ABOVE
    let rel_v_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="^V" wh="20 60" id="z"/>
"#;
    let expected_rect = r#"<rect x="10" y="-40" width="20" height="60" id="z"/>"#;
    assert!(transform_str_default(rel_v_input)
        .unwrap()
        .contains(expected_rect));

    // With a gap
    let rel_v_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="^V 5" wh="20 60" id="z"/>
"#;
    let expected_rect = r#"<rect x="10" y="-45" width="20" height="60" id="z"/>"#;
    assert!(transform_str_default(rel_v_input)
        .unwrap()
        .contains(expected_rect));
}

#[test]
fn test_elref_relh() {
    // TO THE RIGHT
    let rel_h_input = r##"
<rect id="abc" xy="10 20" wh="10" />
<rect id="def" xy="30" wh="2" />
<rect id="z" xy="#abc:h" wh="10" />
"##;
    let expected_rect = r#"<rect id="z" x="20" y="20" width="10" height="10"/>"#;
    assert!(transform_str_default(rel_h_input)
        .unwrap()
        .contains(expected_rect));

    // with a gap
    let rel_h_input = r##"
<rect id="abc" xy="10 20" wh="10" />
<rect id="def" xy="30" wh="2" />
<rect id="z" xy="#abc:h 3" wh="10" />
"##;
    let expected_rect = r#"<rect id="z" x="23" y="20" width="10" height="10"/>"#;
    assert!(transform_str_default(rel_h_input)
        .unwrap()
        .contains(expected_rect));

    // TO THE LEFT
    let rel_h_input = r##"
<rect id="abc" xy="10 20" wh="10" />
<rect id="def" xy="30" wh="2" />
<rect id="z" xy="#abc:H" wh="8" />
"##;
    let expected_rect = r#"<rect id="z" x="2" y="21" width="8" height="8"/>"#;
    assert!(transform_str_default(rel_h_input)
        .unwrap()
        .contains(expected_rect));

    // with a gap
    let rel_h_input = r##"
<rect id="abc" xy="10 20" wh="10" />
<rect id="def" xy="30" wh="2" />
<rect id="z" xy="#abc:H 3" wh="8" />
"##;
    let expected_rect = r#"<rect id="z" x="-1" y="21" width="8" height="8"/>"#;
    assert!(transform_str_default(rel_h_input)
        .unwrap()
        .contains(expected_rect));
}

#[test]
fn test_elref_relv() {
    // BELOW
    let rel_v_input = r##"
<rect id="abc" xy="10 20" wh="10" />
<rect id="def" xy="30" wh="2" />
<rect id="z" xy="#abc:v" wh="10" />
"##;
    let expected_rect = r#"<rect id="z" x="10" y="30" width="10" height="10"/>"#;
    assert!(transform_str_default(rel_v_input)
        .unwrap()
        .contains(expected_rect));

    // with a gap
    let rel_v_input = r##"
<rect id="abc" xy="10 20" wh="10" />
<rect id="def" xy="30" wh="2" />
<rect id="z" xy="#abc:v 3" wh="10" />
"##;
    let expected_rect = r#"<rect id="z" x="10" y="33" width="10" height="10"/>"#;
    assert!(transform_str_default(rel_v_input)
        .unwrap()
        .contains(expected_rect));

    // ABOVE
    let rel_v_input = r##"
<rect id="abc" xy="10 20" wh="10" />
<rect id="def" xy="30" wh="2" />
<rect id="z" xy="#abc:V" wh="8" />
"##;
    let expected_rect = r#"<rect id="z" x="11" y="12" width="8" height="8"/>"#;
    assert!(transform_str_default(rel_v_input)
        .unwrap()
        .contains(expected_rect));

    // with a gap
    let rel_v_input = r##"
<rect id="abc" xy="10 20" wh="10" />
<rect id="def" xy="30" wh="2" />
<rect id="z" xy="#abc:V 3" wh="8" />
"##;
    let expected_rect = r#"<rect id="z" x="11" y="9" width="8" height="8"/>"#;
    assert!(transform_str_default(rel_v_input)
        .unwrap()
        .contains(expected_rect));
}

#[test]
fn test_rel_dx_dy() {
    let rel_h_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="^h" dxy="-1.23 4.56" wh="20 60" id="z"/>
"#;
    let expected_rect = r#"<rect x="28.77" y="24.56" width="20" height="60" id="z"/>"#;
    assert!(transform_str_default(rel_h_input)
        .unwrap()
        .contains(expected_rect));

    let rel_input = r##"
<rect xy="10 20" wh="20 60" id="abc"/>
<rect xy="98 99" wh="123 321" />
<rect xy="22 23" wh="234 654" />
<rect xy="#abc@tr 10 100" wh="20 60" id="z"/>
"##;
    let expected_rect = r#"<rect x="40" y="120" width="20" height="60" id="z"/>"#;
    assert!(transform_str_default(rel_input)
        .unwrap()
        .contains(expected_rect));
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
    assert!(transform_str_default(rel_refid_input)
        .unwrap()
        .contains(expected_rect));
    let expected_rect = r#"<rect x="64" y="20" width="7" height="7" id="y"/>"#;
    assert!(transform_str_default(rel_refid_input)
        .unwrap()
        .contains(expected_rect));
    let expected_rect = r#"<rect x="67" y="23" width="2" height="2" id="z"/>"#;
    assert!(transform_str_default(rel_refid_input)
        .unwrap()
        .contains(expected_rect));
}

#[test]
fn test_rel_multi_recursive() {
    // Ensure a relative position can be derived through many recursive references
    let rel_refid_input = r##"
<rect id="a" xy="#b:H" wh="2" />
<rect id="b" xy="#c:H" wh="2" />
<rect id="c" xy="#d:H" wh="2" />
<rect id="d" xy="#e:H" wh="2" />
<rect id="e" xy="#f:H" wh="2" />
<rect id="f" xy="#g:H" wh="2" />
<rect id="g" xy="50" wh="2" />
"##;
    let expected_rect = r#"<rect id="a" x="38" y="50" width="2" height="2"/>"#;
    assert!(transform_str_default(rel_refid_input)
        .unwrap()
        .contains(expected_rect));
}
