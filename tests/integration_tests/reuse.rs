use assertables::{assert_contains, assert_not_contains};
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
    // equality check: ensure that <specs> doesn't appear in the output.
    assert_eq!(output.trim(), expected.trim());
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
<text x="1" y="39" class="d-text d-text-bottom d-text-left">40</text>
<circle cx="0" cy="40" r="0.5"/>
</g>
<g class="a">
<rect x="0" y="0" width="40" height="30"/>
<text x="1" y="29" class="d-text d-text-bottom d-text-left">30</text>
<circle cx="0" cy="30" r="0.5"/>
</g>
<g id="third" class="test-class a">
<rect x="0" y="0" width="30" height="20"/>
<text x="1" y="19" class="d-text d-text-bottom d-text-left">20</text>
<circle cx="0" cy="20" r="0.5"/>
</g>
"#;
    let output = transform_str_default(input).unwrap();
    // exact equality check: ensure that <specs> doesn't appear in the output.
    assert_eq!(output.trim(), expected.trim());
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
    let expected2 = r#"<text x="31" y="6" class="d-text tb">thing</text>"#;

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
<text x="2.5" y="2.5" class="d-text">5</text></g>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output.trim(), expected.trim());
}

#[test]
fn test_reuse_attr_eval() {
    // Check reuse attributes are evaluated prior to instancing.
    let input = r##"
<specs>
<g id="a"><rect xy="0" wh="10" text="{{$target~w}}"/></g>
</specs>
<loop count="3" start="1" loop-var="ii">
  <rect id="r${ii}" height="2" width="{{$ii * 5}}"/>
  <reuse href="#a" target="#r${ii}"/>
</loop>
"##;
    let expected1 = r#">5</text>"#;
    let expected2 = r#">10</text>"#;
    let expected3 = r#">15</text>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
    assert_contains!(output, expected3);
}

#[test]
fn test_reuse_if() {
    // Check reuse target containing 'if' works
    let base_xml = r##"
<specs>
<g id="a">
 <if test="eq($sel, 1)"><text text="one"/></if>
 <if test="eq($sel, 2)"><text text="two"/></if>
</g>
</specs>
"##
    .to_string();
    let input = base_xml.clone() + r##"<reuse href="#a" sel="1"/>"##;
    let expected = r#">one</text>"#;
    let unexpected = r#">two</text>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
    assert_not_contains!(output, unexpected);

    let input = base_xml + r##"<reuse href="#a" sel="2"/>"##;
    let expected = r#">two</text>"#;
    let unexpected = r#">one</text>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
    assert_not_contains!(output, unexpected);
}

#[test]
fn test_use_circular() {
    assert!(transform_str_default(r##"<use id="a" href="#a"/>"##).is_err());

    assert!(transform_str_default(r##"<use id="a" href="#b"/><use id="b" href="#a"/>"##).is_err());

    // Should be fine; repeated identical _href_s with '^', but they reference different elements.
    // TODO: this doesn't work until we support closures properly; at present the
    //       `^` is evaluated in the context of the current element only, so is the
    //       same throughout the evaluation.
    //     let input = r##"
    // <rect id="a" wh="1"/>
    // <use id="b" href="#a"/>
    // <use id="c" href="^"/>
    // <use id="d" href="^"/>
    // "##;
    //     assert!(transform_str_default(input).is_ok());

    // Should _not_ be ok - circular reference
    let input = r##"
<rect id="a" wh="1"/>
<use id="b" href="#d"/>
<use id="c" href="^"/>
<use id="d" href="^"/>
"##;
    assert!(transform_str_default(input).is_err());
}

#[test]
fn test_reuse_circular() {
    assert!(transform_str_default(r##"<reuse id="a" href="#a"/>"##).is_err());

    assert!(
        transform_str_default(r##"<reuse id="a" href="#b"/><reuse id="b" href="#a"/>"##).is_err()
    );
}

#[test]
fn test_reuse_depth_limit() {
    let input_fn = |limit: u32| {
        format!(
            r##"
<config depth-limit="{limit}"/>
<rect id="a" wh="0"/>
<reuse id="b" href="#a"/>
<reuse id="c" href="#b"/>
<reuse id="d" href="#c"/>
"##
        )
    };

    let input = input_fn(4);
    assert!(transform_str_default(&input).is_ok());

    let input = input_fn(3);
    assert!(transform_str_default(&input).is_err());
}

#[test]
fn test_nesting_depth_limit() {
    let input_fn = |limit: u32| {
        format!(
            r##"
<config depth-limit="{limit}"/>
<g>
  <g>
    <g>
      <rect id="a" wh="0"/>
    </g>
  </g>
</g>
"##
        )
    };

    let input = input_fn(4);
    assert!(transform_str_default(&input).is_ok());

    let input = input_fn(3);
    assert!(transform_str_default(&input).is_err());
}

#[test]
fn test_reuse_group_rel() {
    let input = r##"
<svg>
<config border="0" add-auto-styles="false"/>
<g id="tt"><rect xy="5" wh="10"/></g>
<reuse id="a" href="#tt" x="10" y="15"/>
</svg>
"##;
    let expected1 = r##"<g id="a" transform="translate(10, 15)" class="tt"><rect x="5" y="5" width="10" height="10"/></g>"##;
    let expected2 = r##"viewBox="5 5 20 25"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);

    let input = r##"
<svg>
<config border="0" add-auto-styles="false"/>
<defs>
<g id="tt"><rect xy="5" wh="10"/></g>
</defs>
<reuse id="a" href="#tt" x="10" y="15"/>
</svg>
"##;
    let expected1 = r##"<g id="a" transform="translate(10, 15)" class="tt"><rect x="5" y="5" width="10" height="10"/></g>"##;
    let expected2 = r##"viewBox="15 20 10 10"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);

    // same without the surrounding <g> element
    let input = r##"
<svg>
<config border="0" add-auto-styles="false"/>
<defs>
<rect id="tt" xy="5" wh="10"/>
</defs>
<reuse id="a" href="#tt" x="10" y="15"/>
</svg>
"##;
    let expected1 = r##"<rect id="a" x="5" y="5" width="10" height="10" transform="translate(10, 15)" class="tt"/>"##;
    let expected2 = r##"viewBox="15 20 10 10"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
}

#[test]
fn test_use_bbox() {
    let input = r##"
<svg>
  <config border="0"/>
  <defs>
    <g id="a"><rect xy="0" wh="10"/></g>
  </defs>
  <use x="0" y="0" href="#a"/>
</svg>
"##;
    let expected = r#"viewBox="0 0 10 10""#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_reuse_bbox() {
    let input = r##"
<svg>
  <config border="0"/>
  <specs>
    <g id="a"><rect xy="0" wh="10"/></g>
  </specs>
  <reuse id="b" href="#a"/>
  <circle xy="#b|h" r="5"/>
</svg>
"##;
    let expected = r#"viewBox="0 0 20 10""#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_use_symbol_bbox() {
    let input = r##"
<svg>
  <config border="0"/>
  <symbol id="tt">
    <rect xy="0" wh="5"/>
    <rect xy="3" wh="5"/>
  </symbol>
  <use href="#tt" x="2" y="5"/>
</svg>
"##;
    let expected = r##"viewBox="2 5 8 8""##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_use_relpos() {
    let input = r##"
<defs>
  <rect id="abc" wh="3" xy="-5"/>
  <rect id="pqr" wh="7" xy="-3"/>
</defs>
<use id="u1" href="#abc"/>
<use id="u2" href="#pqr" xy="^|v"/>
"##;
    let expected = r##"<use id="u2" href="#pqr" x="-4" y="1"/>"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r##"
<defs>
 <g id="thing">
  <rect wh="5"/>
 </g>
</defs>
<use href="#thing"/>
<use xy="^|h 2" href="#thing"/>
"##;
    let expected1 = r##"<use href="#thing"/>"##;
    let expected2 = r##"<use href="#thing" x="7" y="0"/>"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
}

#[test]
fn test_reuse_prev() {
    let input = r##"
<rect wh="3" xy="0"/>
<reuse id="z" href="^" y="2"/>"##;
    let expected =
        r##"<rect id="z" x="0" y="0" width="3" height="3" transform="translate(0, 2)"/>"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_use_relspec() {
    // For the <use> case, the bbox of the target is derived,
    // then an adjustment is made to the x/y of the <use> element
    // to put it in the right locspec.
    let input = r##"
<rect id="a" wh="10 6"/>
<circle id="b" r="0.1"/>
<use id="z" href="#b" cxy="#a@c"/>
"##;
    let expected = r##"<use id="z" href="#b" x="5" y="3"/>"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r##"
<rect id="a" wh="10 6"/>
<rect id="b" wh="2" />
<use id="z" href="#b" cxy="#a@c"/>
"##;
    let expected = r##"<use id="z" href="#b" x="4" y="2"/>"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r##"
<defs>
 <g id="zz">
  <rect id="a" wh="5"/>
  <rect xy="^|h 5" wh="^"/>
  <line start="^@l:30%" end="#a@r:30%"/>
 </g>
</defs>
<circle id="base" xy="15 30" r="10"/>
<use cxy="#base 0 -2" href="#zz"/>
"##;
    let expected = r##"
<defs>
 <g id="zz">
  <rect id="a" width="5" height="5"/>
  <rect x="10" y="0" width="5" height="5"/>
  <line x1="10" y1="1.5" x2="5" y2="1.5"/>
 </g>
</defs>
<circle id="base" cx="25" cy="40" r="10"/>
<use href="#zz" x="17.5" y="35.5"/>
"##;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}
