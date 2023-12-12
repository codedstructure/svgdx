/// Recursive descent expression parser
use regex::{Captures, Regex};
use std::collections::HashMap;

use anyhow::{bail, Context, Result};

use crate::fstr;

#[derive(Clone, PartialEq)]
enum Token {
    Number(f32),
    Var(String),
    OpenParen,
    CloseParen,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Whitespace,
    Other,
}

fn valid_variable_name(var: &str) -> Result<&str> {
    let re = Regex::new(r"[a-zA-Z][a-zA-Z0-9_]*").expect("Bad Regex");
    if !re.is_match(var) {
        bail!("Invalid variable name");
    }
    Ok(var)
}

fn tokenize_atom(input: &str) -> Result<Token> {
    if let Some(input) = input.strip_prefix('$') {
        let var_name = if let Some(input) = input.strip_prefix('{') {
            input.strip_suffix('}')
        } else {
            Some(input)
        };
        if let Some(var) = var_name {
            valid_variable_name(var).map(|v| Token::Var(v.to_string()))
        } else {
            bail!("Invalid variable");
        }
    } else {
        input
            .parse::<f32>()
            .ok()
            .map(Token::Number)
            .context("Invalid number")
    }
}

fn tokenize(input: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut buffer = Vec::new();

    for ch in input.chars() {
        let next_token = match ch {
            '(' => Token::OpenParen,
            ')' => Token::CloseParen,
            '+' => Token::Add,
            '-' => Token::Sub,
            '*' => Token::Mul,
            '/' => Token::Div,
            '%' => Token::Mod,
            ' ' | '\t' => Token::Whitespace,
            _ => Token::Other,
        };
        match next_token {
            Token::Other => {
                buffer.push(ch);
            }
            Token::Whitespace => {
                continue;
            }
            _ => {
                if !buffer.is_empty() {
                    let buffer_token = tokenize_atom(&buffer.iter().collect::<String>())?;
                    buffer.clear();
                    tokens.push(buffer_token);
                }
                tokens.push(next_token);
            }
        }
    }

    if !buffer.is_empty() {
        let buffer_token = tokenize_atom(&buffer.iter().collect::<String>())?;
        buffer.clear();
        tokens.push(buffer_token);
    }
    Ok(tokens)
}

struct EvalState {
    tokens: Vec<Token>,
    index: usize,
    var_table: HashMap<String, String>,
}

impl EvalState {
    fn new(tokens: impl IntoIterator<Item = Token>, var_table: HashMap<String, String>) -> Self {
        Self {
            tokens: tokens.into_iter().collect(),
            index: 0,
            var_table,
        }
    }

    fn peek(&self) -> Option<Token> {
        self.tokens.get(self.index).cloned()
    }

    fn next(&mut self) -> Option<Token> {
        self.tokens.get(self.index).map(|v| {
            self.index += 1;
            v.clone()
        })
    }

    fn advance(&mut self) {
        self.index += 1;
    }

    fn lookup(&self, v: &str) -> Result<f32> {
        self.var_table
            .get(v)
            .and_then(|t| evaluate(tokenize(t).ok()?, &self.var_table).ok())
            .context("Could not evaluate variable")
    }
}

fn evaluate(
    tokens: impl IntoIterator<Item = Token>,
    vars: &HashMap<String, String>,
) -> Result<f32> {
    let mut eval_state = EvalState::new(tokens, vars.clone());
    let e = expr(&mut eval_state);
    if eval_state.peek().is_none() {
        e
    } else {
        bail!("Unexpected trailing tokens")
    }
}

fn expr(eval_state: &mut EvalState) -> Result<f32> {
    let mut e = term(eval_state)?;
    loop {
        match eval_state.peek() {
            Some(Token::Add) => {
                eval_state.advance();
                e += term(eval_state)?;
            }
            Some(Token::Sub) => {
                eval_state.advance();
                e -= term(eval_state)?;
            }
            _ => {
                break;
            }
        }
    }
    Ok(e)
}

fn term(eval_state: &mut EvalState) -> Result<f32> {
    let mut e = factor(eval_state)?;
    loop {
        match eval_state.peek() {
            Some(Token::Mul) => {
                eval_state.advance();
                e *= factor(eval_state)?;
            }
            Some(Token::Div) => {
                eval_state.advance();
                e /= factor(eval_state)?;
            }
            Some(Token::Mod) => {
                eval_state.advance();
                e %= factor(eval_state)?;
            }
            _ => {
                break;
            }
        }
    }
    Ok(e)
}

fn factor(eval_state: &mut EvalState) -> Result<f32> {
    match eval_state.next() {
        Some(Token::Number(x)) => Ok(x),
        Some(Token::Var(v)) => eval_state.lookup(&v),
        Some(Token::OpenParen) => {
            let e = expr(eval_state)?;
            if eval_state.next() != Some(Token::CloseParen) {
                bail!("Expected closing parenthesis");
            }
            Ok(e)
        }
        Some(Token::Sub) => match eval_state.peek() {
            Some(Token::OpenParen) | Some(Token::Number(_)) | Some(Token::Var(_)) => {
                Ok(-expr(eval_state)?)
            }
            _ => bail!("Invalid unary minus"),
        },
        _ => bail!("Invalid token in factor()"),
    }
}

/// Convert unescaped '$var' or '${var}' in given input according
/// to the supplied variables. Missing variables are left as-is.
pub fn eval_vars(value: &str, variables: &HashMap<String, String>) -> String {
    let re =
        Regex::new(r"(?<inner>\$(\{(?<var_brace>[[:word:]]+)\}|(?<var_simple>([[:word:]]+))))")
            .expect("invalid regex");
    let value = re.replace_all(value, |caps: &Captures| {
        let inner = caps
            .name("inner")
            .expect("Matched regex must have this group");
        // Check if the match is escaped; do this here rather than within the regex
        // to avoid the need for an extra initial character which can cause matches
        // to overlap and fail replacement. We're safe to look at the previous byte
        // since Match.start() is guaranteed to be a utf8 char boundary, and '\' has
        // the top bit clear, so will only match on a one-byte utf8 char.
        let start = inner.start();
        if start > 0 && value.as_bytes()[start - 1] == b'\\' {
            inner.as_str().to_string()
        } else {
            let cap = caps.name("var_brace").unwrap_or_else(|| {
                caps.name("var_simple")
                    .expect("Matched regex must have var_simple or var_brace")
            });
            variables
                .get(&cap.as_str().to_string())
                .unwrap_or(&inner.as_str().to_string())
                .to_string()
        }
    });
    // Following that, replace any escaped "\$" back into "$"" characters
    let re = Regex::new(r"\\\$").expect("invalid regex");
    re.replace_all(&value, r"$").into_owned()
}

/// Expand arithmetic expressions (including numeric variable lookup) in {{...}}
fn eval_expr(value: &str, variables: &HashMap<String, String>) -> String {
    // Note - non-greedy match to catch "{{a}} {{b}}" as 'a' & 'b', rather than 'a}} {{b'
    let re = Regex::new(r"\{\{(?<inner>.+?)\}\}").expect("invalid regex");
    re.replace_all(value, |caps: &Captures| {
        let inner = caps
            .name("inner")
            .expect("Matched regex must have this group")
            .as_str();
        if let Ok(tokens) = tokenize(inner) {
            if let Ok(parsed) = evaluate(tokens, variables) {
                fstr(parsed)
            } else {
                inner.to_owned()
            }
        } else {
            inner.to_owned()
        }
    })
    .to_string()
}

/// Evaluate attribute value including {{arithmetic}} and ${variable} expressions
pub fn eval_attr(value: &str, variables: &HashMap<String, String>) -> String {
    // Step 1: Evaluate arithmetic expressions. All variables referenced here
    // are assumed to resolve to a numeric expression.
    let value = eval_expr(value, variables);
    // Step 2: Replace other variables (e.g. for string values)
    eval_vars(&value, variables)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_var() {
        let vars: HashMap<String, String> = [
            ("one", "1"),
            ("this_year", "2023"),
            ("empty", ""),
            ("me", "Ben"),
        ]
        .iter()
        .map(|v| (v.0.to_string(), v.1.to_string()))
        .collect();

        assert_eq!(eval_vars("$one", &vars), "1");
        assert_eq!(eval_vars("${one}", &vars), "1");
        assert_eq!(eval_vars("$two", &vars), "$two");
        assert_eq!(eval_vars("${two}", &vars), "${two}");
        assert_eq!(eval_vars(r"\${one}", &vars), "${one}");
        assert_eq!(eval_vars(r"$one$empty$one$one", &vars), "111");
        assert_eq!(eval_vars(r"$one$emptyone$one$one", &vars), "1$emptyone11");
        assert_eq!(eval_vars(r"$one${empty}one$one$one", &vars), "1one11");
        assert_eq!(eval_vars(r"$one $one $one $one", &vars), "1 1 1 1");
        assert_eq!(eval_vars(r"${one}${one}$one$one", &vars), "1111");
        assert_eq!(eval_vars(r"${one}${one}\$one$one", &vars), "11$one1");
        assert_eq!(eval_vars(r"${one}${one}\${one}$one", &vars), "11${one}1");
        assert_eq!(eval_vars(r"Thing1${empty}Thing2", &vars), "Thing1Thing2");
        assert_eq!(
            eval_vars(r"Created in ${this_year} by ${me}", &vars),
            "Created in 2023 by Ben"
        );
    }

    #[test]
    fn test_valid_expressions() {
        let variables = HashMap::from([
            ("pi".to_owned(), "3.1415927".to_owned()),
            ("tau".to_owned(), "(2. * $pi)".to_owned()),
            ("milli".to_owned(), "0.001".to_owned()),
            ("micro".to_owned(), "($milli * $milli)".to_owned()),
            ("kilo".to_owned(), "1000".to_owned()),
            ("mega".to_owned(), "($kilo * $kilo)".to_owned()),
        ]);
        for (expr, expected) in [
            ("2 * 2", Some(4.)),
            ("2 * 2 + 1", Some(5.)),
            ("2 * (2 + 1)", Some(6.)),
            ("(2+1)/2", Some(1.5)),
            ("   (   2 +   1)   / 2", Some(1.5)),
            ("(1+(1+(1+(1+(2+1)))))/2", Some(3.5)),
            ("$pi * 10.", Some(31.415927_f32)),
            ("${pi}*-100.", Some(-314.15927_f32)),
            ("-${pi}*-100.", Some(314.15927_f32)),
            ("${tau} - 6", Some(0.28318548)),
            ("0.125 * $mega", Some(125000.)),
        ] {
            assert_eq!(
                evaluate(tokenize(expr).expect("test"), &variables).ok(),
                expected
            );
        }
    }

    #[test]
    fn test_good_tokenize() {
        for expr in [
            "(((((4+5)))))",
            "$abcthing",
            "$abc-${thing}",
            "$abc}", // Not obvious, but '}' here is just another character.
            "${abcthing}",
        ] {
            assert!(tokenize(expr).is_ok(), "Should succeed: {}", expr);
        }
    }

    #[test]
    fn test_bad_tokenize() {
        for expr in [
            "4&5",
            "$",
            "${}",
            "${abc",
            "${-}",
            "${abc-thing}",
            "${abc-thing}",
            "234#",
        ] {
            assert!(tokenize(expr).is_err(), "Should have failed: {}", expr);
        }
    }

    #[test]
    fn test_bad_expressions() {
        for expr in ["1+", "--23", "2++2", "%1", "(1+2", "1+4)"] {
            assert!(evaluate(tokenize(expr).expect("test"), &HashMap::new()).is_err());
        }
    }

    #[test]
    fn test_eval_precedence() {
        for (expr, expected) in [
            // Subtraction is left-to-right
            ("3-2-1", Some(0.)),
            ("3-(2-1)", Some(2.)),
            // Multiplication is higher precedence than +
            ("2+3*5+2", Some(19.)),
            // Modulo is same precedence as *, left-to-right.
            ("3*4*5%2", Some(0.)),
            ("5%2*3*4", Some(12.)),
            ("3*4*(5%2)", Some(12.)),
            // Unary minus
            ("4*-3", Some(-12.)),
            ("-4*-(2+1)", Some(12.)),
        ] {
            assert_eq!(
                evaluate(tokenize(expr).expect("test"), &HashMap::new()).ok(),
                expected
            );
        }
    }

    #[test]
    fn test_eval_attr() {
        let vars: HashMap<String, String> = [
            ("one", "1"),
            ("this_year", "2023"),
            ("empty", ""),
            ("me", "Ben"),
        ]
        .iter()
        .map(|v| (v.0.to_string(), v.1.to_string()))
        .collect();

        assert_eq!(
            eval_attr("Made by ${me} in 20{{20 + ${one} * 3}}", &vars),
            "Made by Ben in 2023"
        );
        assert_eq!(
            eval_attr("Made by ${me} in {{5*4}}{{20 + ${one} * 3}}", &vars),
            "Made by Ben in 2023"
        );
    }
}
