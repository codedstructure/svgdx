use assertables::assert_contains;
use svgdx::transform_str_default;

#[test]
fn test_rel_size_prev() {
    let rel_wh_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="25 35" wh="^" id="z"/>
"#;
    let expected_rect = r#"<rect id="z" x="25" y="35" width="20" height="60"/>"#;
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
    let expected_rect = r#"<rect id="z" x="12" y="0" width="20" height="60"/>"#;
    let output = transform_str_default(rel_size_refid_input).unwrap();
    assert_contains!(output, expected_rect);
}

#[test]
fn test_rel_size_dxy() {
    let rel_size_dxy_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="1 2" wh="^ 2 -5" id="z"/>
"#;
    let expected_rect = r#"<rect id="z" x="1" y="2" width="22" height="55"/>"#;
    let output = transform_str_default(rel_size_dxy_input).unwrap();
    assert_contains!(output, expected_rect);

    let rel_input = r##"
<rect xy="10 20" wh="20 60" id="abc"/>
<rect xy="98 99" wh="123 321" />
<rect xy="22 23" wh="234 654" />
<rect xy="1 2" wh="#abc -2 5" id="z"/>
"##;
    let expected_rect = r#"<rect id="z" x="1" y="2" width="18" height="65"/>"#;
    let output = transform_str_default(rel_input).unwrap();
    assert_contains!(output, expected_rect);
}

#[test]
fn test_rel_size_delta() {
    // No delta
    let rel_input = r##"
<rect wh="20 60" id="abc"/>
<rect xy="40" wh="#abc"/>
"##;
    let expected_rect = r#"<rect x="40" y="40" width="20" height="60"/>"#;
    let output = transform_str_default(rel_input).unwrap();
    assert_contains!(output, expected_rect);

    // Single delta - %age
    let rel_input = r##"
<rect wh="20 60" id="abc"/>
<rect xy="40" wh="#abc 10%"/>
"##;
    let expected_rect = r#"<rect x="40" y="40" width="2" height="6"/>"#;
    let output = transform_str_default(rel_input).unwrap();
    assert_contains!(output, expected_rect);

    // Single delta - absolute
    let rel_input = r##"
<rect wh="20 60" id="abc"/>
<rect xy="40" wh="#abc 10"/>
"##;
    let expected_rect = r#"<rect x="40" y="40" width="30" height="70"/>"#;
    let output = transform_str_default(rel_input).unwrap();
    assert_contains!(output, expected_rect);

    // Double delta - %age
    let rel_input = r##"
<rect wh="20 60" id="abc"/>
<rect xy="40" wh="#abc 10% 20%"/>
"##;
    let expected_rect = r#"<rect x="40" y="40" width="2" height="12"/>"#;
    let output = transform_str_default(rel_input).unwrap();
    assert_contains!(output, expected_rect);

    // Double delta - absolute
    let rel_input = r##"
<rect wh="20 60" id="abc"/>
<rect xy="40" wh="#abc 10 5"/>
"##;
    let expected_rect = r#"<rect x="40" y="40" width="30" height="65"/>"#;
    let output = transform_str_default(rel_input).unwrap();
    assert_contains!(output, expected_rect);
}

#[test]
fn test_rel_size_circle() {
    let rel_size_circle_input = r#"
<circle r="20" />
<circle xy="^:h 10" r="^" id="z"/>
"#;
    let expected_circle = r#"<circle id="z" cx="50" cy="0" r="20"/>"#;
    let output = transform_str_default(rel_size_circle_input).unwrap();
    assert_contains!(output, expected_circle);

    let rel_size_circle_input = r#"
<circle r="20" />
<circle xy="^:h 10" r="^ 50%" id="z"/>
"#;
    let expected_circle = r#"<circle id="z" cx="40" cy="0" r="10"/>"#;
    let output = transform_str_default(rel_size_circle_input).unwrap();
    assert_contains!(output, expected_circle);
}

#[test]
fn test_rel_size_ellipse() {
    let rel_size_ellipse_input = r#"
<ellipse rxy="10 20" />
<ellipse xy="^:h 10" rxy="^" id="z"/>
"#;
    let expected_ellipse = r#"<ellipse id="z" cx="30" cy="0" rx="10" ry="20"/>"#;
    let output = transform_str_default(rel_size_ellipse_input).unwrap();
    assert_contains!(output, expected_ellipse);

    let rel_size_ellipse_input = r#"
<ellipse rxy="10 20" />
<ellipse xy="^:h 10" rxy="^ 50%" id="z"/>
"#;
    let expected_ellipse = r#"<ellipse id="z" cx="25" cy="0" rx="5" ry="10"/>"#;
    let output = transform_str_default(rel_size_ellipse_input).unwrap();
    assert_contains!(output, expected_ellipse);
}

#[test]
fn test_rel_size_dxy_pct() {
    let rel_size_dxy_pct_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="1 2" wh="^ 110% 50%" id="z"/>
"#;
    let expected_rect = r#"<rect id="z" x="1" y="2" width="22" height="30"/>"#;
    let output = transform_str_default(rel_size_dxy_pct_input).unwrap();
    assert_contains!(output, expected_rect);

    let rel_input = r##"
<rect xy="10 20" wh="20 60" id="abc"/>
<rect xy="98 99" wh="123 321" />
<rect xy="22 23" wh="234 654" />
<rect xy="1 2" wh="#abc 40% 150%" id="z"/>
"##;
    let expected_rect = r#"<rect id="z" x="1" y="2" width="8" height="90"/>"#;
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
    let expected_rect = r#"<rect id="x" x="12" y="34" width="23" height="30"/>"#;
    let output = transform_str_default(rel_recur_input).unwrap();
    assert_contains!(output, expected_rect);
    let expected_rect = r#"<rect id="y" x="2" y="2" width="46" height="27"/>"#;
    let output = transform_str_default(rel_recur_input).unwrap();
    assert_contains!(output, expected_rect);
    let expected_rect = r#"<rect id="z" x="1" y="1" width="46" height="27"/>"#;
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
<rect id="z" x="34.5" y="2" width="1" height="1"/>
"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_rect);
}

#[test]
fn test_rel_size_dwh_anchor() {
    // 'anchor' position (e.g. from xy-loc or implicit cx/cy) shouldn't change if dwh is applied.

    // Basic (implied top-left)
    let input = r#"<rect xy="10" wh="5" dwh="1 2"/>"#;
    let expected = r#"<rect x="10" y="10" width="6" height="7"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r#"<rect xy="10" wh="6" dwh="50% 125%"/>"#;
    let expected = r#"<rect x="10" y="10" width="3" height="7.5"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    // Centre - implicit from cxy
    let input = r#"<rect cxy="10" wh="5" dwh="1 2"/>"#;
    let expected = r#"<rect x="7" y="6.5" width="6" height="7"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r#"<rect cxy="10" wh="6" dwh="50% 125%"/>"#;
    let expected = r#"<rect x="8.5" y="6.25" width="3" height="7.5"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    // Bottom-right - explicit from xy-loc
    let input = r#"<rect xy="10" wh="5" dwh="1 2" xy-loc="br"/>"#;
    let expected = r#"<rect x="4" y="3" width="6" height="7"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r#"<rect xy="10" wh="6" dwh="50% 125%" xy-loc="br"/>"#;
    let expected = r#"<rect x="7" y="2.5" width="3" height="7.5"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}
