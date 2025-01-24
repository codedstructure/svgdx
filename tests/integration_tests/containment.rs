use assertables::assert_contains;
use itertools::Itertools;
use svgdx::transform_str_default;

#[test]
fn test_surround_single_rect() {
    let input = r##"
<rect id="a" xy="0" wh="5" />
<rect id="s" surround="#a" />
"##;
    let expected = r#"<rect id="s" x="0" y="0" width="5" height="5" class="d-surround"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_surround_single_margin() {
    let input = r##"
<rect id="a" xy="0" wh="5" />
<rect id="s" surround="#a" margin="1" />
"##;
    let expected = r#"<rect id="s" x="-1" y="-1" width="7" height="7" class="d-surround"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
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
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_surround_multi_margin() {
    let input = r##"
<rect id="a" xy="0" wh="5" />
<rect id="b" xy="2" wh="2 10" />
<rect id="c" xy="8" wh="1" />
<rect id="s" surround="#a #b #c" margin="1.25 3"/>
"##;
    let expected = r#"<rect id="s" x="-3" y="-1.25" width="15" height="14.5" class="d-surround"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_surround_non_rect() {
    let input = r##"
<rect id="a" xy="0" wh="5" />
<rect id="b" xy="2 0" wh="5" />
<circle id="s" surround="#a #b" margin="2 1"/>
"##;
    let expected = r#"<circle id="s" cx="3.5" cy="2.5" r="6.364" class="d-surround"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r##"
<rect id="a" xy="0" wh="5" />
<rect id="b" xy="2 0" wh="5" />
<ellipse id="s" surround="#a #b" margin="1 2"/>
"##;
    let expected = r#"<ellipse id="s" cx="3.5" cy="2.5" rx="7.778" ry="4.95" class="d-surround"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_surround_recursive() {
    // Check the surround items can refer to later objects
    let input = r##"
<rect id="s" surround="#a #b #c" />
<rect id="a" xy="0" wh="5" />
<rect id="b" xy="2" wh="2 10" />
<rect id="c" surround="#p #q" margin="4" />
<rect id="p" xy="#b|h 3" wh="5" />
<rect id="q" xy="#b|v 3" wh="5" />
"##;
    let expected = r#"<rect id="s" x="-3.5" y="0" width="19.5" height="24" class="d-surround"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_surround_connectors() {
    // Check connectors can be created between surround objects
    let input = r##"
<rect id="s1" surround="#a #b" margin="1" />
<rect id="a" xy="0" wh="5" />
<rect id="b" xy="2" wh="2 10" />
<rect id="s2" surround="#p #q" margin="2" />
<rect id="p" xy="#b|h 20" wh="5" />
<rect id="q" xy="#p|v 3" wh="5" />
<polyline id="ll" start="#s1" end="#s2"/>
"##;
    let expected1 = r#"<rect id="s1" x="-1" y="-1" width="7" height="14" class="d-surround"/>"#;
    let expected2 = r#"<rect id="s2" x="22" y="2.5" width="9" height="17" class="d-surround"/>"#;
    let expected3 = r#"<polyline id="ll" points="6 6, 14 6, 14 11, 22 11"/>"#;

    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
    assert_contains!(output, expected3);
}

#[test]
fn test_surround_bad() {
    // This is unsolvable and should fail: s->c->p->s
    let input = r##"
<rect id="s" surround="#a #b #c" />
<rect id="a" xy="0" wh="5" />
<rect id="b" xy="#2" wh="2 10" />
<rect id="c" surround="#p #q" margin="4" />
<rect id="p" xy="#s|h 3" wh="5" />
<rect id="q" xy="#b|v 3" wh="5" />
"##;
    let output = transform_str_default(input);
    assert!(output.is_err());
}

#[test]
fn test_surround_connectors_permute() {
    // Now for the tough bit: for every ordering of elements, the result should still work.
    // Note collapsed line pairs to reduce number of permutations (since O(n!)...)
    let input = r##"
<rect id="s1" surround="#a #b" margin="1" />
<rect id="a" xy="0" wh="5" /><rect id="b" xy="2" wh="2 10" />
<rect id="s2" surround="#p #q" margin="2" />
<rect id="p" xy="#b|h 20" wh="5" /><rect id="q" xy="#p|v 3" wh="5" />
<polyline id="ll" start="#s1" end="#s2"/>
"##;

    let lines: Vec<_> = input.trim().lines().collect();
    for testcase in lines.iter().permutations(lines.len()) {
        let input = testcase.iter().join("\n");
        let output = transform_str_default(input).unwrap();

        let expected1 = r#"<rect id="s1" x="-1" y="-1" width="7" height="14" class="d-surround"/>"#;
        let expected2 =
            r#"<rect id="s2" x="22" y="2.5" width="9" height="17" class="d-surround"/>"#;
        let expected3 = r#"<polyline id="ll" points="6 6, 14 6, 14 11, 22 11"/>"#;

        assert_contains!(output, expected1);
        assert_contains!(output, expected2);
        assert_contains!(output, expected3);
    }
}

#[test]
fn test_inside_simple() {
    let input = r##"
<rect id="a" wh="20" />
<rect id="z" inside="#a" />
"##;
    let expected = r#"<rect id="z" x="0" y="0" width="20" height="20" class="d-inside"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r##"
<rect id="a" wh="20" />
<rect id="b" wh="50" />
<rect id="z" inside="#a #b" />
"##;
    let expected = r#"<rect id="z" x="0" y="0" width="20" height="20" class="d-inside"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r##"
<rect id="a" wh="20" />
<rect id="b" wh="10" />
<rect id="z" inside="#a #b" />
"##;
    let expected = r#"<rect id="z" x="0" y="0" width="10" height="10" class="d-inside"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_inside_margin() {
    let input = r##"
<rect wh="20" id="a" />
<rect xy="10" wh="20" id="b" />
<rect id="r" inside="#a #b" margin="15%" />
"##;
    let expected = r#"<rect id="r" x="11.5" y="11.5" width="7" height="7" class="d-inside"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r##"
<rect wh="20" id="a" />
<rect xy="10" wh="20" id="b" />
<rect id="r" inside="#a #b" margin="3" />
"##;
    let expected = r#"<rect id="r" x="13" y="13" width="4" height="4" class="d-inside"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r##"
<rect wh="20" id="a" />
<rect xy="10" wh="20" id="b" />
<rect id="r" inside="#a #b" margin="3 10%" />
"##;
    let expected = r#"<rect id="r" x="11" y="13" width="8" height="4" class="d-inside"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_inside_simple_nonrect() {
    let input = r##"
<circle id="a" r="0.5"/>
<circle id="z" inside="#a"/>
"##;
    let expected = r#"<circle id="z" cx="0" cy="0" r="0.5" class="d-inside"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r##"
<circle id="a" r="0.5"/>
<rect id="z" inside="#a"/>
"##;
    let expected =
        r#"<rect id="z" x="-0.354" y="-0.354" width="0.707" height="0.707" class="d-inside"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r##"
<ellipse id="a" rx="1" ry="0.5"/>
<rect id="z" inside="#a"/>
"##;
    let expected =
        r#"<rect id="z" x="-0.707" y="-0.354" width="1.414" height="0.707" class="d-inside"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_inside_mixed_nonrect() {
    let input = r##"
<rect id="a" wh="2"/>
<circle id="b" cxy="#a@r" r="1"/>
<rect id="z" inside="#a #b"/>
"##;
    let expected =
        r#"<rect id="z" x="1.293" y="0.293" width="0.707" height="1.414" class="d-inside"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r##"
<rect id="a" wh="2"/>
<circle id="b" cxy="#a@r" r="1"/>
<circle id="z" inside="#a #b"/>
"##;
    let expected = r#"<circle id="z" cx="1.5" cy="1" r="0.5" class="d-inside"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    // TODO: there are bunch of cases which don't work properly yet when non-rects
    // are involved, e.g. ellipse inside rect+circle, non-axis-aligned shapes, etc.
}

#[test]
fn test_inside_surround_invalid() {
    // Cannot have an element with both surround and inside attributes
    let input = r##"
<rect wh="20" id="a" />
<rect xy="10" wh="20" id="b" />
<rect id="r" inside="#a #b" surround="#a #b" margin="2" />
"##;
    let output = transform_str_default(input);
    assert!(output.is_err());

    // Cannot have an element with either surround or inside referring to a non-present id
    let input = r##"
<rect wh="20" id="a" />
<rect xy="10" wh="20" id="b" />
<rect id="r" inside="#a #b #c" margin="2" />
"##;
    let output = transform_str_default(input);
    assert!(output.is_err());

    let input = r##"
<rect wh="20" id="a" />
<rect xy="10" wh="20" id="b" />
<rect id="r" surround="#a #b #c" margin="2" />
"##;
    let output = transform_str_default(input);
    assert!(output.is_err());
}
