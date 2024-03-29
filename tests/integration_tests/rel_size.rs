use assertables::{assert_contains, assert_contains_as_result};
use svgdx::transform_str_default;

#[test]
fn test_rel_size_prev() {
    let rel_wh_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="25 35" wh="^" id="z"/>
"#;
    let expected_rect = r#"<rect x="25" y="35" width="20" height="60" id="z"/>"#;
    let output = transform_str_default(rel_wh_input).unwrap();
    assert_contains!(output, expected_rect);
}

#[test]
fn test_rel_size_refid() {
    let rel_size_refid_input = r##"
<rect xy="10 20" wh="20 60" id="abc"/>
<rect xy="98 99" wh="123 321" />
<rect xy="22 23" wh="234 654" />
<rect xy="12 0" wh="#abc" id="z"/>
"##;
    let expected_rect = r#"<rect x="12" y="0" width="20" height="60" id="z"/>"#;
    let output = transform_str_default(rel_size_refid_input).unwrap();
    assert_contains!(output, expected_rect);
}

#[test]
fn test_rel_size_dxy() {
    let rel_size_dxy_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="1 2" wh="^ 2 -5" id="z"/>
"#;
    let expected_rect = r#"<rect x="1" y="2" width="22" height="55" id="z"/>"#;
    let output = transform_str_default(rel_size_dxy_input).unwrap();
    assert_contains!(output, expected_rect);

    let rel_input = r##"
<rect xy="10 20" wh="20 60" id="abc"/>
<rect xy="98 99" wh="123 321" />
<rect xy="22 23" wh="234 654" />
<rect xy="1 2" wh="#abc -2 5" id="z"/>
"##;
    let expected_rect = r#"<rect x="1" y="2" width="18" height="65" id="z"/>"#;
    let output = transform_str_default(rel_input).unwrap();
    assert_contains!(output, expected_rect);
}

#[test]
fn test_rel_size_dxy_pct() {
    let rel_size_dxy_pct_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="1 2" wh="^ 110% 50%" id="z"/>
"#;
    let expected_rect = r#"<rect x="1" y="2" width="22" height="30" id="z"/>"#;
    let output = transform_str_default(rel_size_dxy_pct_input).unwrap();
    assert_contains!(output, expected_rect);

    let rel_input = r##"
<rect xy="10 20" wh="20 60" id="abc"/>
<rect xy="98 99" wh="123 321" />
<rect xy="22 23" wh="234 654" />
<rect xy="1 2" wh="#abc 40% 150%" id="z"/>
"##;
    let expected_rect = r#"<rect x="1" y="2" width="8" height="90" id="z"/>"#;
    let output = transform_str_default(rel_input).unwrap();
    assert_contains!(output, expected_rect);
}

#[test]
fn test_rel_size_recursive() {
    // Ensure a relative size can be derived from a referenced
    // (not just previous) element which is also relatively sized
    let rel_recur_input = r##"
<rect xy="10 20" wh="20 60" id="abc"/>
<rect xy="12 34" wh="^ 3 50%" id="x"/>
<rect xy="23 45" wh="2"/>
<rect xy="45 67" wh="2"/>
<rect xy="1 1" wh="2"/>
<rect xy="2 2" wh="#x 200% -3" id="y"/>
<rect xy="1 1" wh="2"/>
<rect xy="1 1" wh="2"/>
<rect xy="1 1" wh="#y" id="z"/>
"##;
    let expected_rect = r#"<rect x="12" y="34" width="23" height="30" id="x"/>"#;
    let output = transform_str_default(rel_recur_input).unwrap();
    assert_contains!(output, expected_rect);
    let expected_rect = r#"<rect x="2" y="2" width="46" height="27" id="y"/>"#;
    let output = transform_str_default(rel_recur_input).unwrap();
    assert_contains!(output, expected_rect);
    let expected_rect = r#"<rect x="1" y="1" width="46" height="27" id="z"/>"#;
    let output = transform_str_default(rel_recur_input).unwrap();
    assert_contains!(output, expected_rect);
}

#[test]
fn test_rel_size_dwh() {
    // Check that cxy can still centrally position a shape with a dwh
    let input = r##"
<rect wh="50" id="a"/>
<rect cxy="#a@t" wh="10" dwh="10 -5" id="b" text="b"/>
<rect cxy="#b@br" wh="1" id="z" />
"##;
    let expected_rect = r#"
<rect x="34.5" y="2" width="1" height="1" id="z"/>
"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_rect);
}
