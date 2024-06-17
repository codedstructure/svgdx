use crate::expression::{EvalState, ExprValue};
use anyhow::{bail, Context, Result};
use rand::Rng;
use std::str::FromStr;

#[derive(Clone, PartialEq)]
pub enum Function {
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
    /// min(a, ...) - minimum of values
    Min,
    /// max(a, ...) - maximum of values
    Max,
    /// sum(a, ...) - sum of values
    Sum,
    /// product(a, ...) - product of values
    Product,
    /// mean(a, ...) - mean of values
    Mean,
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
    /// swap(a, b) - return (b, a)
    Swap,
    /// r2p(x, y) - convert rectangular coordinates to polar
    Rect2Polar,
    /// p2r(r, theta) - convert polar coordinates to rectangular
    Polar2Rect,
    /// select(n, a, b, ...) - select nth argument
    Select,
}

impl FromStr for Function {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
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
            _ => bail!("Unknown function"),
        })
    }
}

trait FnArgs {
    fn get_only(&self) -> Result<f32>;
    fn get_pair(&self) -> Result<(f32, f32)>;
    fn get_triple(&self) -> Result<(f32, f32, f32)>;
}

impl FnArgs for ExprValue {
    fn get_only(&self) -> Result<f32> {
        match self {
            ExprValue::Number(n) => Ok(*n),
            ExprValue::NumberList(l) => {
                if l.len() != 1 {
                    bail!("Expected exactly one argument");
                }
                Ok(l[0])
            }
        }
    }

    fn get_pair(&self) -> Result<(f32, f32)> {
        let nl = self.number_list()?;
        if nl.len() == 2 {
            return Ok((nl[0], nl[1]));
        }
        bail!("Expected exactly two arguments");
    }

    fn get_triple(&self) -> Result<(f32, f32, f32)> {
        let nl = self.number_list()?;
        if nl.len() == 3 {
            return Ok((nl[0], nl[1], nl[2]));
        }
        bail!("Expected exactly three arguments");
    }
}

pub fn eval_function(
    fun: Function,
    args: &ExprValue,
    eval_state: &mut EvalState,
) -> Result<ExprValue> {
    let e = match fun {
        Function::Swap => {
            let (a, b) = args.get_pair()?;
            return Ok(vec![b, a].into());
        }
        Function::Rect2Polar => {
            let (x, y) = args.get_pair()?;
            return Ok(vec![x.hypot(y), y.atan2(x).to_degrees()].into());
        }
        Function::Polar2Rect => {
            let (r, theta) = args.get_pair()?;
            let theta = theta.to_radians();
            return Ok(vec![r * theta.cos(), r * theta.sin()].into());
        }
        Function::Select => {
            let args = args.number_list()?;
            if args.len() < 2 {
                bail!("select() requires at least two arguments");
            }
            let n = args[0] as usize;
            let rest = &args[1..];
            if n < rest.len() {
                rest[n]
            } else {
                bail!("select() index out of range");
            }
        }
        Function::Abs => args.get_only()?.abs(),
        Function::Ceil => args.get_only()?.ceil(),
        Function::Floor => args.get_only()?.floor(),
        Function::Fract => args.get_only()?.fract(),
        Function::Sign => {
            // Can't just use signum since it returns +1 for
            // input of (positive) zero.
            let e = args.get_only()?;
            if e == 0. {
                0.
            } else {
                e.signum()
            }
        }
        Function::Sqrt => args.get_only()?.sqrt(),
        Function::Log => args.get_only()?.ln(),
        Function::Exp => args.get_only()?.exp(),
        Function::Pow => {
            let (x, y) = args.get_pair()?;
            x.powf(y)
        }
        Function::Sin => args.get_only()?.to_radians().sin(),
        Function::Cos => args.get_only()?.to_radians().cos(),
        Function::Tan => args.get_only()?.to_radians().tan(),
        Function::Asin => args.get_only()?.asin().to_degrees(),
        Function::Acos => args.get_only()?.acos().to_degrees(),
        Function::Atan => args.get_only()?.atan().to_degrees(),
        Function::Random => eval_state.context.get_rng().borrow_mut().gen::<f32>(),
        Function::RandInt => {
            let (min, max) = args.get_pair()?;
            let (min, max) = (min as i32, max as i32);
            if min > max {
                bail!("randint(min, max) - `min` must be <= `max`");
            }
            eval_state
                .context
                .get_rng()
                .borrow_mut()
                .gen_range(min..=max) as f32
        }
        Function::Max => args
            .iter()
            .max_by(|a, b| a.total_cmp(b))
            .context("max() requires at least one argument")?,
        Function::Min => args
            .iter()
            .min_by(|a, b| a.total_cmp(b))
            .context("min() requires at least one argument")?,
        Function::Sum => args.iter().sum(),
        Function::Product => args.iter().product(),
        Function::Mean => {
            if args.is_empty() {
                bail!("mean() requires at least one argument");
            }
            let n = args.len() as f32;
            args.iter().sum::<f32>() / n
        }
        Function::Clamp => {
            let (x, min, max) = args.get_triple()?;
            if min > max {
                bail!("clamp(x, min, max) - `min` must be <= `max`");
            }
            x.clamp(min, max)
        }
        Function::Mix => {
            let (a, b, c) = args.get_triple()?;
            a * (1. - c) + b * c
        }
        Function::Equal => {
            let (a, b) = args.get_pair()?;
            if a == b {
                1.
            } else {
                0.
            }
        }
        Function::NotEqual => {
            let (a, b) = args.get_pair()?;
            if a != b {
                1.
            } else {
                0.
            }
        }
        Function::LessThan => {
            let (a, b) = args.get_pair()?;
            if a < b {
                1.
            } else {
                0.
            }
        }
        Function::LessThanEqual => {
            let (a, b) = args.get_pair()?;
            if a <= b {
                1.
            } else {
                0.
            }
        }
        Function::GreaterThan => {
            let (a, b) = args.get_pair()?;
            if a > b {
                1.
            } else {
                0.
            }
        }
        Function::GreaterThanEqual => {
            let (a, b) = args.get_pair()?;
            if a >= b {
                1.
            } else {
                0.
            }
        }
        Function::If => {
            let (cond, a, b) = args.get_triple()?;
            if cond != 0. {
                a
            } else {
                b
            }
        }
        Function::Not => {
            if args.get_only()? == 0. {
                1.
            } else {
                0.
            }
        }
        Function::And => {
            let (a, b) = args.get_pair()?;
            if a != 0. && b != 0. {
                1.
            } else {
                0.
            }
        }
        Function::Or => {
            let (a, b) = args.get_pair()?;
            if a != 0. || b != 0. {
                1.
            } else {
                0.
            }
        }
        Function::Xor => {
            let (a, b) = args.get_pair()?;
            if (a != 0.) ^ (b != 0.) {
                1.
            } else {
                0.
            }
        }
    };
    Ok(e.into())
}
