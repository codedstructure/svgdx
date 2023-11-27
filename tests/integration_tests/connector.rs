use crate::utils::contains;

const RECT_SVG: &str = r#"
<rect x="0" y="0" width="5" height="5" id="a" />
<rect x="20" y="0" width="5" height="5" id="b" />
<rect x="0" y="20" width="5" height="5" id="c" />
<rect x="20" y="20" width="5" height="5" id="d" />
"#;

#[test]
fn test_connector() {
    let input = format!(r##"{}<polyline start="#a@b" end="#d@t" />"##, RECT_SVG);
    let expected_line = r#"<polyline points="2.5 5, 2.5 12.5, 22.5 12.5, 22.5 20"/>"#;
    contains(&input, expected_line);

    let input = format!(r##"{}<polyline start="#a@r" end="#d@t" />"##, RECT_SVG);
    let expected_line = r#"<polyline points="5 2.5, 22.5 2.5, 22.5 20"/>"#;
    contains(&input, expected_line);
}

#[test]
fn test_connector_fixed_start() {
    let input = format!(r##"{}<line start="3 7" end="#d@t" />"##, RECT_SVG);
    let expected_line = r#"<line x1="3" y1="7" x2="22.5" y2="20"/>"#;
    contains(&input, expected_line);
}

#[test]
fn test_connector_fixed_end() {
    let input = format!(r##"{}<line start="#a@r" end="10 17" />"##, RECT_SVG);
    let expected_line = r#"<line x1="5" y1="2.5" x2="10" y2="17"/>"#;
    contains(&input, expected_line);
}

#[test]
fn test_connector_h() {
    let input = r##"
<rect x="0" y="0" width="5" height="5" id="a" />
<rect x="20" y="2" width="5" height="5" id="b" />
<line start="#a" end="#b" edge-type="h"/>"##;
    let expected_line = r#"<line x1="5" y1="3.5" x2="20" y2="3.5"/>"#;
    contains(&input, expected_line);
}

#[test]
fn test_connector_v() {
    let input = r##"
<rect x="0" y="0" width="5" height="5" id="a" />
<rect x="1" y="20" width="5" height="5" id="b" />
<line start="#a" end="#b" edge-type="v"/>"##;
    let expected_line = r#"<line x1="3" y1="5" x2="3" y2="20"/>"#;
    contains(&input, expected_line);
}

#[test]
fn test_connector_offset() {
    let input = format!(
        r##"{}<polyline start="#a@b" end="#d@t" corner-offset="2" />"##,
        RECT_SVG
    );
    let expected_line = r#"<polyline points="2.5 5, 2.5 7, 22.5 7, 22.5 20"/>"#;
    contains(&input, expected_line);

    let input = format!(
        r##"{}<polyline start="#a@b" end="#d@t" corner-offset="75%" />"##,
        RECT_SVG
    );
    let expected_line = r#"<polyline points="2.5 5, 2.5 16.25, 22.5 16.25, 22.5 20"/>"#;
    contains(&input, expected_line);
}
