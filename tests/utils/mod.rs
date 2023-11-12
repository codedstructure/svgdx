use std::io::Cursor;
use svgd::svg_transform;

pub fn compare(input: &str, expected: &str) {
    let mut output: Vec<u8> = vec![];

    let mut input = Cursor::new(input);
    svg_transform(&mut input, &mut output).expect("Transform failure");
    let output = String::from_utf8(output).unwrap();

    assert_eq!(output.trim(), expected.trim());
}

pub fn contains(input: &str, expected: &str) {
    let mut output: Vec<u8> = vec![];

    let mut input = Cursor::new(input);
    svg_transform(&mut input, &mut output).expect("Transform failure");
    let output = String::from_utf8(output).unwrap();

    assert!(
        output.contains(expected),
        "\n {}\nnot found in\n {}",
        expected,
        output
    );
}
