/// Preprocess path 'd' and polyline/polygon 'points' attributes
///
/// * compact runs of whitespace into a single space
/// * strip '//' to end-of-line comments
///   * while leaving `//` untouched inside `{{ ... }}` expressions,
///     where it is the integer division operator.
pub fn preprocess_dpoints(data: &str) -> String {
    let data = data.trim();
    let mut output = String::with_capacity(data.len());
    let mut prev_was_space = false;
    let mut chars = data.chars().peekable();
    let mut in_expr = false;

    while let Some(ch) = chars.next() {
        if ch == '{' && chars.peek() == Some(&'{') {
            output.push(ch);
            output.push(chars.next().unwrap());
            prev_was_space = false;
            in_expr = true;
            continue;
        }

        if in_expr && ch == '}' && chars.peek() == Some(&'}') {
            output.push(ch);
            output.push(chars.next().unwrap());
            prev_was_space = false;
            in_expr = false;
            continue;
        }

        // Note '//' is chosen as comment delimiter rather than '#'
        // that is meaningful directly in attribute values for elrefs
        if !in_expr && ch == '/' && chars.peek() == Some(&'/') {
            // comment - skip to end of line
            while chars.peek().is_some_and(|next| *next != '\n') {
                chars.next();
            }
            continue;
        }

        // compact whitespace (potentially including NBSP etc) to single space
        if ch.is_whitespace() {
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

    #[test]
    fn test_preprocess_dpoints_preserves_expr_integer_division() {
        let input = "M {{3//2}} 0 // comment\nL 1 1";
        let expected = "M {{3//2}} 0 L 1 1";
        let output = preprocess_dpoints(input);
        assert_eq!(output, expected);
    }
}
