use svgdx::transform_str_default;

#[test]
fn test_basic_rect_text() {
    let input = r#"
<rect x="0" y="1" width="5" height="4" text="thing"/>
"#;
    let expected = r#"
<rect x="0" y="1" width="5" height="4"/>
<text x="2.5" y="3" class="d-tbox">thing</text>
"#;

    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_expanded_rect_text() {
    let input = r#"
<rect cxy="20" wh="20" text="thing"/>
"#;
    let expected = r#"
<rect x="10" y="10" width="20" height="20"/>
<text x="20" y="20" class="d-tbox">thing</text>
"#;

    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_text_loc() {
    let input = r#"
<rect cxy="20" wh="20" text="thing" text-loc="t"/>
"#;
    let expected = r#"
<rect x="10" y="10" width="20" height="20"/>
<text x="20" y="11" class="d-tbox d-text-top">thing</text>
"#;

    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    let input = r#"
<rect cxy="20" wh="20" text="thing" text-loc="bl"/>
"#;
    let expected = r#"
<rect x="10" y="10" width="20" height="20"/>
<text x="11" y="29" class="d-tbox d-text-bottom d-text-left">thing</text>
"#;

    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_text_multiline() {
    let input = r#"
<rect xy="0" wh="10" text="multi\nline"/>
"#;
    let expected = r#"
<rect x="0" y="0" width="10" height="10"/>
<text x="5" y="5" class="d-tbox">
<tspan x="5" dy="-0.525em">multi</tspan><tspan x="5" dy="1.05em">line</tspan>
</text>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    let input = r#"
<rect xy="0" wh="10" text-loc="t" text-lsp="2" text="multi\nline"/>
"#;
    let expected = r#"
<rect x="0" y="0" width="10" height="10"/>
<text x="5" y="1" class="d-tbox d-text-top">
<tspan x="5" dy="0em">multi</tspan><tspan x="5" dy="2em">line</tspan>
</text>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    let input = r#"
<rect xy="0" wh="10" text-loc="br" text-lsp="1" text="multi\nline"/>
"#;
    let expected = r#"
<rect x="0" y="0" width="10" height="10"/>
<text x="9" y="9" class="d-tbox d-text-bottom d-text-right">
<tspan x="9" dy="-1em">multi</tspan><tspan x="9" dy="1em">line</tspan>
</text>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    // test adjacent empty lines
    let input = r#"
<rect xy="0" wh="10" text-lsp="1" text="multi\n\n\nline"/>
"#;
    // Slightly hacky, we insert zero-width spaces in empty lines
    let expected = format!(
        "{}\u{200b}{}\u{200b}{}",
        r#"
<rect x="0" y="0" width="10" height="10"/>
<text x="5" y="5" class="d-tbox">
<tspan x="5" dy="-1.5em">multi</tspan><tspan x="5" dy="1em">"#,
        r#"</tspan><tspan x="5" dy="1em">"#,
        r#"</tspan><tspan x="5" dy="1em">line</tspan>
</text>
"#
    );
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_text_vertical() {
    let input = r#"
<rect xy="0" wh="10 50" text="The Rust\nProgramming Language" class="d-text-vertical"/>
"#;
    let expected = r#"
<rect x="0" y="0" width="10" height="50" class="d-text-vertical"/>
<text x="5" y="25" writing-mode="tb" class="d-text-vertical d-tbox">
<tspan y="25" dx="-0.525em">Programming Language</tspan><tspan y="25" dx="1.05em">The Rust</tspan>
</text>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_text_dxy() {
    let input = r#"
<rect xy="0" wh="10" text-dx="2" text="blob"/>
"#;
    let expected = r#"
<rect x="0" y="0" width="10" height="10"/>
<text x="7" y="5" class="d-tbox">blob</text>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    let input = r#"
<rect xy="0" wh="10" text-dy="-2" text="blob"/>
"#;
    let expected = r#"
<rect x="0" y="0" width="10" height="10"/>
<text x="5" y="3" class="d-tbox">blob</text>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    let input = r#"
<rect xy="0" wh="10" text-dxy="1.5 3" text="blob"/>
"#;
    let expected = r#"
<rect x="0" y="0" width="10" height="10"/>
<text x="6.5" y="8" class="d-tbox">blob</text>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
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
<text x="5" y="0" class="d-tbox">blob</text>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    let input = r#"
<line xy1="0" xy2="10 0" text="blob" text-loc="r"/>
"#;
    let expected = r#"
<line x1="0" y1="0" x2="10" y2="0"/>
<text x="11" y="0" class="d-tbox d-text-left">blob</text>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_text_content() {
    let input = r#"
<rect xy="0" wh="10">some text</rect>
"#;
    let expected = r#"
<rect x="0" y="0" width="10" height="10"/>
<text x="5" y="5" class="d-tbox">some text</text>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    let input = r#"
<rect xy="0" wh="10">multi-line
text</rect>
"#;
    let expected = r#"
<rect x="0" y="0" width="10" height="10"/>
<text x="5" y="5" class="d-tbox">
<tspan x="5" dy="-0.525em">multi-line</tspan><tspan x="5" dy="1.05em">text</tspan>
</text>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_text_cdata() {
    let input = r#"
<rect xy="0" wh="10"><![CDATA[some text]]></rect>
"#;
    let expected = r#"
<rect x="0" y="0" width="10" height="10"/>
<text x="5" y="5" class="d-tbox">some text</text>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    let input = r#"
<rect xy="0" wh="10">
<![CDATA[
    def start():
        print("Hello World!")
]]>
</rect>
"#;

    // In this test we replace 'Z' with a zero-width space for easier string authoring...
    let expected = r#"
<rect x="0" y="0" width="10" height="10"/>
<text x="5" y="5" class="d-tbox">
<tspan x="5" dy="-1.05em">Z</tspan><tspan x="5" dy="1.05em">    def start():</tspan><tspan x="5" dy="1.05em">        print(&quot;Hello World!&quot;)</tspan>
</text>
"#;
    let expected = expected.replace('Z', "\u{200b}");
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_text_cdata_pre() {
    // While conversion preserves whitespace, the rendering of SVG does not, due
    // to XML whitespace rules. The `text-pre` attribute converts whitespace to
    // non-breaking spaces to preserve formatting.
    let input = r#"
<rect xy="0" wh="10" text-pre="true">
<![CDATA[
    def start():
        print("Hello World!")
]]>
</rect>
"#;

    // In this test we replace 'N' and 'Z' for non-breaking and zero-width spaces
    // repectively, for easier string authoring...
    let expected = r#"
<rect x="0" y="0" width="10" height="10"/>
<text x="5" y="5" class="d-tbox">
<tspan x="5" dy="-1.05em">Z</tspan><tspan x="5" dy="1.05em">NNNNdefNstart():</tspan><tspan x="5" dy="1.05em">NNNNNNNNprint(&quot;HelloNWorld!&quot;)</tspan>
</text>
"#;
    let expected = expected.replace('N', "\u{a0}").replace('Z', "\u{200b}");
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}
