use assertables::{assert_contains, assert_not_contains};
use svgdx::transform_str_default;

#[test]
fn test_point_simple() {
    let input = r##"
<point id="p0" xy="0"/>
<point id="p1" xy="10"/>
<line start="#p0" end="#p1"/>
"##;
    let expected = r#"<line x1="0" y1="0" x2="10" y2="10"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
    assert_not_contains!(output, "<point");
}

#[test]
fn test_point_polyline() {
    let input = r##"
<point id="p0" xy="0"/>
<point id="p1" xy="10"/>
<point id="p2" xy="10 0"/>
<polyline points="#p0 #p1 #p2"/>
"##;
    let expected = r#"<polyline points="0 0 10 10 10 0"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_point_refspec() {
    let input = r##"
<rect id="r0" wh="10"/>
<point id="p0" xy="#r0@l"/>
<point id="p1" xy="#r0@r"/>
<line start="#p0" end="#p1"/>
"##;
    let expected = r#"<line x1="0" y1="5" x2="10" y2="5"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r##"
<rect id="r0" wh="10"/>
<point id="p0" xy="#r0@t:25%"/>
<point id="p1" xy="#r0@b:75%"/>
<line start="#p0" end="#p1"/>
"##;
    let expected = r#"<line x1="2.5" y1="0" x2="7.5" y2="10"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_point_ignored_at_toplevel() {
    let input = r##"
<svg>
<config border="0"/>
<point id="p0" xy="#r0@l"/>
<point id="p1" xy="#r0@r"/>
<rect id="r0" wh="10"/>
<point id="p2" xy="900" _="This should not expand the SVG viewBox"/>
<line start="#p0" end="#p1"/>
</svg>
"##;
    let expected = r#"viewBox="0 0 10 10"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}
