use assertables::{assert_contains, assert_contains_as_result};
use svgdx::transform_str_default;

const RECT_SVG: &str = r#"
<rect x="0" y="0" width="5" height="5" id="a" />
<rect x="20" y="0" width="5" height="5" id="b" />
<rect x="0" y="20" width="5" height="5" id="c" />
<rect x="20" y="20" width="5" height="5" id="d" />
"#;

#[test]
fn test_basic_ref() {
    let input = format!(r##"{RECT_SVG}<line start="#a" end="#b" />"##);
    let expected_line = r#"<line x1="5" y1="2.5" x2="20" y2="2.5"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_line);
}

#[test]
fn test_closest_points() {
    let input = format!(r##"{RECT_SVG}<line start="#a" end="#b" />"##);
    let expected_line = r#"<line x1="5" y1="2.5" x2="20" y2="2.5"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_line);

    let input = format!(r##"{RECT_SVG}<line start="#b" end="#d" />"##);
    let expected_line = r#"<line x1="22.5" y1="5" x2="22.5" y2="20"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_line);

    let input = format!(r##"{RECT_SVG}<line start="#a" end="#d" />"##);
    let expected_line = r#"<line x1="5" y1="5" x2="20" y2="20"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_line);
}

#[test]
fn test_loc_select() {
    let input = format!(r##"{RECT_SVG}<line start="#a@tr" end="#b@bl" />"##);
    let expected_line = r#"<line x1="5" y1="0" x2="20" y2="5"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_line);

    let input = format!(r##"{RECT_SVG}<line start="#a@r" end="#b@c" />"##);
    let expected_line = r#"<line x1="5" y1="2.5" x2="22.5" y2="2.5"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_line);

    let input = format!(r##"{RECT_SVG}<line start="#a@tl" end="#d@br" />"##);
    let expected_line = r#"<line x1="0" y1="0" x2="25" y2="25"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_line);
}

/// Should support mixture of explicit and loc-based endpoints
#[test]
fn test_loc_mixed() {
    let input = format!(r##"{RECT_SVG}<line start="#a@tr" end="12 23" />"##);
    let expected_line = r#"<line x1="5" y1="0" x2="12" y2="23"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_line);

    let input = format!(r##"{RECT_SVG}<line start="3 7" end="#a" />"##);
    let expected_line = r#"<line x1="3" y1="7" x2="2.5" y2="5"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_line);
}

#[test]
fn test_loc_shape() {
    let input = format!(r##"{RECT_SVG}<circle cxy="#a@r" r="2" />"##);
    let expected_circle = r#"<circle cx="5" cy="2.5" r="2"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_circle);
}

#[test]
fn test_loc_shape_offset() {
    let input = format!(r##"{RECT_SVG}<circle cxy="#a@r 1.5 2.3" r="2" />"##);
    let expected_circle = r#"<circle cx="6.5" cy="4.8" r="2"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected_circle);
}
