use assertables::assert_contains;
use svgdx::transform_str_default;

#[test]
fn test_scalarspec() {
    let input = r#"
  <rect id="a" wh="20"/>
  <text xy="^" text="{{#a~w}}"/>
"#;
    let expected = r#"
  <rect id="a" width="20" height="20"/>
  <text x="10" y="10" class="d-text">20</text>
"#;

    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    // As above but use '^' for scalar elref
    let input = r#"
  <rect wh="20"/>
  <text xy="^" text="{{^~w}}"/>
"#;
    let expected = r#"
  <rect width="20" height="20"/>
  <text x="10" y="10" class="d-text">20</text>
"#;

    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    // As above but use an id containing '-'
    let input = r#"
  <rect id="a-1" wh="20"/>
  <text xy="^" text="{{#a-1~w}}"/>
"#;
    let expected = r#"
  <rect id="a-1" width="20" height="20"/>
  <text x="10" y="10" class="d-text">20</text>
"#;

    assert_eq!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_scalarspec_forwardref() {
    let input = r##"
  <text id="z" text="{{(#a~x2 + #b~x1) / 2}}"/>
  <rect id="a" wh="20"/>
  <rect id="b" xy="#a|h 10" wh="20"/>
"##;
    let expected = r#"
  <text id="z" x="0" y="0" class="d-text">25</text>
"#;

    assert_contains!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_bbox_surround() {
    let input = r##"
  <rect id="a" x="10" y="20" width="30" height="40"/>
  <rect id="b" x="15" y="25" width="30" height="40"/>
  <text id="z" text="{{surround(#a, #b)}}"/>
"##;
    let expected = r#"<text id="z" x="0" y="0" class="d-text">10, 20, 45, 65</text>"#;
    assert_contains!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_bbox_inside() {
    // Partial overlap
    let input = r##"
  <rect id="a" x="10" y="20" width="30" height="40"/>
  <rect id="b" x="15" y="25" width="30" height="40"/>
  <text text="{{inside(#a, #b)}}"/>
"##;
    let expected = r#">15, 25, 40, 60</text>"#;
    assert_contains!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    // Complete overlap
    let input = r##"
  <rect id="a" x="10" y="10" width="20" height="20"/>
  <rect id="b" x="15" y="15" width="5" height="5"/>
  <text text="{{inside(#a, #b)}}"/>
"##;
    let expected = r#">15, 15, 20, 20</text>"#;
    assert_contains!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    // Overlap in one dimension only
    let input = r##"
  <rect id="a" x="10" y="10" width="20" height="20"/>
  <rect id="b" x="15" y="40" width="5" height="5"/>
  <text text="{{inside(#a, #b)}}"/>
"##;
    let expected = r#">15, 30, 20, 40</text>"#;
    assert_contains!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );

    // No overlap - check the 'absolute' overlap is returned
    let input = r##"
  <rect id="a" x="10" y="10" width="20" height="20"/>
  <rect id="b" x="40" y="40" width="5" height="5"/>
  <text text="{{inside(#a, #b)}}"/>
"##;
    let expected = r#">30, 30, 40, 40</text>"#;
    assert_contains!(
        transform_str_default(input).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn test_mid() {
    // midpoint of two values
    let input = r##"<text text="{{mid(10, 20)}}"/>"##;
    let expected = r#">15</text>"#;
    assert_contains!(transform_str_default(input).unwrap(), expected);

    // midpoint of two scalarspecs
    let input = r##"
  <rect id="a" wh="20"/>
  <rect id="b" xy="#a|h 10" wh="20"/>
  <text text="{{mid(#a~x2, #b~x1)}}"/>
"##;
    let expected = r#">25</text>"#;
    assert_contains!(transform_str_default(input).unwrap(), expected);

    // midpoint of two coordinates (locspecs)
    let input = r##"
  <rect id="a" wh="20"/>
  <rect id="b" xy="#a|h 10" wh="20"/>
  <text text="{{mid(#a@b, #b@b)}}"/>
"##;
    let expected = r#">25, 20</text>"#;
    assert_contains!(transform_str_default(input).unwrap(), expected);

    // midpoint of two elements
    let input = r##"
  <rect id="a" wh="20"/>
  <rect id="b" xy="#a|h 10" wh="20"/>
  <text text="{{mid(#a, #b)}}"/>
"##;
    let expected = r#">25, 10</text>"#;
    assert_contains!(transform_str_default(input).unwrap(), expected);
}

#[test]
fn test_bbox_loc() {
    let input = r##"
  <rect id="a" x="10" y="20" width="30" height="40"/>
  <text text="tl: {{loc('tl', #a)}}"/>
  <text text="tr: {{loc('tr', #a)}}"/>
  <text text="bl: {{loc('bl', #a)}}"/>
  <text text="t: {{loc('t', #a)}}"/>
"##;
    let expected = &["tl: 10, 20<", "tr: 40, 20<", "bl: 10, 60<", "t: 25, 20<"];
    let output = transform_str_default(input).unwrap();
    for exp in expected {
        assert_contains!(output, exp);
    }
}

#[test]
fn test_bbox_values() {
    let input = r##"
  <rect id="a" x="10" y="20" width="30" height="40"/>
  <text text="xy: {{xy(#a)}}"/>
  <text text="wh: {{wh(#a)}}"/>
  <text text="size: {{size(#a)}}"/>
  <text text="x1: {{x1(#a)}}"/>
  <text text="y1: {{y1(#a)}}"/>
  <text text="x2: {{x2(#a)}}"/>
  <text text="y2: {{y2(#a)}}"/>
  <text text="cx: {{cx(#a)}}"/>
  <text text="cy: {{cy(#a)}}"/>
  <text text="width: {{width(#a)}}"/>
  <text text="height: {{height(#a)}}"/>
"##;
    let expected = &[
        "xy: 10, 20<",
        "wh: 30, 40<",
        "size: 30, 40<",
        "x1: 10<",
        "y1: 20<",
        "x2: 40<",
        "y2: 60<",
        "cx: 25<",
        "cy: 40<",
        "width: 30<",
        "height: 40<",
    ];
    let output = transform_str_default(input).unwrap();
    for exp in expected {
        assert_contains!(output, exp);
    }
}
