/// Recursive descent expression parser
use lazy_regex::regex;
use rand::Rng;
use regex::Captures;

use anyhow::{bail, Context, Result};

use crate::transform::TransformerContext;
use crate::types::{fstr, ScalarSpec};

#[derive(Clone, PartialEq)]
enum Function {
    /// abs(x) - absolute value of x
    Abs,
    /// ceil(x) - ceiling of x
    Ceil,
    /// floor(x) - floor of x
    Floor,
    /// fract(x) - fractional part of x
    Fract,
    /// sign(x) - -1 for x < 0, 0 for x == 0, 1 for x > 0
    Sign,
    /// sqrt(x) - square root of x
    Sqrt,
    /// log(x) - (natural) log of x
    Log,
    /// exp(x) - raise e to the power of x
    Exp,
    /// pow(x, y) - raise x to the power of y
    Pow,
    /// sin(x) - sine of x (x in degrees)
    Sin,
    /// cos(x) - cosine of x (x in degrees)
    Cos,
    /// tan(x) - tangent of x (x in degrees)
    Tan,
    /// asin(x) - arcsine of x degrees
    Asin,
    /// acos(x) - arccosine of x in degrees
    Acos,
    /// atan(x) - arctangent of x in degrees
    Atan,
    /// random() - generate uniform random number in range 0..1
    Random,
    /// randint(min, max) - generate uniform random integer in range min..max
    RandInt,
    /// min(a, b) - minimum of two values
    Min,
    /// max(a, b) - maximum of two values
    Max,
    /// clamp(x, min, max) - return x, clamped between min and max
    Clamp,
    /// mix(start, end, amount) - linear interpolation between start and end
    Mix,
    /// eq(a, b) - 1 if a == b, 0 otherwise
    Equal,
    /// ne(a, b) - 1 if a != b, 0 otherwise
    NotEqual,
    /// lt(a, b) - 1 if a < b, 0 otherwise
    LessThan,
    /// le(a, b) - 1 if a <= b, 0 otherwise
    LessThanEqual,
    /// gt(a, b) - 1 if a > b, 0 otherwise
    GreaterThan,
    /// ge(a, b) - 1 if a >= b, 0 otherwise
    GreaterThanEqual,
    /// if(cond, a, b) - if cond is non-zero, return a, else return b
    If,
    /// not(a) - 1 if a is zero, 0 otherwise
    Not,
    /// and(a, b) - 1 if both a and b are non-zero, 0 otherwise
    And,
    /// or(a, b) - 1 if either a or b are non-zero, 0 otherwise
    Or,
    /// xor(a, b) - 1 if either a or b are non-zero but not both, 0 otherwise
    Xor,
}

impl TryFrom<&str> for Function {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> std::prelude::v1::Result<Self, Self::Error> {
        Ok(match value {
            "abs" => Self::Abs,
            "ceil" => Self::Ceil,
            "floor" => Self::Floor,
            "fract" => Self::Fract,
            "sign" => Self::Sign,
            "sqrt" => Self::Sqrt,
            "log" => Self::Log,
            "exp" => Self::Exp,
            "pow" => Self::Pow,
            "sin" => Self::Sin,
            "cos" => Self::Cos,
            "tan" => Self::Tan,
            "asin" => Self::Asin,
            "acos" => Self::Acos,
            "atan" => Self::Atan,
            "random" => Self::Random,
            "randint" => Self::RandInt,
            "min" => Self::Min,
            "max" => Self::Max,
            "clamp" => Self::Clamp,
            "mix" => Self::Mix,
            "eq" => Self::Equal,
            "ne" => Self::NotEqual,
            "lt" => Self::LessThan,
            "le" => Self::LessThanEqual,
            "gt" => Self::GreaterThan,
            "ge" => Self::GreaterThanEqual,
            "if" => Self::If,
            "not" => Self::Not,
            "and" => Self::And,
            "or" => Self::Or,
            "xor" => Self::Xor,
            _ => bail!("Unknown function"),
        })
    }
}

#[derive(Clone, PartialEq)]
enum Token {
    /// A numeric literal
    Number(f32),
    /// A variable reference, beginning with '$'
    Var(String),
    /// Reference to an element-derived value, beginning with '#'
    ElementRef(String),
    /// A function reference
    FnRef(Function),
    /// A literal '('
    OpenParen,
    /// A literal ')'
    CloseParen,
    /// A literal ',' - used for separating function arguments
    Comma,
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
    } else if let Ok(func) = Function::try_from(input) {
        Ok(Token::FnRef(func))
    } else {
        input
            .parse::<f32>()
            .ok()
            .map(Token::Number)
            .with_context(|| format!("Invalid number or function '{input}"))
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
            ',' => Token::Comma,
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

    fn require(&mut self, token: Token) -> Result<()> {
        if self.peek() == Some(token) {
            self.advance();
            Ok(())
        } else {
            bail!("Expected token not matched")
        }
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
            eval_state.require(Token::CloseParen)?;
            Ok(e)
        }
        Some(Token::Sub) => match eval_state.peek() {
            Some(Token::OpenParen) | Some(Token::Number(_)) | Some(Token::Var(_)) => {
                Ok(-expr(eval_state)?)
            }
            _ => bail!("Invalid unary minus"),
        },
        Some(Token::FnRef(fun)) => {
            eval_state.require(Token::OpenParen)?;
            let e = match fun {
                Function::Abs => expr(eval_state)?.abs(),
                Function::Ceil => expr(eval_state)?.ceil(),
                Function::Floor => expr(eval_state)?.floor(),
                Function::Fract => expr(eval_state)?.fract(),
                Function::Sign => {
                    // Can't just use signum since it returns +1 for
                    // input of (positive) zero.
                    let e = expr(eval_state)?;
                    if e == 0. {
                        0.
                    } else {
                        e.signum()
                    }
                }
                Function::Sqrt => expr(eval_state)?.sqrt(),
                Function::Log => expr(eval_state)?.ln(),
                Function::Exp => expr(eval_state)?.exp(),
                Function::Pow => {
                    let x = expr(eval_state)?;
                    eval_state.require(Token::Comma)?;
                    let y = expr(eval_state)?;
                    x.powf(y)
                }
                Function::Sin => expr(eval_state)?.to_radians().sin(),
                Function::Cos => expr(eval_state)?.to_radians().cos(),
                Function::Tan => expr(eval_state)?.to_radians().tan(),
                Function::Asin => expr(eval_state)?.asin().to_degrees(),
                Function::Acos => expr(eval_state)?.acos().to_degrees(),
                Function::Atan => expr(eval_state)?.atan().to_degrees(),
                Function::Random => eval_state.context.get_rng().borrow_mut().gen::<f32>(),
                Function::RandInt => {
                    let min = expr(eval_state)? as i32;
                    eval_state.require(Token::Comma)?;
                    let max = expr(eval_state)? as i32;
                    if min > max {
                        bail!("randint(min, max) - `min` must be <= `max`");
                    }
                    eval_state
                        .context
                        .get_rng()
                        .borrow_mut()
                        .gen_range(min..=max) as f32
                }
                Function::Max => {
                    let a = expr(eval_state)?;
                    eval_state.require(Token::Comma)?;
                    let b = expr(eval_state)?;
                    a.max(b)
                }
                Function::Min => {
                    let a = expr(eval_state)?;
                    eval_state.require(Token::Comma)?;
                    let b = expr(eval_state)?;
                    a.min(b)
                }
                Function::Clamp => {
                    let x = expr(eval_state)?;
                    eval_state.require(Token::Comma)?;
                    let min = expr(eval_state)?;
                    eval_state.require(Token::Comma)?;
                    let max = expr(eval_state)?;
                    if min > max {
                        bail!("clamp(x, min, max) - `min` must be <= `max`");
                    }
                    x.clamp(min, max)
                }
                Function::Mix => {
                    let a = expr(eval_state)?;
                    eval_state.require(Token::Comma)?;
                    let b = expr(eval_state)?;
                    eval_state.require(Token::Comma)?;
                    let c = expr(eval_state)?;
                    a * (1. - c) + b * c
                }
                Function::Equal => {
                    let a = expr(eval_state)?;
                    eval_state.require(Token::Comma)?;
                    let b = expr(eval_state)?;
                    if a == b {
                        1.
                    } else {
                        0.
                    }
                }
                Function::NotEqual => {
                    let a = expr(eval_state)?;
                    eval_state.require(Token::Comma)?;
                    let b = expr(eval_state)?;
                    if a != b {
                        1.
                    } else {
                        0.
                    }
                }
                Function::LessThan => {
                    let a = expr(eval_state)?;
                    eval_state.require(Token::Comma)?;
                    let b = expr(eval_state)?;
                    if a < b {
                        1.
                    } else {
                        0.
                    }
                }
                Function::LessThanEqual => {
                    let a = expr(eval_state)?;
                    eval_state.require(Token::Comma)?;
                    let b = expr(eval_state)?;
                    if a <= b {
                        1.
                    } else {
                        0.
                    }
                }
                Function::GreaterThan => {
                    let a = expr(eval_state)?;
                    eval_state.require(Token::Comma)?;
                    let b = expr(eval_state)?;
                    if a > b {
                        1.
                    } else {
                        0.
                    }
                }
                Function::GreaterThanEqual => {
                    let a = expr(eval_state)?;
                    eval_state.require(Token::Comma)?;
                    let b = expr(eval_state)?;
                    if a >= b {
                        1.
                    } else {
                        0.
                    }
                }
                Function::If => {
                    let cond = expr(eval_state)?;
                    eval_state.require(Token::Comma)?;
                    let a = expr(eval_state)?;
                    eval_state.require(Token::Comma)?;
                    let b = expr(eval_state)?;
                    if cond != 0. {
                        a
                    } else {
                        b
                    }
                }
                Function::Not => {
                    let a = expr(eval_state)?;
                    if a == 0. {
                        1.
                    } else {
                        0.
                    }
                }
                Function::And => {
                    let a = expr(eval_state)?;
                    eval_state.require(Token::Comma)?;
                    let b = expr(eval_state)?;
                    if a != 0. && b != 0. {
                        1.
                    } else {
                        0.
                    }
                }
                Function::Or => {
                    let a = expr(eval_state)?;
                    eval_state.require(Token::Comma)?;
                    let b = expr(eval_state)?;
                    if a != 0. || b != 0. {
                        1.
                    } else {
                        0.
                    }
                }
                Function::Xor => {
                    let a = expr(eval_state)?;
                    eval_state.require(Token::Comma)?;
                    let b = expr(eval_state)?;
                    if (a != 0.) ^ (b != 0.) {
                        1.
                    } else {
                        0.
                    }
                }
            };
            eval_state.require(Token::CloseParen)?;
            Ok(e)
        }
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
    use assertables::{assert_in_delta, assert_in_delta_as_result, assert_lt, assert_lt_as_result};

    use super::*;

    #[test]
    fn test_eval_var() {
        let mut ctx = TransformerContext::new();
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
    }

    #[test]
    fn test_eval_local_vars() {
        let mut ctx = TransformerContext::new();
        // provide some global variables so we can check they are overridden
        for (name, value) in [("one", "1"), ("this_year", "2023")] {
            ctx.set_var(name, value);
        }

        // Check attributes as locals; this would be something like the attributes
        // of a surrounding <g> element, which can be referenced by child elements.
        ctx.push_current_element(&SvgElement::new(
            "g",
            &[
                ("width".to_string(), "3".to_string()),
                ("height".to_string(), "4".to_string()),
                // Check this overrides the 'global' variables
                ("this_year".to_string(), "2024".to_string()),
            ],
        ));
        // push another element - the actual 'current' element containing this attribute.
        // This is skipped in variable lookup, so needed so the previous ('<g>') element
        // is used.
        ctx.push_current_element(&SvgElement::new("rect", &[]));
        assert_eq!(
            eval_vars("$this_year: $width.$one$height", &ctx),
            "2024: 3.14"
        );

        ctx.pop_current_element();
        ctx.pop_current_element();
        // Now `this_year` isn't overridden by the local variable should revert to
        // the global value.
        assert_eq!(eval_vars("$this_year", &ctx), "2023");

        // Check multiple levels of override
        ctx.push_current_element(&SvgElement::new(
            "g",
            &[("level".to_string(), "1".to_string())],
        ));
        ctx.push_current_element(&SvgElement::new(
            "g",
            &[("level".to_string(), "2".to_string())],
        ));
        ctx.push_current_element(&SvgElement::new(
            "g",
            &[("level".to_string(), "3".to_string())],
        ));
        // The 'inner' element, where attributes should be ignored in variable lookup
        ctx.push_current_element(&SvgElement::new(
            "rect",
            &[("level".to_string(), "inside!!".to_string())],
        ));
        assert_eq!(eval_vars("$level", &ctx), "3");
        ctx.pop_current_element();
        assert_eq!(eval_vars("$level", &ctx), "2");
        ctx.pop_current_element();
        assert_eq!(eval_vars("$level", &ctx), "1");
        ctx.pop_current_element();
        assert_eq!(eval_vars("$level", &ctx), "$level");
    }

    #[test]
    fn test_valid_expressions() {
        let mut ctx = TransformerContext::new();
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
            ("abs(1)", Some(1.)),
            ("abs(-1)", Some(1.)),
            ("30 * abs((3 - 4) * 2) + 3", Some(63.)),
            ("min($kilo, $mega)", Some(1000.)),
            ("max($kilo, $mega)", Some(1000000.)),
            ("min($mega, abs($kilo * 1.5))", Some(1500.)),
            ("max($mega * 3, $kilo)", Some(3000000.)),
            ("mix(10, 100, 0.5)", Some(55.)),
            ("mix(-10, 30, 0.25)", Some(0.)),
            ("mix(-10, 30, 0.75)", Some(20.)),
            ("mix(-10, 30, 0)", Some(-10.)),
            ("mix(-10, 30, 1)", Some(30.)),
            ("mix(-10, 30, 10)", Some(390.)),
        ] {
            assert_eq!(evaluate(tokenize(expr).expect("test"), &ctx).ok(), expected);
        }
    }

    #[test]
    fn test_func_simple() {
        let mut ctx = TransformerContext::new();
        for (name, value) in [("kilo", "1000"), ("mega", "($kilo * $kilo)")] {
            ctx.set_var(name, value);
        }
        for (expr, expected) in [
            ("abs(1)", 1.),
            ("abs(-1)", 1.),
            ("30 * abs((3 - 4) * 2) + 3", 63.),
            ("min($kilo, $mega)", 1000.),
            ("max($kilo, $mega)", 1000000.),
            ("min($mega, abs($kilo * 1.5))", 1500.),
            ("max($mega * 3, $kilo)", 3000000.),
            ("mix(10, 100, 0.5)", 55.),
            ("mix(-10, 30, 0.25)", 0.),
            ("mix(-10, 30, 0.75)", 20.),
            ("mix(-10, 30, 0)", -10.),
            ("mix(-10, 30, 1)", 30.),
            ("mix(-10, 30, 10)", 390.),
            ("clamp(30, -10, 20)", 20.),
            ("clamp(-30, -10, 20)", -10.),
            ("ceil(-2.5)", -2.),
            ("ceil(2.5)", 3.),
            ("floor(-2.5)", -3.),
            ("floor(2.5)", 2.),
            ("fract(2.75)", 0.75),
            ("fract(-2.75)", -0.75),
            ("sign(-2.75)", -1.),
            ("sign(0.75)", 1.),
            ("sign(0)", 0.),
            ("sqrt(81)", 9.),
            ("sqrt(2)", std::f32::consts::SQRT_2),
            ("log(1000) / log(10)", 3.),
            ("log(4096) / log(2)", 12.),
            ("log(exp(1))", 1.),
            ("exp(1)", std::f32::consts::E),
            ("pow(2, 10)", 1024.),
            ("pow(2, -1)", 0.5),
            ("pow(9, 0.5)", 3.),
        ] {
            assert_in_delta!(
                evaluate(tokenize(expr).expect("test"), &ctx).ok().unwrap(),
                expected,
                0.00001
            );
        }
    }

    #[test]
    fn test_func_trig() {
        let ctx = TransformerContext::new();
        for (expr, expected) in [
            ("sin(0)", 0.),
            ("sin(45)", 2_f32.sqrt() / 2.),
            ("sin(90)", 1.),
            ("cos(0)", 1.),
            ("cos(30)", 3_f32.sqrt() / 2.),
            ("cos(60)", 0.5),
            ("cos(90)", 0.),
            ("tan(0)", 0.),
            ("tan(45)", 1.),
            ("tan(60)", 3_f32.sqrt()),
            ("asin(sin(30))", 30.),
            ("acos(cos(30))", 30.),
            ("atan(tan(30))", 30.),
            ("asin(sin(60))", 60.),
            ("acos(cos(60))", 60.),
            ("atan(tan(60))", 60.),
        ] {
            assert_in_delta!(
                evaluate(tokenize(expr).expect("test"), &ctx).ok().unwrap(),
                expected,
                0.00001
            );
        }
    }

    #[test]
    fn test_func_random() {
        // Check random() provides reasonable samples
        let ctx = TransformerContext::new();
        let expr = "random()";
        let tokens = tokenize(expr).unwrap();
        let mut count_a = 0;
        let mut count_b = 0;
        for counter in 0..1000 {
            let sample = evaluate(tokens.clone(), &ctx).ok().unwrap();
            assert!((0. ..=1.).contains(&sample));
            if count_a == 0 && sample > 0.3 && sample < 0.35 {
                count_a = counter;
            }
            if count_b == 0 && sample > 0.85 && sample < 0.9 {
                count_b = counter;
            }
            if count_a != 0 && count_b != 0 {
                break;
            }
        }
        assert_ne!(count_a, 0);
        assert_lt!(count_a, 100);
        assert_ne!(count_b, 0);
        assert_lt!(count_b, 100);

        // Check randint hits different values
        let expr = "randint(1, 6)";
        let tokens = tokenize(expr).unwrap();
        let mut count_a = 0;
        let mut count_b = 0;
        for counter in 0..1000 {
            let sample = evaluate(tokens.clone(), &ctx).ok().unwrap();
            assert!((1. ..=6.).contains(&sample));
            if count_a == 0 && sample == 1. {
                count_a = counter;
            }
            if count_b == 0 && sample == 6. {
                count_b = counter;
            }
            if count_a != 0 && count_b != 0 {
                break;
            }
        }
        assert_ne!(count_a, 0);
        assert_lt!(count_a, 100);
        assert_ne!(count_b, 0);
        assert_lt!(count_b, 100);
    }

    #[test]
    fn test_func_comparison() {
        let ctx = TransformerContext::new();
        for (expr, expected) in [
            ("eq(1, 1)", 1.),
            ("eq(1, 2)", 0.),
            ("ne(1, 1)", 0.),
            ("ne(1, 2)", 1.),
            ("lt(1, 2)", 1.),
            ("lt(2, 1)", 0.),
            ("lt(1, 1)", 0.),
            ("le(1, 2)", 1.),
            ("le(2, 1)", 0.),
            ("le(1, 1)", 1.),
            ("gt(2, 1)", 1.),
            ("gt(1, 2)", 0.),
            ("gt(1, 1)", 0.),
            ("ge(2, 1)", 1.),
            ("ge(1, 2)", 0.),
            ("ge(1, 1)", 1.),
        ] {
            assert_eq!(
                evaluate(tokenize(expr).expect("test"), &ctx).ok().unwrap(),
                expected,
            );
        }
    }

    #[test]
    fn test_func_logic() {
        let ctx = TransformerContext::new();
        for (expr, expected) in [
            ("if(1, 2, 3)", 2.),
            ("if(0, 2, 3)", 3.),
            ("if(100, 2, 3)", 2.),
            ("not(1)", 0.),
            ("not(0)", 1.),
            ("not(100)", 0.),
            ("and(1, 1)", 1.),
            ("and(1, 0)", 0.),
            ("and(0, 1)", 0.),
            ("and(0, 0)", 0.),
            ("or(1, 1)", 1.),
            ("or(1, 0)", 1.),
            ("or(0, 1)", 1.),
            ("or(0, 0)", 0.),
            ("xor(1, 1)", 0.),
            ("xor(1, 0)", 1.),
            ("xor(0, 1)", 1.),
            ("xor(0, 0)", 0.),
        ] {
            assert_eq!(
                evaluate(tokenize(expr).expect("test"), &ctx).ok().unwrap(),
                expected,
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
        let mut ctx = TransformerContext::new();
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
                evaluate(tokenize(expr).expect("test"), &TransformerContext::new(),).ok(),
                expected
            );
        }
    }

    #[test]
    fn test_eval_attr() {
        let mut ctx = TransformerContext::new();
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
