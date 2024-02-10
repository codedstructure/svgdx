/// Recursive descent expression parser
use lazy_regex::regex;
use regex::Captures;

use anyhow::{bail, Context, Result};

use crate::transform::TransformerContext;
use crate::types::{fstr, ScalarSpec};

#[derive(Clone, PartialEq)]
enum Token {
    /// A numeric literal
    Number(f32),
    /// A variable reference, beginning with '$'
    Var(String),
    /// Reference to an element-derived value, beginning with '#'
    ElementRef(String),
    /// A literal '('
    OpenParen,
    /// A literal ')'
    CloseParen,
    /// A literal '+' for addition
    Add,
    /// A literal '-' for subtraction or unary minus
    Sub,
    /// A literal '*' for multiplication
    Mul,
    /// A literal '/' for division
    Div,
    /// A literal '%' for mod operation
    Mod,
    /// Tabs and spaces are whitespace
    Whitespace,
    /// Internal-only token for collecting characters for use in
    /// `Number`, `Var` or `ElementRef` variants.
    Other,
}

fn valid_variable_name(var: &str) -> Result<&str> {
    let re = regex!(r"[a-zA-Z][a-zA-Z0-9_]*");
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
    } else if input.starts_with('#') {
        Ok(Token::ElementRef(input.to_owned()))
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
        if next_token == Token::Other {
            buffer.push(ch);
        } else {
            if !buffer.is_empty() {
                let buffer_token = tokenize_atom(&buffer.iter().collect::<String>())?;
                buffer.clear();
                tokens.push(buffer_token);
            }
            if next_token != Token::Whitespace {
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

struct EvalState<'a> {
    tokens: Vec<Token>,
    index: usize,
    context: &'a TransformerContext,
}

impl<'a> EvalState<'a> {
    fn new(tokens: impl IntoIterator<Item = Token>, context: &'a TransformerContext) -> Self {
        Self {
            tokens: tokens.into_iter().collect(),
            index: 0,
            context,
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
        self.context
            .get_var(v)
            .and_then(|t| evaluate(tokenize(&t).ok()?, self.context).ok())
            .context("Could not evaluate variable")
    }

    /// Extract a single numeric value according to the given spec.
    ///
    /// Example: `#abc.h` - height of element #abc
    ///
    /// The following values are available; all assuming a bounding box:
    /// t - the y coordinate of the top of the element
    /// r - the x coordinate of the right of the element
    /// b - the y coordinate of the bottom of the element
    /// l - the x coordinate of the left of the element
    /// w - the width of the element
    /// h - the height of the element
    fn element_ref(&self, v: &str) -> Result<f32> {
        // TODO: perhaps this should be in the SvgElement impl, so it can
        // be re-used by other single-value attribute references, e.g.
        // <line x1="#abc.l" .../>
        let re = regex!(r"#(?<id>[[:alpha:]][[:word:]]*)\.(?<val>[[:alpha:]][[:word:]]*)");
        if let Some(caps) = re.captures(v) {
            let id = caps.name("id").expect("must match if here").as_str();
            let val = caps.name("val").expect("must match if here").as_str();
            let val = ScalarSpec::try_from(val)?;
            if let Some(elem) = self.context.get_element(id) {
                if let Some(bb) = elem.bbox()? {
                    Ok(bb.scalarspec(val))
                } else {
                    bail!("No bounding box for #{}", id);
                }
            } else {
                bail!("Element #{} not found", id);
            }
        } else {
            bail!("Invalid element_ref: {}", v);
        }
    }
}

fn evaluate(tokens: impl IntoIterator<Item = Token>, context: &TransformerContext) -> Result<f32> {
    let mut eval_state = EvalState::new(tokens, context);
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
        Some(Token::ElementRef(v)) => eval_state.element_ref(&v),
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
pub fn eval_vars(value: &str, context: &TransformerContext) -> String {
    let re = regex!(r"(?<inner>\$(\{(?<var_brace>[[:word:]]+)\}|(?<var_simple>([[:word:]]+))))");
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
            context
                .get_var(cap.as_str())
                .unwrap_or(inner.as_str().to_string())
                .to_string()
        }
    });
    // Following that, replace any escaped "\$" back into "$"" characters
    let re = regex!(r"\\\$");
    re.replace_all(&value, r"$").into_owned()
}

/// Expand arithmetic expressions (including numeric variable lookup) in {{...}}
fn eval_expr(value: &str, context: &TransformerContext) -> String {
    // Note - non-greedy match to catch "{{a}} {{b}}" as 'a' & 'b', rather than 'a}} {{b'
    let re = regex!(r"\{\{(?<inner>.+?)\}\}");
    re.replace_all(value, |caps: &Captures| {
        let inner = caps
            .name("inner")
            .expect("Matched regex must have this group")
            .as_str();
        if let Ok(tokens) = tokenize(inner) {
            if let Ok(parsed) = evaluate(tokens, context) {
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
pub fn eval_attr(value: &str, context: &TransformerContext) -> String {
    // Step 1: Evaluate arithmetic expressions. All variables referenced here
    // are assumed to resolve to a numeric expression.
    let value = eval_expr(value, context);
    // Step 2: Replace other variables (e.g. for string values)
    eval_vars(&value, context)
}

#[cfg(test)]
mod tests {
    use crate::element::SvgElement;

    use super::*;

    #[test]
    fn test_eval_var() {
        let mut ctx = TransformerContext::default();
        for (name, value) in [
            ("one", "1"),
            ("this_year", "2023"),
            ("empty", ""),
            ("me", "Ben"),
        ] {
            ctx.set_var(name, value);
        }

        assert_eq!(eval_vars("$one", &ctx), "1");
        assert_eq!(eval_vars("${one}", &ctx), "1");
        assert_eq!(eval_vars("$two", &ctx), "$two");
        assert_eq!(eval_vars("${two}", &ctx), "${two}");
        assert_eq!(eval_vars(r"\${one}", &ctx), "${one}");
        assert_eq!(eval_vars(r"$one$empty$one$one", &ctx), "111");
        assert_eq!(eval_vars(r"$one$emptyone$one$one", &ctx), "1$emptyone11");
        assert_eq!(eval_vars(r"$one${empty}one$one$one", &ctx), "1one11");
        assert_eq!(eval_vars(r"$one $one $one $one", &ctx), "1 1 1 1");
        assert_eq!(eval_vars(r"${one}${one}$one$one", &ctx), "1111");
        assert_eq!(eval_vars(r"${one}${one}\$one$one", &ctx), "11$one1");
        assert_eq!(eval_vars(r"${one}${one}\${one}$one", &ctx), "11${one}1");
        assert_eq!(eval_vars(r"Thing1${empty}Thing2", &ctx), "Thing1Thing2");
        assert_eq!(
            eval_vars(r"Created in ${this_year} by ${me}", &ctx),
            "Created in 2023 by Ben"
        );

        // Check attributes as locals
        ctx.set_current_element(&SvgElement::new(
            "rect",
            &[
                ("width".to_string(), "3".to_string()),
                ("height".to_string(), "4".to_string()),
                // Check this overrides the 'global' variables
                ("this_year".to_string(), "2024".to_string()),
            ],
        ));
        assert_eq!(
            eval_vars("$this_year: $width.$one$height", &ctx),
            "2024: 3.14"
        )
    }

    #[test]
    fn test_valid_expressions() {
        let mut ctx = TransformerContext::default();
        for (name, value) in [
            ("pi", "3.1415927"),
            ("tau", "(2. * $pi)"),
            ("milli", "0.001"),
            ("micro", "($milli * $milli)"),
            ("kilo", "1000"),
            ("mega", "($kilo * $kilo)"),
        ] {
            ctx.set_var(name, value);
        }
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
            assert_eq!(evaluate(tokenize(expr).expect("test"), &ctx).ok(), expected);
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
            assert!(tokenize(expr).is_ok(), "Should succeed: {expr}");
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
            assert!(tokenize(expr).is_err(), "Should have failed: {expr}");
        }
    }

    #[test]
    fn test_bad_expressions() {
        let mut ctx = TransformerContext::default();
        ctx.set_var("numbers", "20 40");
        for expr in ["1+", "--23", "2++2", "%1", "(1+2", "1+4)", "$numbers"] {
            assert!(evaluate(tokenize(expr).expect("test"), &ctx).is_err());
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
                evaluate(
                    tokenize(expr).expect("test"),
                    &TransformerContext::default(),
                )
                .ok(),
                expected
            );
        }
    }

    #[test]
    fn test_eval_attr() {
        let mut ctx = TransformerContext::default();
        for (name, value) in [
            ("one", "1"),
            ("this_year", "2023"),
            ("empty", ""),
            ("me", "Ben"),
            ("numbers", "20  40"),
        ] {
            ctx.set_var(name, value);
        }

        assert_eq!(
            eval_attr("Made by ${me} in 20{{20 + ${one} * 3}}", &ctx),
            "Made by Ben in 2023"
        );
        assert_eq!(
            eval_attr("Made by ${me} in {{5*4}}{{20 + ${one} * 3}}", &ctx),
            "Made by Ben in 2023"
        );
        // This should 'fail' evaluation and be preserved as the variable value
        assert_eq!(eval_vars(r"$numbers", &ctx), "20  40");
    }
}
