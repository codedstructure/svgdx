use svgdx::transform_str_default;

#[test]
fn test_transform_full_svg() {
    let input = include_str!("./data/transform-in.svg");
    let expected = include_str!("./data/transform-out.svg");

    assert_eq!(transform_str_default(input).unwrap(), expected);
}
