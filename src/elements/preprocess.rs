/// Preprocess path 'd' and polyline/polygon 'points' attributes
///
/// * compact runs of whitespace into a single space
/// * strip '//' to end-of-line comments
///   * requires any expressions have already been evaluated, since
///     `//` is integer division operator inside `{{ ... }}` expressions
pub fn preprocess_dpoints(data: &str) -> String {
    let data = data.trim();
    let mut output = String::with_capacity(data.len());
    let mut prev_was_space = false;
    let mut chars = data.chars().peekable();

    while let Some(ch) = chars.next() {
        // Note '//' is chosen as comment delimiter rather than '#'
        // that is meaningful directly in attribute values for elrefs
        if ch == '/' && chars.peek() == Some(&'/') {
            // comment - skip to end of line
            while chars.peek().is_some_and(|next| *next != '\n') {
                chars.next();
            }
            continue;
        }

        if ch.is_ascii_whitespace() {
            if !prev_was_space && !output.is_empty() {
                output.push(' ');
                prev_was_space = true;
            }
            continue;
        }

        output.push(ch);
        prev_was_space = false;
    }

    if output.ends_with(' ') {
        output.pop();
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preprocess_dpoints() {
        let input = r#"
            M 0 0
            L   10  10 // this is a comment
            L 20 20// another comment
    // comment
z


Z
z
        "#;
        let expected = "M 0 0 L 10 10 L 20 20 z Z z";
        let output = preprocess_dpoints(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_preprocess_drawing_attr_skips_leading_comment_space() {
        let input = "// comment\nM 0 0";
        let expected = "M 0 0";
        let output = preprocess_dpoints(input);
        assert_eq!(output, expected);
    }
}
