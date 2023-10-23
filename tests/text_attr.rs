mod utils;
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
fn test_expand_rect_xy1_wh() {
    let input = r#"<rect xy1="1 2" wh="3 6"/>"#;
    let expected = r#"<rect x="1" y="2" width="3" height="6"/>"#;

    compare(input, expected);
}

#[test]
fn test_expand_rect_xy1_xy2() {
    let input = r#"<rect xy1="1 2" xy2="3 6"/>"#;
    let expected = r#"<rect x="1" y="2" width="2" height="4"/>"#;

    compare(input, expected);
}

#[test]
fn test_expand_rect_wh_xy2() {
    let input = r#"<rect xy2="4 6" wh="2 1"/>"#;
    let expected = r#"<rect x="2" y="5" width="2" height="1"/>"#;

    compare(input, expected);
}

#[test]
fn test_expand_rect_cxy_wh() {
    let input = r#"<rect cxy="5 7" wh="3 4"/>"#;
    let expected = r#"<rect x="3.5" y="5" width="3" height="4"/>"#;

    compare(input, expected);
}
