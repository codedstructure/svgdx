use assertables::{assert_contains, assert_contains_as_result};
use svgdx::transform_str_default;

#[test]
fn test_reuse_simple() {
    let input = r##"
<specs>
 <rect id="target" xy="0" wh="1 2"/>
</specs>
<reuse href="#target"/>
"##;
    let expected = r#"
<rect x="0" y="0" width="1" height="2" class="target"/>
"#;
    let output = transform_str_default(input).unwrap();
    // exact equality check: ensure that <specs> doesn't appear in the output.
    assert_eq!(output, expected);
}

#[test]
fn test_reuse_attr_locals() {
    let input = r##"
<specs>
 <rect id="square" width="$size" height="$size"/>
</specs>
<reuse href="#square" size="10" x="3" y="4"/>
"##;
    let expected = r#"<rect width="10" height="10" transform="translate(3, 4)" class="square"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r##"
<specs>
  <rect id="square" rx="$rx" width="$size" height="$size"/>
</specs>
<reuse id="base" href="#square" rx="2" size="10" class="thing"/>
"##;
    let expected = r#"<rect id="base" width="10" height="10" rx="2" class="thing square"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_reuse_group() {
    let input = r##"
<specs>
<g id="a">
<rect xy="0" wh="{{10 + $h}} $h" text="$h" text-loc="bl"/>
<circle cx="0" cy="$h" r="0.5"/>
</g>
</specs>
<reuse id="first" href="#a" h="40"/>
<reuse href="#a" h="30"/>
<reuse id="third" href="#a" h="20" class="test-class"/>
"##;
    let expected = r#"
<g id="first" class="a">
<rect x="0" y="0" width="50" height="40"/>
<text x="1" y="39" class="d-tbox d-text-bottom d-text-left">40</text>
<circle cx="0" cy="40" r="0.5"/>
</g>
<g class="a">
<rect x="0" y="0" width="40" height="30"/>
<text x="1" y="29" class="d-tbox d-text-bottom d-text-left">30</text>
<circle cx="0" cy="30" r="0.5"/>
</g>
<g id="third" class="test-class a">
<rect x="0" y="0" width="30" height="20"/>
<text x="1" y="19" class="d-tbox d-text-bottom d-text-left">20</text>
<circle cx="0" cy="20" r="0.5"/>
</g>
"#;
    let output = transform_str_default(input).unwrap();
    // exact equality check: ensure that <specs> doesn't appear in the output.
    assert_eq!(output, expected);
}

#[test]
fn test_reuse_group_svg() {
    // At one point this failed because <reuse> remained on the element_stack
    // at the time '</svg>' was processed.
    let input = r##"
<svg>
  <specs>
    <g id="a"><rect xy="0" wh="10"/></g>
  </specs>
  <reuse id="b" href="#a"/>
</svg>
"##;
    assert!(transform_str_default(input).is_ok());
}

#[test]
fn test_reuse_xy_transform() {
    let input = r##"
<specs>
  <rect id="tb" wh="20 10"/>
</specs>
<reuse href="#tb" x="123"/>
"##;
    let output = transform_str_default(input).unwrap();
    let expected = r#"<rect width="20" height="10" transform="translate(123, 0)" class="tb"/>"#;

    assert_contains!(output, expected);

    let input = r##"
<specs>
  <rect id="tb" text="$text" wh="20 10" transform="translate(10)"/>
</specs>
<reuse href="#tb" text="thing" y="1" transform="translate(11)"/>
"##;
    let output = transform_str_default(input).unwrap();
    let expected1 = r#"<rect width="20" height="10" transform="translate(10) translate(11) translate(0, 1)" class="tb"/>"#;
    let expected2 = r#"<text x="10" y="5" transform="translate(10) translate(11) translate(0, 1)" class="tb d-tbox">thing</text>"#;

    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
}

#[test]
fn test_reuse_group_transform() {
    let input = r##"
<specs>
<g id="square">
<rect x="0" y="0" width="$size" height="$size"/>
</g>
</specs>
<reuse id="this" href="#square" x="3" y="5" size="10" transform="rotate(45)"/>
"##;
    let expected = r#"<g id="this" transform="rotate(45) translate(3, 5)" class="square">"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r##"
<specs>
<g id="square">
<rect x="0" y="0" width="$size" height="$size"/>
</g>
</specs>
<reuse id="this" href="#square" y="5" size="10"/>
"##;
    let expected = r#"<g id="this" transform="translate(0, 5)" class="square">"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r##"
<specs>
<g id="square">
<rect x="0" y="0" width="$size" height="$size"/>
</g>
</specs>
<reuse id="this" href="#square" size="10"/>
"##;
    let expected = r#"<g id="this" class="square">"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_reuse_symbol() {
    let input = r##"
<defs>
  <symbol id="sym"><circle r="1"/></symbol>
</defs>
<reuse href="#sym"/>
  "##;
    let expected = r#"<g class="sym"><circle r="1"/></g>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_reuse_recursive() {
    let input = r##"
<specs>
<g id="a"><rect xy="0" wh="5" text="$t"/></g>
<reuse id="b" href="#a" t="2"/>
<reuse id="c" href="#b" t="3"/>
<reuse id="d" href="#c" t="4"/>
</specs>
<reuse href="#d" t="5"/>
"##;
    let expected = r#"<g class="d c b a"><rect x="0" y="0" width="5" height="5"/>
<text x="2.5" y="2.5" class="d-tbox">5</text></g>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}
