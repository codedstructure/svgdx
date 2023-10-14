use std::io::Cursor;
use svgd::Transformer;

fn compare(input: &str, expected: &str) {
    let mut t = Transformer::new();
    let mut output: Vec<u8> = vec![];

    let mut input = Cursor::new(input);
    t.transform(&mut input, &mut output);
    let output = String::from_utf8(output).unwrap();

    assert_eq!(output, expected);
}

#[test]
fn test_expand_rect_xy_wh() {
    let input = r#"<svg><rect xy="1 2" wh="3 4"/></svg>"#;
    let expected = r#"<svg><rect x="1" y="2" width="3" height="4"/></svg>"#;

    compare(input, expected);
}

#[test]
fn test_expand_xy1_xy2() {
    let input = r#"<svg><line xy1="1 2" xy2="3 4"/></svg>"#;
    let expected = r#"<svg><line x1="1" y1="2" x2="3" y2="4"/></svg>"#;

    compare(input, expected);
}

#[test]
fn test_expand_rect_xy1_wh() {
    let input = r#"<svg><rect xy1="1 2" wh="3 6"/></svg>"#;
    let expected = r#"<svg><rect x="1" y="2" width="3" height="6"/></svg>"#;

    compare(input, expected);
}

#[test]
fn test_expand_rect_xy1_xy2() {
    let input = r#"<svg><rect xy1="1 2" xy2="3 6"/></svg>"#;
    let expected = r#"<svg><rect x="1" y="2" width="2" height="4"/></svg>"#;

    compare(input, expected);
}

#[test]
fn test_expand_rect_wh_xy2() {
    let input = r#"<svg><rect xy2="4 6" wh="2 1"/></svg>"#;
    let expected = r#"<svg><rect x="2" y="5" width="2" height="1"/></svg>"#;

    compare(input, expected);
}

#[test]
fn test_expand_rect_cxy_wh() {
    let input = r#"<svg><rect cxy="5 7" wh="3 4"/></svg>"#;
    let expected = r#"<svg><rect x="3.5" y="5" width="3" height="4"/></svg>"#;

    compare(input, expected);
}
