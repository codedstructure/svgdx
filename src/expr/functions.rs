use super::expression::{EvalState, ExprValue};
use crate::errors::{Result, SvgdxError};

use rand::Rng;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq)]
pub enum Function {
    /// `abs(x)` -- absolute value of x
    Abs,
    /// `ceil(x)` -- ceiling of x
    Ceil,
    /// `floor(x)` -- floor of x
    Floor,
    /// `fract(x)` -- fractional part of x
    Fract,
    /// `sign(x)` -- -1 for x < 0, 0 for x == 0, 1 for x > 0
    Sign,
    /// `divmod(x, n)` -- x // n, x % n
    DivMod,
    /// `sqrt(x)` -- square root of x
    Sqrt,
    /// `log(x)` -- (natural) log of x
    Log,
    /// `exp(x)` -- raise e to the power of x
    Exp,
    /// `pow(x, y)` -- raise x to the power of y
    Pow,
    /// `sin(x)` -- sine of x (x in degrees)
    Sin,
    /// `cos(x)` -- cosine of x (x in degrees)
    Cos,
    /// `tan(x)` -- tangent of x (x in degrees)
    Tan,
    /// `asin(x)` -- arcsine of x degrees
    Asin,
    /// `acos(x)` -- arccosine of x in degrees
    Acos,
    /// `atan(x)` -- arctangent of x in degrees
    Atan,
    /// `random()` -- generate uniform random number in range 0..1
    Random,
    /// `randint(min, max)` -- generate uniform random integer in range min..max
    RandInt,
    /// `min(a, ...)` -- minimum of values
    Min,
    /// `max(a, ...)` -- maximum of values
    Max,
    /// `sum(a, ...)` -- sum of values
    Sum,
    /// `product(a, ...)` -- product of values
    Product,
    /// `mean(a, ...)` -- mean of values
    Mean,
    /// `clamp(x, min, max)` -- return x, clamped between min and max
    Clamp,
    /// `mix(start, end, amount)` -- linear interpolation between start and end
    Mix,
    /// `eq(a, b)` -- 1 if a == b, 0 otherwise
    Equal,
    /// `ne(a, b)` -- 1 if a != b, 0 otherwise
    NotEqual,
    /// `lt(a, b)` -- 1 if a < b, 0 otherwise
    LessThan,
    /// `le(a, b)` -- 1 if a <= b, 0 otherwise
    LessThanEqual,
    /// `gt(a, b)` -- 1 if a > b, 0 otherwise
    GreaterThan,
    /// `ge(a, b)` -- 1 if a >= b, 0 otherwise
    GreaterThanEqual,
    /// `if(cond, a, b)` -- if cond is non-zero, return a, else return b
    If,
    /// `not(a)` -- 1 if a is zero, 0 otherwise
    Not,
    /// `and(a, b)` -- 1 if both a and b are non-zero, 0 otherwise
    And,
    /// `or(a, b)` -- 1 if either a or b are non-zero, 0 otherwise
    Or,
    /// `xor(a, b)` -- 1 if either a or b are non-zero but not both, 0 otherwise
    Xor,
    /// `swap(a, b)` -- return (b, a)
    Swap,
    /// `r2p(x, y)` -- convert rectangular coordinates to polar
    Rect2Polar,
    /// `p2r(r, theta)` -- convert polar coordinates to rectangular
    Polar2Rect,
    /// `select(n, a, b, ...)` -- select nth argument
    Select,
    /// `addv(a1, a2, ..., aN, b1, b2, ...bN)` -- vector sum
    Addv,
    /// `subv(a1, a2, ..., aN, b1, b2, ...bN)` -- vector difference
    Subv,
    /// `scalev(s, a1, a2, ..., aN)` -- scale vector by s
    Scalev,
    /// `head(a, ...)` -- first element of list
    Head,
    /// `tail(a, ...)` -- all but the first element of list
    Tail,
    /// `empty(a, ...)` -- 1 if list is empty, 0 otherwise
    Empty,
    /// `count(a, ...)` -- number of elements in list
    Count,
    /// `in(x, a, ...)` -- 1 if x is in list, 0 otherwise
    In,
    /// `split(sep, a)` -- split string a into list of substrings using sep
    Split,
    /// `splitw(a)` -- split string on whitespace
    Splitw,
    /// `trim(a)` -- remove leading and trailing whitespace
    Trim,
    /// `join(sep, a, ...)` -- join list of strings into a single string
    Join,
    /// `_(a)` -- return a as text
    Text,
}

impl FromStr for Function {
    type Err = SvgdxError;

    fn from_str(value: &str) -> Result<Self> {
        Ok(match value {
            "abs" => Self::Abs,
            "ceil" => Self::Ceil,
            "floor" => Self::Floor,
            "fract" => Self::Fract,
            "sign" => Self::Sign,
            "divmod" => Self::DivMod,
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
            "sum" => Self::Sum,
            "product" => Self::Product,
            "mean" => Self::Mean,
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
            "swap" => Self::Swap,
            "r2p" => Self::Rect2Polar,
            "p2r" => Self::Polar2Rect,
            "select" => Self::Select,
            "addv" => Self::Addv,
            "subv" => Self::Subv,
            "scalev" => Self::Scalev,
            "head" => Self::Head,
            "tail" => Self::Tail,
            "empty" => Self::Empty,
            "count" => Self::Count,
            "in" => Self::In,
            "split" => Self::Split,
            "splitw" => Self::Splitw,
            "trim" => Self::Trim,
            "join" => Self::Join,
            "_" => Self::Text,
            _ => {
                return Err(SvgdxError::ParseError(format!(
                    "Unknown function: '{value}'"
                )))
            }
        })
    }
}

pub fn eval_function(
    fun: Function,
    args: &ExprValue,
    eval_state: &mut EvalState,
) -> Result<ExprValue> {
    let e = match fun {
        Function::Swap => {
            let (a, b) = args.pair()?;
            return Ok(([b.to_owned(), a.to_owned()].as_slice()).into());
        }
        Function::Rect2Polar => {
            let (x, y) = args.number_pair()?;
            return Ok([x.hypot(y), y.atan2(x).to_degrees()].as_slice().into());
        }
        Function::Polar2Rect => {
            let (r, theta) = args.number_pair()?;
            let theta = theta.to_radians();
            return Ok([r * theta.cos(), r * theta.sin()].as_slice().into());
        }
        Function::Addv => {
            let args = args.number_list()?;
            if args.len() % 2 != 0 {
                return Err(SvgdxError::ParseError(
                    "addv() requires an even number of arguments".to_string(),
                ));
            }
            let halflen = args.len() / 2;
            let mut result = Vec::with_capacity(halflen);
            for i in 0..halflen {
                result.push(args[i] + args[i + halflen]);
            }
            return Ok(result.into());
        }
        Function::Subv => {
            let args = args.number_list()?;
            if args.len() % 2 != 0 {
                return Err(SvgdxError::ParseError(
                    "subv() requires an even number of arguments".to_string(),
                ));
            }
            let halflen = args.len() / 2;
            let mut result = Vec::with_capacity(halflen);
            for i in 0..halflen {
                result.push(args[i] - args[i + halflen]);
            }
            return Ok(result.into());
        }
        Function::Scalev => {
            let args = args.number_list()?;
            if args.len() < 2 {
                return Err(SvgdxError::ParseError(
                    "scalev() requires at least two arguments".to_string(),
                ));
            }
            let mut result = Vec::new();
            for i in 1..args.len() {
                result.push(args[0] * args[i]);
            }
            return Ok(result.into());
        }
        Function::Head => {
            let args = args.flatten();
            if args.is_empty() {
                return Ok(ExprValue::new());
            }
            return Ok(args[0].to_owned());
        }
        Function::Tail => {
            let args = args.flatten();
            if args.len() < 2 {
                return Ok(ExprValue::new());
            }
            return Ok(args[1..args.len()].to_owned().into());
        }
        Function::Empty => {
            if args.is_empty() {
                1.
            } else {
                0.
            }
        }
        Function::Count => args.len() as f32,
        Function::Select => {
            let args = args.flatten();
            if args.len() < 2 {
                return Err(SvgdxError::ParseError(
                    "select() requires at least two arguments".to_string(),
                ));
            }
            let n = args[0].one_number()? as usize;
            let rest = &args[1..];
            if n < rest.len() {
                return Ok(rest[n].to_owned());
            } else {
                return Err(SvgdxError::InvalidData(
                    "select() index out of range".to_string(),
                ));
            }
        }
        Function::In => {
            let args = args.flatten();
            if args.is_empty() {
                return Err(SvgdxError::ParseError(
                    "in() requires at least one argument".to_string(),
                ));
            }
            let value = &args[0];
            let rest = &args[1..];
            if rest.iter().any(|v| v == value) {
                1.
            } else {
                0.
            }
        }
        Function::Abs => args.one_number()?.abs(),
        Function::Ceil => args.one_number()?.ceil(),
        Function::Floor => args.one_number()?.floor(),
        Function::Fract => args.one_number()?.fract(),
        Function::Sign => {
            // Can't just use signum since it returns +1 for
            // input of (positive) zero.
            let e = args.one_number()?;
            if e == 0. {
                0.
            } else {
                e.signum()
            }
        }
        Function::DivMod => {
            let (x, n) = args.number_pair()?;
            let div = x.div_euclid(n);
            let rem = x.rem_euclid(n);
            return Ok([div, rem].as_slice().into());
        }
        Function::Sqrt => args.one_number()?.sqrt(),
        Function::Log => args.one_number()?.ln(),
        Function::Exp => args.one_number()?.exp(),
        Function::Pow => {
            let (x, y) = args.number_pair()?;
            x.powf(y)
        }
        Function::Sin => args.one_number()?.to_radians().sin(),
        Function::Cos => args.one_number()?.to_radians().cos(),
        Function::Tan => args.one_number()?.to_radians().tan(),
        Function::Asin => args.one_number()?.asin().to_degrees(),
        Function::Acos => args.one_number()?.acos().to_degrees(),
        Function::Atan => args.one_number()?.atan().to_degrees(),
        Function::Random => eval_state.context.get_rng().borrow_mut().random::<f32>(),
        Function::RandInt => {
            let (min, max) = args.number_pair()?;
            let (min, max) = (min as i32, max as i32);
            if min > max {
                return Err(SvgdxError::InvalidData(
                    "randint(min, max) - `min` must be <= `max`".to_string(),
                ));
            }
            eval_state
                .context
                .get_rng()
                .borrow_mut()
                .random_range(min..=max) as f32
        }
        Function::Max => args
            .number_list()?
            .into_iter()
            .max_by(|a, b| a.total_cmp(b))
            .ok_or_else(|| {
                SvgdxError::InvalidData("max() requires at least one argument".to_owned())
            })?,
        Function::Min => args
            .number_list()?
            .into_iter()
            .min_by(|a, b| a.total_cmp(b))
            .ok_or_else(|| {
                SvgdxError::InvalidData("min() requires at least one argument".to_owned())
            })?,
        Function::Sum => args.number_list()?.into_iter().sum(),
        Function::Product => args.number_list()?.into_iter().product(),
        Function::Mean => {
            if args.is_empty() {
                return Err(SvgdxError::ParseError(
                    "mean() requires at least one argument".to_string(),
                ));
            }
            let n = args.len() as f32;
            args.number_list()?.into_iter().sum::<f32>() / n
        }
        Function::Clamp => {
            let (x, min, max) = args.number_triple()?;
            if min > max {
                return Err(SvgdxError::InvalidData(
                    "clamp(x, min, max) - `min` must be <= `max`".to_string(),
                ));
            }
            x.clamp(min, max)
        }
        Function::Mix => {
            let (a, b, c) = args.number_triple()?;
            a * (1. - c) + b * c
        }
        Function::Equal => {
            let (a, b) = args.pair()?;
            if a == b {
                1.
            } else {
                0.
            }
        }
        Function::NotEqual => {
            let (a, b) = args.pair()?;
            if a != b {
                1.
            } else {
                0.
            }
        }
        Function::LessThan => {
            let (a, b) = args.number_pair()?;
            if a < b {
                1.
            } else {
                0.
            }
        }
        Function::LessThanEqual => {
            let (a, b) = args.number_pair()?;
            if a <= b {
                1.
            } else {
                0.
            }
        }
        Function::GreaterThan => {
            let (a, b) = args.number_pair()?;
            if a > b {
                1.
            } else {
                0.
            }
        }
        Function::GreaterThanEqual => {
            let (a, b) = args.number_pair()?;
            if a >= b {
                1.
            } else {
                0.
            }
        }
        Function::If => {
            if let [cond, a, b] = &args.flatten()[..] {
                if cond.one_number()? != 0. {
                    return Ok(a.clone());
                } else {
                    return Ok(b.clone());
                }
            }
            return Err(SvgdxError::ParseError(
                "if() requires three arguments".to_string(),
            ));
        }
        Function::Not => {
            if args.one_number()? == 0. {
                1.
            } else {
                0.
            }
        }
        Function::And => {
            let (a, b) = args.number_pair()?;
            if a != 0. && b != 0. {
                1.
            } else {
                0.
            }
        }
        Function::Or => {
            let (a, b) = args.number_pair()?;
            if a != 0. || b != 0. {
                1.
            } else {
                0.
            }
        }
        Function::Xor => {
            let (a, b) = args.number_pair()?;
            if (a != 0.) ^ (b != 0.) {
                1.
            } else {
                0.
            }
        }
        Function::Split => {
            let (sep, a) = args.string_pair()?;
            let sep = sep.to_string();
            let a = a.to_string();
            return Ok(ExprValue::List(
                a.split(&sep)
                    .map(|s| ExprValue::String(s.to_owned()))
                    .collect(),
            ));
        }
        Function::Splitw => {
            let a = args.one_string()?;
            return Ok(ExprValue::List(
                a.split_ascii_whitespace()
                    .map(|s| ExprValue::String(s.to_owned()))
                    .collect(),
            ));
        }
        Function::Trim => {
            let a = args.one_string()?;
            return Ok(ExprValue::String(a.trim().to_owned()));
        }
        Function::Join => {
            if let Some((sep, rest)) = args.string_list()?.split_first() {
                let combined = rest.to_vec().join(sep);
                return Ok(ExprValue::String(combined));
            } else {
                return Err(SvgdxError::ParseError(
                    "join() requires at least one argument".to_string(),
                ));
            }
        }
        Function::Text => {
            let a = args.one_string()?;
            return Ok(ExprValue::Text(a));
        }
    };
    Ok(e.into())
}
