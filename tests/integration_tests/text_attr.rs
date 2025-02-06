use svgdx::transform_str_default;

#[test]
fn test_basic_rect_text() {
    let input = r#"
<rect x="0" y="1" width="5" height="4" text="thing"/>
"#;
    let expected = r#"
<rect x="0" y="1" width="5" height="4"/>
<text x="2.5" y="3" class="d-text">thing</text>
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
<text x="20" y="20" class="d-text">thing</text>
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
<text x="20" y="11" class="d-text d-text-top">thing</text>
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
<text x="11" y="29" class="d-text d-text-bottom d-text-left">thing</text>
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
<text x="5" y="5" class="d-text">
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
<text x="5" y="1" class="d-text d-text-top">
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
<text x="9" y="9" class="d-text d-text-bottom d-text-right">
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
<text x="5" y="5" class="d-text">
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
<rect x="0" y="0" width="10" height="50"/>
<text x="5" y="25" writing-mode="tb" class="d-text d-text-vertical">
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
<text x="7" y="5" class="d-text">blob</text>
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
<text x="5" y="3" class="d-text">blob</text>
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
<text x="6.5" y="8" class="d-text">blob</text>
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
<text x="5" y="0" class="d-text">blob</text>
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
<text x="11" y="0" class="d-text d-text-left">blob</text>
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
<text x="5" y="5" class="d-text">some text</text>
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
<text x="5" y="5" class="d-text">
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
<text x="5" y="5" class="d-text">some text</text>
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
<text x="5" y="5" class="d-text">
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
    // to XML whitespace rules. The `d-text-pre` class converts whitespace to
    // non-breaking spaces to preserve formatting.
    let input = r#"
<rect xy="0" wh="10" class="d-text-pre">
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
<text x="5" y="5" class="d-text d-text-pre">
<tspan x="5" dy="-1.05em">Z</tspan><tspan x="5" dy="1.05em">NNNNdefNstart():</tspan><tspan x="5" dy="1.05em">NNNNNNNNprint(&quot;HelloNWorld!&quot;)</tspan>
</text>
"#;
    let expected = expected.replace('N', "\u{a0}").replace('Z', "\u{200b}");
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_text_offset() {
    let input = r#"
<rect cxy="20" wh="20" text="thing" text-loc="t" text-offset="3"/>
"#;
    let expected = r#"
<rect x="10" y="10" width="20" height="20"/>
<text x="20" y="13" class="d-text d-text-top">thing</text>
"#;

    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    let input = r#"
<rect cxy="20" wh="20" text="thing" text-loc="bl" text-offset="3"/>
"#;
    let expected = r#"
<rect x="10" y="10" width="20" height="20"/>
<text x="13" y="27" class="d-text d-text-bottom d-text-left">thing</text>
"#;

    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_text_inset_dxy() {
    // text-dxy should be applied after text-offset (which defaults to 1)
    let input = r#"
<rect cxy="20" wh="20" text="thing" text-loc="t" text-dx="1" text-dy="2"/>
"#;
    let expected = r#"
<rect x="10" y="10" width="20" height="20"/>
<text x="21" y="13" class="d-text d-text-top">thing</text>
"#;

    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    let input = r#"
<rect cxy="20" wh="20" text="thing" text-loc="t" text-offset="3" text-dxy="0.5"/>
"#;
    let expected = r#"
<rect x="10" y="10" width="20" height="20"/>
<text x="20.5" y="13.5" class="d-text d-text-top">thing</text>
"#;

    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    let input = r#"
<rect cxy="20" wh="20" text="thing" text-loc="tr" text-offset="3" text-dxy="-0.5"/>
"#;
    let expected = r#"
<rect x="10" y="10" width="20" height="20"/>
<text x="26.5" y="12.5" class="d-text d-text-top d-text-right">thing</text>
"#;

    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_text_style() {
    let input = r#"
<rect xy="0" wh="10" text="thing" text-style="font-size: 2em; font-weight: bold;"/>
"#;
    let expected = r#"
<rect x="0" y="0" width="10" height="10"/>
<text x="5" y="5" style="font-size: 2em; font-weight: bold;" class="d-text">thing</text>
"#;

    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    // Multi-line - check tspan gets the style
    let input = r#"
<rect xy="0" wh="10" text="two\nlines" text-style="font-size: 2em;"/>
"#;
    let expected = r#"
<rect x="0" y="0" width="10" height="10"/>
<text x="5" y="5" style="font-size: 2em;" class="d-text">
<tspan x="5" style="font-size: 2em;" dy="-0.525em">two</tspan><tspan x="5" style="font-size: 2em;" dy="1.05em">lines</tspan>
</text>
"#;

    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_text_element() {
    let input = r#"
<text xy="0" text="thing"/>
"#;
    let expected = r#"
<text x="0" y="0" class="d-text">thing</text>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    let input = r#"
<text xy="0">thing</text>
"#;
    let expected = r#"
<text x="0" y="0" class="d-text">thing</text>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    let input = r##"
<rect id="z" xy="0" wh="10"/>
<text xy="#z@c">thing</text>
"##;
    let expected = r#"
<rect id="z" x="0" y="0" width="10" height="10"/>
<text x="5" y="5" class="d-text">thing</text>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    let input = r##"
<rect id="z" xy="0" wh="10"/>
<text xy="#z@c" text="thing"/>
"##;
    let expected = r#"
<rect id="z" x="0" y="0" width="10" height="10"/>
<text x="5" y="5" class="d-text">thing</text>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_text_anchor() {
    let input = r##"
<rect id="z" xy="0" wh="10"/>
<text xy="#z|h 3" text="thing"/>
"##;
    let expected = r#"
<rect id="z" x="0" y="0" width="10" height="10"/>
<text x="14" y="5" class="d-text d-text-left">thing</text>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    let input = r##"
<rect id="z" xy="0" wh="10"/>
<text xy="#z@bl" text-offset="2" text="thing"/>
"##;
    let expected = r#"
<rect id="z" x="0" y="0" width="10" height="10"/>
<text x="-2" y="12" class="d-text d-text-top d-text-right">thing</text>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    let input = r##"
<rect id="z" xy="0" wh="10"/>
<text xy="#z@bl" text-offset="2" class="d-text-inside" text="thing"/>
"##;
    let expected = r#"
<rect id="z" x="0" y="0" width="10" height="10"/>
<text x="2" y="8" class="d-text d-text-bottom d-text-left">thing</text>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    let input = r##"
<rect id="z" xy="0" wh="10" text="thing" text-loc="r" class="d-text-outside"/>
"##;
    let expected = r#"
<rect id="z" x="0" y="0" width="10" height="10"/>
<text x="11" y="5" class="d-text d-text-left">thing</text>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_text_element_anchor() {
    // Key thing: xy doesn't have a locspec => text should be centered
    let input = r##"
<rect id="z" xy="0" wh="10"/>
<text xy="#z" text="thing"/>
"##;
    let expected = r#"
<rect id="z" x="0" y="0" width="10" height="10"/>
<text x="5" y="5" class="d-text">thing</text>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    // No locspec on xy, but text-loc is set (outside by default)
    let input = r##"
<rect id="z" xy="0" wh="10"/>
<text xy="#z" text-loc="r" text="thing"/>
"##;
    let expected = r#"
<rect id="z" x="0" y="0" width="10" height="10"/>
<text x="11" y="5" class="d-text d-text-left">thing</text>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    // No locspec on xy, text-loc set, inside flag set
    let input = r##"
<rect id="z" xy="0" wh="10"/>
<text xy="#z" text-loc="r" text="thing" class="d-text-inside"/>
"##;
    let expected = r#"
<rect id="z" x="0" y="0" width="10" height="10"/>
<text x="9" y="5" class="d-text d-text-right">thing</text>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_text_element_attrs() {
    let input1 = r#"
<text xy="0" text="thing" font-size="2em" font-weight="bold"/>
"#;
    let input2 = r#"
<text xy="0" font-size="2em" font-weight="bold">thing</text>
"#;
    let expected = r#"
<text x="0" y="0" font-size="2em" font-weight="bold" class="d-text">thing</text>
"#;
    assert_eq!(
        transform_str_default(input1).unwrap().trim(),
        expected.trim()
    );
    assert_eq!(
        transform_str_default(input2).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_multiline_outside() {
    let input = r#"
<rect xy="0" wh="10" text="multi\nline" text-loc="br" class="d-text-outside"/>
"#;
    let expected = r#"
<rect x="0" y="0" width="10" height="10"/>
<text x="11" y="11" class="d-text d-text-top d-text-left">
<tspan x="11" dy="0em">multi</tspan><tspan x="11" dy="1.05em">line</tspan>
</text>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    let input = r#"
<rect xy="0" wh="10"/>
<text xy="^@tl">multi\nline</text>
"#;
    let expected = r#"
<rect x="0" y="0" width="10" height="10"/>
<text x="-1" y="-1" class="d-text d-text-bottom d-text-right">
<tspan x="-1" dy="-1.05em">multi</tspan><tspan x="-1" dy="1.05em">line</tspan>
</text>
"#;
    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}
