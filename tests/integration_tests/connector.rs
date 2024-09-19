use assertables::{assert_contains, assert_contains_as_result};
use svgdx::transform_str_default;

const RECT_SVG: &str = r#"
<rect x="0" y="0" width="5" height="5" id="a" />
<rect x="20" y="0" width="5" height="5" id="b" />
<rect x="0" y="20" width="5" height="5" id="c" />
<rect x="20" y="20" width="5" height="5" id="d" />
"#;

#[test]
fn test_connector() {
    let input = format!(r##"{RECT_SVG}<polyline start="#a@b" end="#d@t" />"##);
    let expected_line = r#"<polyline points="2.5 5, 2.5 12.5, 22.5 12.5, 22.5 20"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_line);

    let input = format!(r##"{RECT_SVG}<polyline start="#a@r" end="#d@t" />"##);
    let expected_line = r#"<polyline points="5 2.5, 22.5 2.5, 22.5 20"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_line);
}

#[test]
fn test_connector_closest() {
    let input = format!(r##"{RECT_SVG}<line start="#a" end="#d" />"##);
    let expected_line = r#"<line x1="5" y1="5" x2="20" y2="20"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_line);

    let input = format!(r##"{RECT_SVG}<line start="#a" end="#b" />"##);
    let expected_line = r#"<line x1="5" y1="2.5" x2="20" y2="2.5"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_line);
}

#[test]
fn test_connector_fixed_start() {
    let input = format!(r##"{RECT_SVG}<line start="3 7" end="#d@t" />"##);
    let expected_line = r#"<line x1="3" y1="7" x2="22.5" y2="20"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_line);
}

#[test]
fn test_connector_fixed_end() {
    let input = format!(r##"{RECT_SVG}<line start="#a@r" end="10 17" />"##);
    let expected_line = r#"<line x1="5" y1="2.5" x2="10" y2="17"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_line);
}

#[test]
fn test_connector_h() {
    let input = r##"
<rect x="0" y="0" width="5" height="5" id="a" />
<rect x="20" y="2" width="5" height="5" id="b" />
<line start="#a" end="#b" edge-type="h"/>"##;
    let expected_line = r#"<line x1="5" y1="3.5" x2="20" y2="3.5"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_line);
}

#[test]
fn test_connector_v() {
    let input = r##"
<rect x="0" y="0" width="5" height="5" id="a" />
<rect x="1" y="20" width="5" height="5" id="b" />
<line start="#a" end="#b" edge-type="v"/>"##;
    let expected_line = r#"<line x1="3" y1="5" x2="3" y2="20"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_line);
}

#[test]
fn test_connector_u_bb() {
    // Loc b->b
    let input = r##"
<rect xy="0" wh="5" id="a" />
<rect xy="10 0" wh="5" id="b" />
<polyline start="#a@b" end="#b@b" corner-offset="2"/>"##;
    let expected_line = r#"<polyline points="2.5 5, 2.5 7, 12.5 7, 12.5 5"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_line);

    // default corner-offset for this edge type is 3
    let input = r##"
<rect xy="0" wh="5" id="a" />
<rect xy="10 0" wh="5" id="b" />
<polyline start="#a@b" end="#b@b"/>"##;
    let expected_line = r#"<polyline points="2.5 5, 2.5 8, 12.5 8, 12.5 5"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_line);
}

#[test]
fn test_connector_u_tt() {
    // Loc t->t
    let input = r##"
<rect xy="0" wh="5" id="a" />
<rect xy="10 0" wh="5" id="b" />
<polyline start="#a@t" end="#b@t" corner-offset="2"/>"##;
    let expected_line = r#"<polyline points="2.5 0, 2.5 -2, 12.5 -2, 12.5 0"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_line);

    // default corner-offset for this edge type is 3
    let input = r##"
<rect xy="0" wh="5" id="a" />
<rect xy="10 0" wh="5" id="b" />
<polyline start="#a@t" end="#b@t"/>"##;
    let expected_line = r#"<polyline points="2.5 0, 2.5 -3, 12.5 -3, 12.5 0"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_line);
}

#[test]
fn test_connector_u_ll() {
    // Loc l->l
    let input = r##"
<rect xy="0" wh="5" id="a" />
<rect xy="0 10" wh="5" id="b" />
<polyline start="#a@l" end="#b@l" corner-offset="2"/>"##;
    let expected_line = r#"<polyline points="0 2.5, -2 2.5, -2 12.5, 0 12.5"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_line);

    // default corner-offset for this edge type is 3
    let input = r##"
<rect xy="0" wh="5" id="a" />
<rect xy="0 10" wh="5" id="b" />
<polyline start="#a@l" end="#b@l"/>"##;
    let expected_line = r#"<polyline points="0 2.5, -3 2.5, -3 12.5, 0 12.5"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_line);
}

#[test]
fn test_connector_u_rr() {
    // Loc l->l
    let input = r##"
<rect xy="0" wh="5" id="a" />
<rect xy="0 10" wh="5" id="b" />
<polyline start="#a@r" end="#b@r" corner-offset="2"/>"##;
    let expected_line = r#"<polyline points="5 2.5, 7 2.5, 7 12.5, 5 12.5"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_line);

    // default corner-offset for this edge type is 3
    let input = r##"
<rect xy="0" wh="5" id="a" />
<rect xy="0 10" wh="5" id="b" />
<polyline start="#a@r" end="#b@r"/>"##;
    let expected_line = r#"<polyline points="5 2.5, 8 2.5, 8 12.5, 5 12.5"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_line);
}

#[test]
fn test_connector_offset() {
    let input = format!(r##"{RECT_SVG}<polyline start="#a@b" end="#d@t" corner-offset="2" />"##);
    let expected_line = r#"<polyline points="2.5 5, 2.5 7, 22.5 7, 22.5 20"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_line);

    let input = format!(r##"{RECT_SVG}<polyline start="#a@b" end="#d@t" corner-offset="75%" />"##);
    let expected_line = r#"<polyline points="2.5 5, 2.5 16.25, 22.5 16.25, 22.5 20"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_line);
}

/// Check shapes can be positioned relative to a connector
#[test]
fn test_connector_relpos() {
    let input = format!(
        r##"{RECT_SVG}<line id="conn1" start="#a@b" end="#c@t"/><rect id="x" xy="#conn1" wh="1"/>"##
    );
    let expected_rect = r#"<rect id="x" x="2.5" y="5" width="1" height="1"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_rect);

    let input = format!(
        r##"{RECT_SVG}<line id="conn1" start="#a@b" end="#c@t"/><rect id="x" xy="#conn1@b" xy-loc="t" wh="1"/>"##
    );
    let expected_rect = r#"<rect id="x" x="2" y="20" width="1" height="1"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_rect);
}

#[test]
fn test_connector_reuse() {
    let input = r##"
<specs>
  <rect id="tt" wh="10"/>
</specs>
<reuse id="a" href="#tt" x="10" y="0"/>
<reuse id="b" href="#tt" x="10" y="30"/>
<line start="#a" end="#b"/>
"##;
    let expected = r#"<line x1="15" y1="10" x2="15" y2="30"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    // And with explicit locspecs
    let input = r##"
<specs>
  <rect id="tt" wh="10"/>
</specs>
<reuse id="a" href="#tt" x="10" y="0"/>
<reuse id="b" href="#tt" x="10" y="30"/>
<line start="#a@tl" end="#b@br"/>
"##;
    let expected = r#"<line x1="10" y1="0" x2="20" y2="40"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_connector_use() {
    // Same as test_connector_reuse, but with `<use>`
    let input = r##"
<defs>
  <rect id="tt" wh="10"/>
</defs>
<use id="a" href="#tt" x="10" y="0"/>
<use id="b" href="#tt" x="10" y="30"/>
<line start="#a" end="#b"/>
"##;
    let expected = r#"<line x1="15" y1="10" x2="15" y2="30"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    // And with explicit locspecs
    let input = r##"
<defs>
  <rect id="tt" wh="10"/>
</defs>
<use id="a" href="#tt" x="10" y="0"/>
<use id="b" href="#tt" x="10" y="30"/>
<line start="#a@tr" end="#b@bl"/>
"##;
    let expected = r#"<line x1="20" y1="0" x2="10" y2="40"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}
