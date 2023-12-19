use std::io::Cursor;

use anyhow::Result;

use svgdx::svg_transform;

pub fn compare(input: &str, expected: &str) {
    let mut output: Vec<u8> = vec![];

    let mut input = Cursor::new(input);
    svg_transform(&mut input, &mut output).expect("Transform failure");
    let output = String::from_utf8(output).expect("not UTF8");

    assert_eq!(output.trim(), expected.trim());
}

pub fn contains(input: &str, expected: &str) {
    let mut output: Vec<u8> = vec![];

    let mut input = Cursor::new(input);
    svg_transform(&mut input, &mut output).expect("Transform failure");
    let output = String::from_utf8(output).expect("not UTF8");

    assert!(
        output.contains(expected),
        "\n {}\nnot found in\n {}",
        expected,
        output
    );
}

pub fn transform(input: &str) -> Result<String> {
    let mut output: Vec<u8> = vec![];

    let mut input = Cursor::new(input);
    svg_transform(&mut input, &mut output)?;
    Ok(String::from_utf8(output).expect("not UTF8"))
}
