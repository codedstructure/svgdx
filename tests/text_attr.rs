pub mod utils;
use utils::compare;

#[test]
fn test_basic_rect_text() {
    let input = r#"
<rect x="0" y="1" width="5" height="4" text="thing"/>
"#;
    let expected = r#"
<rect x="0" y="1" width="5" height="4"/>
<text x="2.5" y="3" class="tbox">thing</text>
"#;

    compare(input, expected);
}

#[test]
fn test_expanded_rect_text() {
    let input = r#"
<rect cxy="20" wh="20" text="thing"/>
"#;
    let expected = r#"
<rect x="10" y="10" width="20" height="20"/>
<text x="20" y="20" class="tbox">thing</text>
"#;

    compare(input, expected);
}

#[test]
fn test_text_loc() {
    let input = r#"
<rect cxy="20" wh="20" text="thing" text-loc="t"/>
"#;
    let expected = r#"
<rect x="10" y="10" width="20" height="20"/>
<text x="20" y="11" class="tbox text-top">thing</text>
"#;

    compare(input, expected);

    let input = r#"
<rect cxy="20" wh="20" text="thing" text-loc="bl"/>
"#;
    let expected = r#"
<rect x="10" y="10" width="20" height="20"/>
<text x="11" y="29" class="tbox text-bottom text-left">thing</text>
"#;

    compare(input, expected);
}
