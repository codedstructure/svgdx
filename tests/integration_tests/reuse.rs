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
 <rect id="square" width="$size" height="$size" xy="$x $y"/>
</specs>
<reuse href="#square" size="10" x="3" y="4"/>
"##;
    let expected = r#"<rect x="3" y="4" width="10" height="10" class="square"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r##"
<specs>
  <rect id="square" x="$x" y="$y" width="$size" height="$size"/>
</specs>
<reuse id="base" href="#square" x="0" y="0" size="10" class="thing"/>
"##;
    let expected = r#"<rect id="base" x="0" y="0" width="10" height="10" class="thing square"/>"#;
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
fn test_reuse_transform() {
    // TODO: once x/y/transform are implemented for non-<g> elements, test those too.
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
