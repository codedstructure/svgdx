//! Support for 'repeat' commands in SVG paths.
//!
//! A block of path commands delimited by square brackets is repeated `N` times
//! when preceded by an `r N` command. Together with the bearing commands, this
//! allows turtle-graphics style path definitions.
//!
//! - repeated blocks may be nested.
//! - analogous to the 'z' command, 'r' may be upper or lower case.
//!
//! Expansion accounting, limited by path-repeat-limit config value:
//! - Nested repeats multiply (`r9[r9[...]]` => 81)
//! - Sequential repeats sum (`r9[...] r9[...]` => 18)
//!
//! Example: `<path d="M 0 0 r 6 [ h 10 b 60 ]"/>`

use super::PathSyntax;
use crate::errors::{Error, Result};

pub struct RepeatPathSyntax {
    data: Vec<char>,
    index: usize,
}

impl RepeatPathSyntax {
    pub fn new(data: &str) -> Self {
        Self {
            data: data.chars().collect(),
            index: 0,
        }
    }
}

impl PathSyntax for RepeatPathSyntax {
    fn at_command(&self) -> Result<bool> {
        self.check_not_end()?;
        let c = self
            .current()
            .ok_or_else(|| Error::Parse("no data".to_string()))?;
        // Adds 'r', 'R', '[' and ']' to the set of SVG commands.
        // Also includes 'B' and 'b' bearing commands; repeat should
        // be evaluated before bearing.
        Ok("MmLlHhVvZzCcSsQqTtAaRr[]Bb".contains(c))
    }

    fn current(&self) -> Option<char> {
        self.data.get(self.index).copied()
    }

    fn advance(&mut self) {
        self.index += 1;
    }

    fn at_end(&self) -> bool {
        self.index >= self.data.len()
    }
}

struct PathRepeat {
    tokens: RepeatPathSyntax,
    output: String,
    command: Option<char>,
    limit: u32,
}

impl PathRepeat {
    fn new(data: &str, limit: u32) -> Self {
        PathRepeat {
            tokens: RepeatPathSyntax::new(data),
            output: String::new(),
            command: None,
            limit,
        }
    }

    // returns the number of repetitions processed
    fn process_instruction(&mut self) -> Result<u32> {
        if self.command.is_none() || self.tokens.at_command()? {
            // "The command letter can be eliminated on subsequent commands if the same
            // command is used multiple times in a row (e.g., you can drop the second
            // "L" in "M 100 200 L 200 100 L -100 -200" and use "M 100 200 L 200 100
            // -100 -200" instead)."
            self.command = Some(self.tokens.read_command()?);
        }

        let cmd = self.command.expect("Command should be already set");
        let rep_count = match cmd {
            'R' | 'r' => {
                // Repeat command
                let count = self.tokens.read_count()?;
                self.tokens.skip_whitespace();
                if self.tokens.current() != Some('[') {
                    return Err(Error::Parse(format!("expected '[' after '{cmd} COUNT'")));
                }
                self.tokens.advance(); // skip '['

                // Collect inner tokens
                // TODO: just use indices/slices here rather than building a new string
                let mut inner_tokens = String::new();
                let mut nest_depth = 0;
                while !self.tokens.at_end() {
                    let c = self.tokens.current().unwrap();
                    if c == '[' {
                        nest_depth += 1;
                    } else if c == ']' {
                        if nest_depth == 0 {
                            break;
                        }
                        nest_depth -= 1;
                    }
                    inner_tokens.push(c);
                    self.tokens.advance();
                }

                if self.tokens.current() != Some(']') {
                    return Err(Error::Parse(
                        "expected ']' to close repeat block".to_string(),
                    ));
                }
                self.tokens.advance(); // skip ']'

                // Process inner content recursively
                let (content, inner_rep_count) =
                    process_path_repeat_with_limit(&inner_tokens, self.limit)?;
                let content = content.trim().to_string();

                // Ensure we don't exceed repeat expansion limits
                let nested_rep_count = if inner_rep_count > 0 {
                    count
                        .checked_mul(inner_rep_count)
                        .ok_or_else(|| Error::PathRepeatLimit(u32::MAX, self.limit))?
                } else {
                    count
                };
                if nested_rep_count > self.limit {
                    return Err(Error::PathRepeatLimit(nested_rep_count, self.limit));
                }

                // Repeat the content
                for i in 0..count {
                    if i > 0 {
                        self.output.push(' ');
                    }
                    self.output.push_str(&content);
                }
                self.output.push(' ');
                self.tokens.skip_whitespace();
                self.command = None;

                nested_rep_count
            }
            _ => {
                // copy to output
                self.output.push(cmd);
                while !self.tokens.at_end() {
                    if self.tokens.at_command()? {
                        break;
                    }
                    self.output.push(self.tokens.current().unwrap());
                    self.tokens.advance();
                }

                0
            }
        };
        Ok(rep_count)
    }

    fn evaluate(&mut self) -> Result<u32> {
        self.tokens.skip_whitespace();
        let mut total_repeats = 0;
        while !self.tokens.at_end() {
            total_repeats += self.process_instruction()?;
            if total_repeats > self.limit {
                return Err(Error::PathRepeatLimit(total_repeats, self.limit));
            }
        }
        Ok(total_repeats)
    }
}

/// Support 'repeat' syntax as part of path data.
///
/// Syntax:
///  `r N [ ... ]`
///
/// as with 'z', this command can be upper or lower case.
///
/// Example:
///
/// `"M 0 0 r 3 [ l 10 0 ]"` => `"M 0 0 l 10 0 l 10 0 l 10 0"`
///
/// Repeat commands may be nested. Any unclosed repeat blocks at the
/// end of the document are automatically closed.
pub fn process_path_repeat(data: &str, limit: u32) -> Result<String> {
    Ok(process_path_repeat_with_limit(data, limit)?.0)
}

fn process_path_repeat_with_limit(data: &str, limit: u32) -> Result<(String, u32)> {
    let mut pp = PathRepeat::new(data, limit);
    let reps = pp.evaluate()?;
    Ok((pp.output, reps))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_repeat() {
        let input = r#"M0 0 r 3 [ l 10 0 ] l 5 0"#;
        let output = process_path_repeat(input, 100).unwrap();
        assert_eq!(output, r#"M0 0 l10 0 l10 0 l10 0 l5 0"#);
    }

    #[test]
    fn test_path_repeat_nested() {
        let input = r#"M 0 0 r 3 [ h 3 b 45 r 2 [ l 10 0 ] ] l 5 0"#;
        assert_eq!(
            process_path_repeat(input, 100).unwrap(),
            r#"M0 0 h3 b45 l10 0 l10 0 h3 b45 l10 0 l10 0 h3 b45 l10 0 l10 0 l5 0"#
        );
    }

    #[test]
    fn test_path_repeat_limit() {
        // Check simple repeat exceeding limit
        let input = r#"M0 0 r 1000 [ l 10 0 ]"#;
        let result = process_path_repeat(input, 999);
        assert!(result.is_err());
        let result = process_path_repeat(input, 1000);
        assert!(result.is_ok());

        // Check nested repeats exceeding limit
        let input = r#"M0 0 r 10 [ r 100 [ l 10 0 ] ]"#;
        let result = process_path_repeat(input, 500);
        assert!(result.is_err());

        // Check massive repeat exceeding limit (resource exhaustion if not limited)
        let input = r#"M0 0 r999[r999[r999[r999[r999[h1]]]]]"#;
        let result = process_path_repeat(input, 1_000_000);
        assert!(result.is_err());

        // Check multiple repeat instances are summed
        let input = r#"M0 0 r10[h1] r10[v1]"#;
        let result = process_path_repeat(input, 19);
        assert!(result.is_err());
        let result = process_path_repeat(input, 20);
        assert!(result.is_ok());

        let input = r#"M0 0 r10[h1] r10[r2[v1]] r0[h1] h1 r3[v1 h1]"#;
        let result = process_path_repeat(input, 32);
        assert!(result.is_err());
        let result = process_path_repeat(input, 33);
        assert!(result.is_ok());
    }
}
