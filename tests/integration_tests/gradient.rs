use assertables::assert_contains;
use svgdx::transform_str_default;

#[test]
fn test_gradient_empty() {
    let input = r#"<linearGradient id="grad"/>"#;
    let expected = r#"<linearGradient id="grad"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r#"<radialGradient id="grad"/>"#;
    let expected = r#"<radialGradient id="grad"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_gradient_nonempty() {
    let input = r#"
<linearGradient stops="0 red">
  <description>gradient</description>
  <stop offset="100%" stop-color="blue"/>
</linearGradient>"#;
    let expected1 = r#"<linearGradient>"#;
    let expected2 = r#"<description>gradient</description>"#;
    let expected3 = r#"<stop offset="0" stop-color="red"/>"#;
    let expected4 = r#"<stop offset="100%" stop-color="blue"/>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected1);
    assert_contains!(output, expected2);
    assert_contains!(output, expected3);
    assert_contains!(output, expected4);
}

#[test]
fn test_gradient_with_stops() {
    let input = r#"<linearGradient id="grad" stops="0% red; 100% blue"/>"#;
    let expected = r#"<linearGradient id="grad"><stop offset="0%" stop-color="red"/><stop offset="100%" stop-color="blue"/></linearGradient>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r#"<radialGradient id="grad" stops="0% yellow; 50% green; 100% black"/>"#;
    let expected = r#"<radialGradient id="grad"><stop offset="0%" stop-color="yellow"/><stop offset="50%" stop-color="green"/><stop offset="100%" stop-color="black"/></radialGradient>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    // with opacity
    let input = r#"<linearGradient id="grad" stops="0% red 1; 100% blue 0.5"/>"#;
    let expected = r#"<linearGradient id="grad"><stop offset="0%" stop-color="red" stop-opacity="1"/><stop offset="100%" stop-color="blue" stop-opacity="0.5"/></linearGradient>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    // fraction instead of percentage
    let input = r#"<radialGradient id="grad" stops="0 red; 0.5 green; 1 black"/>"#;
    let expected = r#"<radialGradient id="grad"><stop offset="0" stop-color="red"/><stop offset="0.5" stop-color="green"/><stop offset="1" stop-color="black"/></radialGradient>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_gradient_compound_attrs() {
    // xy1, xy2 for linearGradient
    let input =
        r#"<linearGradient id="grad" xy1="0,0.5" xy2="80% 90%" stops="0% red; 100% blue"/>"#;
    let expected = r#"<linearGradient id="grad" x1="0" y1="0.5" x2="80%" y2="90%"><stop offset="0%" stop-color="red"/><stop offset="100%" stop-color="blue"/></linearGradient>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    // cxy, fxy for radialGradient
    let input = r#"<radialGradient id="grad" cxy="50%,50%" r="1" fxy="0.3 0.5" stops="0% yellow; 100% black"/>"#;
    let expected = r#"<radialGradient id="grad" cx="50%" cy="50%" r="1" fx="0.3" fy="0.5"><stop offset="0%" stop-color="yellow"/><stop offset="100%" stop-color="black"/></radialGradient>"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_gradient_linear_dir() {
    let input = r#"<linearGradient id="grad" dir="90" stops="0% red; 100% blue"/>"#;
    let expected = r#"<linearGradient id="grad" x1="0" y1="0" x2="0" y2="1">"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r#"<linearGradient id="grad" dir="225" stops="0% red; 100% blue"/>"#;
    let expected = r#"<linearGradient id="grad" x1="1" y1="1" x2="0" y2="0">"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    // dir + length
    let input = r#"<linearGradient id="grad" dir="0" length="50%" stops="0% red; 100% blue"/>"#;
    let expected = r#"<linearGradient id="grad" x1="0" y1="0" x2="0.5" y2="0">"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    let input = r#"<linearGradient id="grad" dir="135" length="50%" stops="0% red; 100% blue"/>"#;
    let expected = r#"<linearGradient id="grad" x1="1" y1="0" x2="0.5" y2="0.5">"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}

#[test]
fn test_gradient_single_point() {
    // origin only
    let input = r#"<linearGradient id="grad" xy1="0.25 0.75" dir="0" stops="0% red; 100% blue"/>"#;
    let expected = r#"<linearGradient id="grad" x1="0.25" y1="0.75" x2="1" y2="0.75">"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    // origin + length
    let input = r#"<linearGradient id="grad" xy1="0.25 0.75" dir="0" length="0.5" stops="0% red; 100% blue"/>"#;
    let expected = r#"<linearGradient id="grad" x1="0.25" y1="0.75" x2="0.75" y2="0.75">"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    // endpoint only
    let input =
        r#"<linearGradient id="grad" xy2="0.25 0.75" dir="270" stops="0% red; 100% blue"/>"#;
    let expected = r#"<linearGradient id="grad" x1="0.25" y1="0" x2="0.25" y2="0.75">"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);

    // endpoint + length
    let input = r#"<linearGradient id="grad" xy2="1 0.5" dir="180" length="0.25" stops="0% red; 100% blue"/>"#;
    let expected = r#"<linearGradient id="grad" x1="0.75" y1="0.5" x2="1" y2="0.5">"#;
    let output = transform_str_default(input).unwrap();
    assert_contains!(output, expected);
}
