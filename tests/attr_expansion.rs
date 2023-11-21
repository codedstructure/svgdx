pub mod utils;
use utils::compare;

#[test]
fn test_expand_rect_xy_wh() {
    let input = r#"<rect xy="1 2" wh="3 4"/>"#;
    let expected = r#"<rect x="1" y="2" width="3" height="4"/>"#;

    compare(input, expected);
}

#[test]
fn test_expand_xy1_xy2() {
    let input = r#"<line xy1="1 2" xy2="3 4"/>"#;
    let expected = r#"<line x1="1" y1="2" x2="3" y2="4"/>"#;

    compare(input, expected);
}

#[test]
fn test_expand_rect_cxy_wh() {
    let input = r#"<rect cxy="5 7" wh="3 4"/>"#;
    let expected = r#"<rect x="3.5" y="5" width="3" height="4"/>"#;

    compare(input, expected);
}

#[test]
fn test_expand_rect_xy_loc() {
    let input = r#"<rect xy="5 7" wh="3 4" xy-loc="br"/>"#;
    let expected = r#"<rect x="2" y="3" width="3" height="4"/>"#;
    compare(input, expected);

    let input = r#"<rect xy="5 7" wh="3 4" xy-loc="t"/>"#;
    let expected = r#"<rect x="3.5" y="7" width="3" height="4"/>"#;
    compare(input, expected);

    let input = r#"<rect xy="5 7" wh="4 6" xy-loc="c"/>"#;
    let expected = r#"<rect x="3" y="4" width="4" height="6"/>"#;
    compare(input, expected);
}

#[test]
fn test_expand_cycle() {
    let input = r#"<rect xy="5.5" wh="2"/>"#;
    let expected = r#"<rect x="5.5" y="5.5" width="2" height="2"/>"#;
    compare(input, expected);
}
