use assertables::assert_contains;
use svgdx::transform_str_default;

#[test]
fn test_rel_prev() {
    let rel_h_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="^@tr" wh="20 60" id="z"/>
"#;
    let expected_rect = r#"<rect id="z" x="30" y="20" width="20" height="60"/>"#;
    let output = transform_str_default(rel_h_input).unwrap();
    assert_contains!(output, expected_rect);

    let rel_v_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="^@bl -1 1" wh="20 60" id="z"/>
"#;
    let expected_rect = r#"<rect id="z" x="9" y="81" width="20" height="60"/>"#;
    let output = transform_str_default(rel_v_input).unwrap();
    assert_contains!(output, expected_rect);
}

#[test]
fn test_rel_refid() {
    let rel_refid_input = r##"
<rect xy="10 20" wh="20 60" id="abc"/>
<rect xy="98 99" wh="123 321" />
<rect xy="22 23" wh="234 654" />
<rect xy="#abc@tr" wh="20 60" id="z"/>
"##;
    let expected_rect = r#"<rect id="z" x="30" y="20" width="20" height="60"/>"#;
    let output = transform_str_default(rel_refid_input).unwrap();
    assert_contains!(output, expected_rect);
}

#[test]
fn test_relh() {
    // TO THE RIGHT
    let rel_h_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="^|h" wh="20 60" id="z"/>
"#;
    let expected_rect = r#"<rect id="z" x="30" y="20" width="20" height="60"/>"#;
    let output = transform_str_default(rel_h_input).unwrap();
    assert_contains!(output, expected_rect);

    // With a gap
    let rel_h_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="^|h 3" wh="20 60" id="z"/>
"#;
    let expected_rect = r#"<rect id="z" x="33" y="20" width="20" height="60"/>"#;
    let output = transform_str_default(rel_h_input).unwrap();
    assert_contains!(output, expected_rect);

    // TO THE LEFT
    let rel_h_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="^|H" wh="20 60" id="z"/>
"#;
    let expected_rect = r#"<rect id="z" x="-10" y="20" width="20" height="60"/>"#;
    let output = transform_str_default(rel_h_input).unwrap();
    assert_contains!(output, expected_rect);

    // With a gap
    let rel_h_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="^|H 3" wh="20 60" id="z"/>
"#;
    let expected_rect = r#"<rect id="z" x="-13" y="20" width="20" height="60"/>"#;
    let output = transform_str_default(rel_h_input).unwrap();
    assert_contains!(output, expected_rect);
}

#[test]
fn test_relv() {
    // VERTICAL BELOW
    let rel_v_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="^|v" wh="20 60" id="z"/>
"#;
    let expected_rect = r#"<rect id="z" x="10" y="80" width="20" height="60"/>"#;
    let output = transform_str_default(rel_v_input).unwrap();
    assert_contains!(output, expected_rect);

    // With a gap
    let rel_v_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="^|v 5" wh="20 60" id="z"/>
"#;
    let expected_rect = r#"<rect id="z" x="10" y="85" width="20" height="60"/>"#;
    let output = transform_str_default(rel_v_input).unwrap();
    assert_contains!(output, expected_rect);

    // VERTICAL ABOVE
    let rel_v_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="^|V" wh="20 60" id="z"/>
"#;
    let expected_rect = r#"<rect id="z" x="10" y="-40" width="20" height="60"/>"#;
    let output = transform_str_default(rel_v_input).unwrap();
    assert_contains!(output, expected_rect);

    // With a gap
    let rel_v_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="^|V 5" wh="20 60" id="z"/>
"#;
    let expected_rect = r#"<rect id="z" x="10" y="-45" width="20" height="60"/>"#;
    let output = transform_str_default(rel_v_input).unwrap();
    assert_contains!(output, expected_rect);
}

#[test]
fn test_elref_relh() {
    // TO THE RIGHT
    let rel_h_input = r##"
<rect id="abc" xy="10 20" wh="10" />
<rect id="def" xy="30" wh="2" />
<rect id="z" xy="#abc|h" wh="10" />
"##;
    let expected_rect = r#"<rect id="z" x="20" y="20" width="10" height="10"/>"#;
    let output = transform_str_default(rel_h_input).unwrap();
    assert_contains!(output, expected_rect);

    // with a gap
    let rel_h_input = r##"
<rect id="abc" xy="10 20" wh="10" />
<rect id="def" xy="30" wh="2" />
<rect id="z" xy="#abc|h 3" wh="10" />
"##;
    let expected_rect = r#"<rect id="z" x="23" y="20" width="10" height="10"/>"#;
    let output = transform_str_default(rel_h_input).unwrap();
    assert_contains!(output, expected_rect);

    // TO THE LEFT
    let rel_h_input = r##"
<rect id="abc" xy="10 20" wh="10" />
<rect id="def" xy="30" wh="2" />
<rect id="z" xy="#abc|H" wh="8" />
"##;
    let expected_rect = r#"<rect id="z" x="2" y="21" width="8" height="8"/>"#;
    let output = transform_str_default(rel_h_input).unwrap();
    assert_contains!(output, expected_rect);

    // with a gap
    let rel_h_input = r##"
<rect id="abc" xy="10 20" wh="10" />
<rect id="def" xy="30" wh="2" />
<rect id="z" xy="#abc|H 3" wh="8" />
"##;
    let expected_rect = r#"<rect id="z" x="-1" y="21" width="8" height="8"/>"#;
    let output = transform_str_default(rel_h_input).unwrap();
    assert_contains!(output, expected_rect);
}

#[test]
fn test_elref_relv() {
    // BELOW
    let rel_v_input = r##"
<rect id="abc" xy="10 20" wh="10" />
<rect id="def" xy="30" wh="2" />
<rect id="z" xy="#abc|v" wh="10" />
"##;
    let expected_rect = r#"<rect id="z" x="10" y="30" width="10" height="10"/>"#;
    let output = transform_str_default(rel_v_input).unwrap();
    assert_contains!(output, expected_rect);

    // with a gap
    let rel_v_input = r##"
<rect id="abc" xy="10 20" wh="10" />
<rect id="def" xy="30" wh="2" />
<rect id="z" xy="#abc|v 3" wh="10" />
"##;
    let expected_rect = r#"<rect id="z" x="10" y="33" width="10" height="10"/>"#;
    let output = transform_str_default(rel_v_input).unwrap();
    assert_contains!(output, expected_rect);

    // ABOVE
    let rel_v_input = r##"
<rect id="abc" xy="10 20" wh="10" />
<rect id="def" xy="30" wh="2" />
<rect id="z" xy="#abc|V" wh="8" />
"##;
    let expected_rect = r#"<rect id="z" x="11" y="12" width="8" height="8"/>"#;
    let output = transform_str_default(rel_v_input).unwrap();
    assert_contains!(output, expected_rect);

    // with a gap
    let rel_v_input = r##"
<rect id="abc" xy="10 20" wh="10" />
<rect id="def" xy="30" wh="2" />
<rect id="z" xy="#abc|V 3" wh="8" />
"##;
    let expected_rect = r#"<rect id="z" x="11" y="9" width="8" height="8"/>"#;
    let output = transform_str_default(rel_v_input).unwrap();
    assert_contains!(output, expected_rect);
}

#[test]
fn test_rel_dx_dy() {
    let rel_h_input = r#"
<rect xy="10 20" wh="20 60" />
<rect xy="^|h" dxy="-1.23 4.56" wh="20 60" id="z"/>
"#;
    let expected_rect = r#"<rect id="z" x="28.77" y="24.56" width="20" height="60"/>"#;
    let output = transform_str_default(rel_h_input).unwrap();
    assert_contains!(output, expected_rect);

    let rel_input = r#"
<rect xy="10" wh="20" />
<rect id="z" xy="^|h 5" dy="-1" wh="20"/>
"#;
    let expected_rect = r#"<rect id="z" x="35" y="9" width="20" height="20"/>"#;
    let output = transform_str_default(rel_input).unwrap();
    assert_contains!(output, expected_rect);

    let rel_input = r##"
<rect xy="10 20" wh="20 60" id="abc"/>
<rect xy="98 99" wh="123 321" />
<rect xy="22 23" wh="234 654" />
<rect xy="#abc@tr 10 100" wh="20 60" id="z"/>
"##;
    let expected_rect = r#"<rect id="z" x="40" y="120" width="20" height="60"/>"#;
    let output = transform_str_default(rel_input).unwrap();
    assert_contains!(output, expected_rect);
}

#[test]
fn test_rel_pos_delta() {
    // No delta
    let rel_input = r##"
<rect xy="20 60" wh="10" id="abc"/>
<rect xy="#abc" wh="10"/>
"##;
    let expected_rect = r#"<rect x="20" y="60" width="10" height="10"/>"#;
    let output = transform_str_default(rel_input).unwrap();
    assert_contains!(output, expected_rect);

    // Single delta
    let rel_input = r##"
<rect xy="20 60" wh="10" id="abc"/>
<rect xy="#abc -5" wh="10"/>
"##;
    let expected_rect = r#"<rect x="15" y="55" width="10" height="10"/>"#;
    let output = transform_str_default(rel_input).unwrap();
    assert_contains!(output, expected_rect);

    // Double delta
    let rel_input = r##"
<rect xy="20 60" wh="10" id="abc"/>
<rect xy="#abc -5 10" wh="10"/>
"##;
    let expected_rect = r#"<rect x="15" y="70" width="10" height="10"/>"#;
    let output = transform_str_default(rel_input).unwrap();
    assert_contains!(output, expected_rect);
}

#[test]
fn test_rel_recursive() {
    // Ensure a relative position can be derived from a referenced
    // (not just previous) element which is also relatively positioned
    let rel_refid_input = r##"
<rect xy="10 20" wh="20 60" id="abc"/>
<rect xy="^@tr 12 0" wh="10" id="x"/>
<rect xy="^@tl 1 1" wh="2"/>
<rect xy="^@tl 1 1" wh="2"/>
<rect xy="^@tl 1 1" wh="2"/>
<rect xy="#x@tr 12 0" wh="7" id="y"/>
<rect xy="^@tl 1 1" wh="2"/>
<rect xy="^@tl 1 1" wh="2"/>
<rect xy="^@tl 1 1" wh="2" id="z"/>
"##;
    let expected_rect = r#"<rect id="x" x="42" y="20" width="10" height="10"/>"#;
    let output = transform_str_default(rel_refid_input).unwrap();
    assert_contains!(output, expected_rect);
    let expected_rect = r#"<rect id="y" x="64" y="20" width="7" height="7"/>"#;
    let output = transform_str_default(rel_refid_input).unwrap();
    assert_contains!(output, expected_rect);
    let expected_rect = r#"<rect id="z" x="67" y="23" width="2" height="2"/>"#;
    let output = transform_str_default(rel_refid_input).unwrap();
    assert_contains!(output, expected_rect);
}

#[test]
fn test_rel_multi_recursive() {
    // Ensure a relative position can be derived through many recursive references
    let rel_refid_input = r##"
<rect id="a" xy="#b|H" wh="2" />
<rect id="b" xy="#c|H" wh="2" />
<rect id="c" xy="#d|H" wh="2" />
<rect id="d" xy="#e|H" wh="2" />
<rect id="e" xy="#f|H" wh="2" />
<rect id="f" xy="#g|H" wh="2" />
<rect id="g" xy="50" wh="2" />
"##;
    let expected_rect = r#"<rect id="a" x="38" y="50" width="2" height="2"/>"#;
    let output = transform_str_default(rel_refid_input).unwrap();
    assert_contains!(output, expected_rect);

    // Ensure a relative position can be derived through many recursive references
    let rel_refid_input = r##"
<rect id="a" xy="+|H" wh="2" />
<rect id="b" xy="+|H" wh="2" />
<rect id="c" xy="++|H" wh="2" />
<rect id="d" xy="+++|H" wh="2" />
<rect id="e" xy="+|H" wh="2" />
<rect id="f" xy="^^|H" wh="2" />
<rect id="g" xy="50" wh="2" />
"##;
    let expected_rect = r#"<rect id="a" x="38" y="50" width="2" height="2"/>"#;
    let output = transform_str_default(rel_refid_input).unwrap();
    assert_contains!(output, expected_rect);
}

#[test]
fn test_rel_scalar_point() {
    let input = r##"
<rect id="a" xy="10" wh="100 50" />
<circle x1="#a" y2="#a@l" wh="10" />
"##;
    let expected = r#"<circle cx="15" cy="30" r="5"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_rel_scalar_point_delta() {
    let input = r##"
<rect id="a" xy="10" wh="100 50" />
<circle x1="#a" y2="#a@l 3" wh="10" />
"##;
    let expected = r#"<circle cx="15" cy="33" r="5"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r##"
<rect xy="2 6" wh="10" id="abc"/>
<rect x="#abc" wh="4" dxy="1"/>
"##;
    let expected = r#"<rect x="3" y="1" width="4" height="4"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_path_refspec() {
    let input = r##"
  <box id="p" xy="0 15" wh="15 5"/>
  <path d="M #p@tl h #p~w v #p~h h -#p~w M #p@t:60% v #p~h M #p@t:70% v #p~h M #p@t:80% v #p~h M #p@t:90% v #p~h"/>
"##;
    let expected =
        r#"<path d="M 0 15 h 15 v 5 h -15 M 9 15 v 5 M 10.5 15 v 5 M 12 15 v 5 M 13.5 15 v 5"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_line_relpos() {
    let input = r#"
<line xy1="0 5" xy2="10 5"/>
<line xy="^|v 1" wh="^"/>"#;
    let expected = r#"
<line x1="0" y1="5" x2="10" y2="5"/>
<line x1="0" y1="6" x2="10" y2="6"/>"#;
    assert_eq!(transform_str_default(input).unwrap(), expected);
}

#[test]
fn test_dirspec_next() {
    let input = r#"
<rect id="z1" xy="+|v" wh="2"/>
<rect id="z2" xy="10" wh="2"/>
"#;
    let expected1 = r#"<rect id="z1" x="10" y="12" width="2" height="2"/>"#;
    let expected2 = r#"<rect id="z2" x="10" y="10" width="2" height="2"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);

    let input = r#"
<circle id="z1" xy="+|v" r="2"/>
<rect id="z2" xy="10" wh="2"/>
"#;
    let expected1 = r#"<circle id="z1" cx="11" cy="14" r="2"/>"#;
    let expected2 = r#"<rect id="z2" x="10" y="10" width="2" height="2"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
}

#[test]
fn test_rational_delta() {
    let input = r##"
<rect id="a" wh="100 60"/>
<rect id="l" xy="#a 1/10 1/10" wh="#a 2/10 8/10"/>
<rect id="m" xy="#a 5/10 1/10" xy-loc="t" wh="^"/>
<rect id="r" xy="#a 9/10 1/10" xy-loc="tr" wh="^"/>
"##;
    let expected1 = r#"<rect id="l" x="10" y="6" width="20" height="48"/>"#;
    let expected2 = r#"<rect id="m" x="40" y="6" width="20" height="48"/>"#;
    let expected3 = r#"<rect id="r" x="70" y="6" width="20" height="48"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
    assert_contains!(output, expected3);
}

#[test]
fn test_rational_xy2_delta() {
    // use implicit 'xy2 implies from bottom right of ref'
    let input = r##"
<rect id="a" wh="100 60"/>
<rect id="l" xy="#a 1/10" wh="#a 4/10 8/10"/>
<rect id="r" xy2="#a -1/10" wh="^"/>
"##;
    let expected1 = r#"<rect id="l" x="10" y="6" width="40" height="48"/>"#;
    let expected2 = r#"<rect id="r" x="50" y="6" width="40" height="48"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
}

#[test]
fn test_mixed_locspec_rational_delta() {
    // supply locspec on ref element, use rational and absolute deltas
    let input = r##"
<rect id="a" wh="100 50"/>
<rect id="b" cxy="#a@c 2/10 0" wh="#a 2/10"/>
<rect id="c" cxy="#a@c -2/10 -1" wh="#a 2/10"/>
<rect id="d" cxy="#a@tl 3/10 1" wh="#a 2/10"/>
"##;
    let expected1 = r#"<rect id="b" x="60" y="20" width="20" height="10"/>"#;
    let expected2 = r#"<rect id="c" x="20" y="19" width="20" height="10"/>"#;
    let expected3 = r#"<rect id="d" x="20" y="-4" width="20" height="10"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
    assert_contains!(output, expected3);
}

#[test]
fn test_dirspec_broken() {
    // missing '#' - should error.
    let input = r##"
<rect id="z1" wh="2"/>
<rect id="z2" xy="z1|h" wh="2"/>"##;
    assert!(transform_str_default(input).is_err());
}
