use crate::utils::compare;

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

#[test]
fn test_text_multiline() {
    let input = r#"
<rect xy="0" wh="10" text="multi\nline"/>
"#;
    let expected = r#"
<rect x="0" y="0" width="10" height="10"/>
<text x="5" y="5" class="tbox">
<tspan x="5" dy="-0.525em">multi</tspan><tspan x="5" dy="1.05em">line</tspan>
</text>
"#;
    compare(input, expected);

    let input = r#"
<rect xy="0" wh="10" text-loc="t" text-lsp="2" text="multi\nline"/>
"#;
    let expected = r#"
<rect x="0" y="0" width="10" height="10"/>
<text x="5" y="1" class="tbox text-top">
<tspan x="5" dy="0em">multi</tspan><tspan x="5" dy="2em">line</tspan>
</text>
"#;
    compare(input, expected);

    let input = r#"
<rect xy="0" wh="10" text-loc="br" text-lsp="1" text="multi\nline"/>
"#;
    let expected = r#"
<rect x="0" y="0" width="10" height="10"/>
<text x="9" y="9" class="tbox text-bottom text-right">
<tspan x="9" dy="-1em">multi</tspan><tspan x="9" dy="1em">line</tspan>
</text>
"#;
    compare(input, expected);

    // test adjacent empty lines
    let input = r#"
<rect xy="0" wh="10" text-lsp="1" text="multi\n\n\nline"/>
"#;
    let expected = r#"
<rect x="0" y="0" width="10" height="10"/>
<text x="5" y="5" class="tbox">
<tspan x="5" dy="-1.5em">multi</tspan><tspan x="5" dy="1em"></tspan><tspan x="5" dy="1em"></tspan><tspan x="5" dy="1em">line</tspan>
</text>
"#;
    compare(input, expected);
}

#[test]
fn test_text_dxy() {
    let input = r#"
<rect xy="0" wh="10" text-dx="2" text="blob"/>
"#;
    let expected = r#"
<rect x="0" y="0" width="10" height="10"/>
<text x="7" y="5" class="tbox">blob</text>
"#;
    compare(input, expected);

    let input = r#"
<rect xy="0" wh="10" text-dy="-2" text="blob"/>
"#;
    let expected = r#"
<rect x="0" y="0" width="10" height="10"/>
<text x="5" y="3" class="tbox">blob</text>
"#;
    compare(input, expected);

    let input = r#"
<rect xy="0" wh="10" text-dxy="1.5 3" text="blob"/>
"#;
    let expected = r#"
<rect x="0" y="0" width="10" height="10"/>
<text x="6.5" y="8" class="tbox">blob</text>
"#;
    compare(input, expected);
}

#[test]
fn test_text_line() {
    let input = r#"
<line xy1="0" xy2="10 0" text="blob"/>
"#;
    // TODO: at least for horizontal(ish?) lines like this,
    // would it be better if default was 'above' the line?
    let expected = r#"
<line x1="0" y1="0" x2="10" y2="0"/>
<text x="5" y="0" class="tbox">blob</text>
"#;
    compare(input, expected);

    let input = r#"
<line xy1="0" xy2="10 0" text="blob" text-loc="r"/>
"#;
    let expected = r#"
<line x1="0" y1="0" x2="10" y2="0"/>
<text x="11" y="0" class="tbox text-left">blob</text>
"#;
    compare(input, expected);
}
