use svgdx::transform_str_default;

#[test]
fn test_surround_single_rect() {
    let input = r##"
<rect id="a" xy="0" wh="5" />
<rect id="s" surround="#a" />
"##;
    let expected = r#"<rect id="s" x="0" y="0" width="5" height="5" class="d-surround"/>"#;

    assert!(transform_str_default(input).unwrap().contains(expected));
}

#[test]
fn test_surround_single_margin() {
    let input = r##"
<rect id="a" xy="0" wh="5" />
<rect id="s" surround="#a" margin="1" />
"##;
    let expected = r#"<rect id="s" x="-1" y="-1" width="7" height="7" class="d-surround"/>"#;

    assert!(transform_str_default(input).unwrap().contains(expected));
}

#[test]
fn test_surround_multi_rect() {
    let input = r##"
<rect id="a" xy="0" wh="5" />
<rect id="b" xy="2" wh="2 10" />
<rect id="c" xy="8" wh="1" />
<rect id="s" surround="#a #b #c" />
"##;
    let expected = r#"<rect id="s" x="0" y="0" width="9" height="12" class="d-surround"/>"#;

    assert!(transform_str_default(input).unwrap().contains(expected));
}

#[test]
fn test_surround_multi_margin() {
    let input = r##"
<rect id="a" xy="0" wh="5" />
<rect id="b" xy="2" wh="2 10" />
<rect id="c" xy="8" wh="1" />
<rect id="s" surround="#a #b #c" margin="1.25 3"/>
"##;
    let expected = r#"<rect id="s" x="-1.25" y="-3" width="11.5" height="18" class="d-surround"/>"#;

    assert!(transform_str_default(input).unwrap().contains(expected));
}
