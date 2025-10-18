/// Recursive descent expression parser
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use super::functions::{eval_function, Function};
use crate::constants::{
    ELREF_ID_PREFIX, EXPR_END, EXPR_START, LOCSPEC_SEP, SCALARSPEC_SEP, VAR_END_BRACE,
    VAR_OPEN_BRACE, VAR_PREFIX,
};
use crate::context::{ContextView, VariableMap};
use crate::errors::{Error, Result};
use crate::geometry::{BoundingBox, LocSpec, ScalarSpec};
use crate::types::{extract_elref, fstr};

#[derive(Debug, Clone, PartialEq)]
pub enum ExprValue {
    Number(f32),
    String(String),
    Text(String),
    List(Vec<ExprValue>),
}

impl From<bool> for ExprValue {
    fn from(v: bool) -> Self {
        Self::Number(if v { 1. } else { 0. })
    }
}

impl From<f32> for ExprValue {
    fn from(v: f32) -> Self {
        Self::Number(v)
    }
}

impl From<Vec<f32>> for ExprValue {
    fn from(v: Vec<f32>) -> Self {
        Self::List(v.into_iter().map(ExprValue::Number).collect())
    }
}

impl From<&[f32]> for ExprValue {
    fn from(v: &[f32]) -> Self {
        Self::List(v.iter().copied().map(ExprValue::Number).collect())
    }
}

impl<const N: usize> From<[f32; N]> for ExprValue {
    fn from(v: [f32; N]) -> Self {
        Self::List(v.iter().copied().map(ExprValue::Number).collect())
    }
}

impl From<Vec<ExprValue>> for ExprValue {
    fn from(v: Vec<ExprValue>) -> Self {
        Self::List(v)
    }
}

impl From<&[ExprValue]> for ExprValue {
    fn from(v: &[ExprValue]) -> Self {
        Self::List(v.to_vec())
    }
}

fn escape(input: &str) -> String {
    let mut result = String::new();
    for ch in input.chars() {
        match ch {
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\'' => result.push_str("\\'"),
            _ => result.push(ch),
        }
    }
    result
}

impl Display for ExprValue {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // Note use of `, ` rather than just ` ` to allow use
        // in contexts which require comma-separated values
        // such as rgb colour: `fill="rgb({{255 * $r, 255 * $g, 255 * $b}})"`
        match self {
            Self::Number(n) => write!(f, "{}", fstr(*n)),
            Self::String(s) => write!(f, "'{}'", escape(s)),
            Self::Text(t) => write!(f, "{t}"),
            Self::List(list) => {
                for (idx, v) in list.iter().enumerate() {
                    if idx > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{v}")?;
                }
                Ok(())
            }
        }
    }
}

impl ExprValue {
    pub fn new() -> Self {
        Self::List(Vec::new())
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Number(_) => 1,
            Self::String(_) | Self::Text(_) => 1,
            Self::List(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// flatten potentially nested list into a single list of ExprValue items
    pub fn flatten(&self) -> Vec<ExprValue> {
        match self {
            Self::Number(_) => vec![self.clone()],
            Self::String(_) | Self::Text(_) => vec![self.clone()],
            Self::List(v) => {
                let mut out = Vec::new();
                for e in v {
                    match e {
                        Self::List(_) => out.extend(e.flatten()),
                        _ => out.push(e.clone()),
                    }
                }
                out
            }
        }
    }

    /// return a pair of values of any type, iff we are a list of exactly two values
    pub fn pair(&self) -> Result<(ExprValue, ExprValue)> {
        if let [a, b] = &self.flatten().as_slice() {
            Ok((a.to_owned(), b.to_owned()))
        } else {
            Err(Error::Arity("expected exactly two arguments".to_owned()))
        }
    }

    /// convert each element to its raw string representation
    pub fn to_string_vec(&self) -> Vec<String> {
        match self {
            Self::Number(n) => vec![fstr(*n)],
            Self::String(s) | Self::Text(s) => vec![s.clone()],
            Self::List(v) => {
                let mut out = Vec::new();
                for e in v {
                    // slight optimisation for single depth list over fully recursive approach
                    match e {
                        Self::Number(n) => out.push(fstr(*n)),
                        Self::String(s) | Self::Text(s) => out.push(s.clone()),
                        _ => out.extend(e.to_string_vec()),
                    }
                }
                out
            }
        }
    }

    /// return list of strings, iff we are a list of string/text values
    pub fn string_list(&self) -> Result<Vec<String>> {
        match self {
            Self::Number(_) => Err(Error::Parse(
                "expected a list of strings, got a number".to_owned(),
            )),
            Self::String(s) | Self::Text(s) => Ok(vec![s.clone()]),
            Self::List(v) => {
                let mut out = Vec::new();
                for e in v {
                    match e {
                        Self::String(s) | Self::Text(s) => out.push(s.clone()),
                        _ => return Err(Error::Parse("expected a list of strings".to_owned())),
                    }
                }
                Ok(out)
            }
        }
    }

    /// return single string, iff we are a single string/text value
    pub fn one_string(&self) -> Result<String> {
        if let [nl] = &self.string_list()?.as_slice() {
            Ok(nl.clone())
        } else {
            Err(Error::Arity("expected a single string argument".to_owned()))
        }
    }

    /// return pair of strings, iff we are a list of exactly two string/text values
    pub fn string_pair(&self) -> Result<(String, String)> {
        if let [a, b] = &self.string_list()?.as_slice() {
            Ok((a.clone(), b.clone()))
        } else {
            Err(Error::Arity(
                "expected exactly two string arguments".to_owned(),
            ))
        }
    }

    /// return list of numbers, iff we are a list of numeric values
    pub fn number_list(&self) -> Result<Vec<f32>> {
        match self {
            Self::Number(v) => Ok(vec![*v]),
            Self::String(s) | Self::Text(s) => Err(Error::Parse(format!(
                "expected a list of numbers, got '{s}'"
            ))),
            Self::List(v) => {
                let mut out = Vec::new();
                for e in v {
                    if let Self::Number(n) = e {
                        out.push(*n);
                    } else {
                        return Err(Error::Parse("expected a list of numbers".to_owned()));
                    }
                }
                Ok(out)
            }
        }
    }

    pub fn one_number(&self) -> Result<f32> {
        if let [a] = self.number_list()?.as_slice() {
            Ok(*a)
        } else {
            Err(Error::Arity(
                "expected a single numeric argument".to_owned(),
            ))
        }
    }

    pub fn number_pair(&self) -> Result<(f32, f32)> {
        if let [a, b] = self.number_list()?.as_slice() {
            Ok((*a, *b))
        } else {
            Err(Error::Arity(
                "expected exactly two numeric arguments".to_owned(),
            ))
        }
    }

    pub fn number_triple(&self) -> Result<(f32, f32, f32)> {
        if let [a, b, c] = self.number_list()?.as_slice() {
            Ok((*a, *b, *c))
        } else {
            Err(Error::Arity(
                "expected exactly three numeric arguments".to_owned(),
            ))
        }
    }

    pub fn one_bbox(&self) -> Result<BoundingBox> {
        if let [a, b, c, d] = self.number_list()?.as_slice() {
            Ok(BoundingBox::new(*a, *b, *c, *d))
        } else {
            Err(Error::Arity(
                "expected exactly four numeric arguments".to_owned(),
            ))
        }
    }

    pub fn bbox_pair(&self) -> Result<(BoundingBox, BoundingBox)> {
        if let [a1, a2, a3, a4, b1, b2, b3, b4] = self.number_list()?.as_slice() {
            Ok((
                BoundingBox::new(*a1, *a2, *a3, *a4),
                BoundingBox::new(*b1, *b2, *b3, *b4),
            ))
        } else {
            Err(Error::Arity(
                "expected exactly eight numeric arguments".to_owned(),
            ))
        }
    }

    pub fn bbox_list(&self) -> Result<Vec<BoundingBox>> {
        let args = self.number_list()?;
        if args.len() % 4 != 0 {
            return Err(Error::Arity(
                "expected a multiple of four numeric arguments".to_owned(),
            ));
        }
        let mut out = Vec::new();
        for b in args.chunks_exact(4) {
            out.push(BoundingBox::new(b[0], b[1], b[2], b[3]));
        }
        Ok(out)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ComparisonOp {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
}

impl FromStr for ComparisonOp {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "eq" => Ok(Self::Eq),
            "ne" => Ok(Self::Ne),
            "gt" => Ok(Self::Gt),
            "ge" => Ok(Self::Ge),
            "lt" => Ok(Self::Lt),
            "le" => Ok(Self::Le),
            _ => Err(Error::Parse(format!("invalid comparison op '{s}'"))),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LogicalOp {
    And,
    Or,
    Xor,
}

impl FromStr for LogicalOp {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "and" => Ok(Self::And),
            "or" => Ok(Self::Or),
            "xor" => Ok(Self::Xor),
            _ => Err(Error::Parse(format!("invalid logical op '{s}'"))),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Token {
    /// A numeric literal
    Number(f32),
    /// A variable reference, beginning with '$'
    Var(String),
    /// Reference to an element-derived value
    ElementRef(String),
    /// String surrounded by single or double quotes
    String(String),
    /// Symbol - used for alphanumeric operators & function identifiers
    Symbol(String),
    /// A literal '('
    OpenParen,
    /// A literal ')'
    CloseParen,
    /// A literal ',' - used for separating function arguments or expressions
    Comma,
    /// A literal '+' for addition
    Add,
    /// A literal '-' for subtraction or unary minus
    Sub,
    /// A literal '*' for multiplication
    Mul,
    /// A literal '/' for division
    Div,
    /// A literal '//' for integer division
    IntDiv,
    /// A literal '%' for mod operation
    Mod,
    /// Internal-only token used for separating otherwise
    /// indistinguishable tokens. (Tabs & spaces).
    Whitespace,
    /// Internal-only token for collecting characters for use in
    /// `Number`, `Var` or `ElementRef` variants.
    Other,
}

fn valid_variable_name(var: &str) -> Result<&str> {
    if !var.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return Err(Error::Parse(format!("invalid variable name '{var}'")));
    }
    if !var[1..]
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(Error::Parse(format!("invalid variable name '{var}'")));
    }
    Ok(var)
}

pub(super) fn valid_symbol(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && s.chars()
            .skip(1)
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Return inner text of a quoted string, or None if not a quoted string
fn extract_string(s: &str) -> Option<String> {
    let first = s.chars().next().unwrap_or(' ');
    let last = s.chars().last().unwrap_or(' ');
    if (first == '\'' || first == '"') && first == last {
        // strip matching surrounding quote char
        let inner = &s[1..s.len() - 1];
        Some(inner.to_owned())
    } else {
        None
    }
}

fn tokenize_atom(input: &str) -> Result<Token> {
    if let Some(input) = input.strip_prefix(VAR_PREFIX) {
        let var_name = if let Some(input) = input.strip_prefix(VAR_OPEN_BRACE) {
            input.strip_suffix(VAR_END_BRACE)
        } else {
            Some(input)
        };
        if let Some(var) = var_name {
            valid_variable_name(var).map(|v| Token::Var(v.to_string()))
        } else {
            Err(Error::Parse(format!("missing closing brace in '{input}'")))
        }
    } else if let Some(content) = extract_string(input) {
        // using delimited-atoms strings (e.g. `["hello world"]`) allows
        // unescaped quote chars within the string
        Ok(Token::String(content))
    } else if extract_elref(input).is_ok() {
        Ok(Token::ElementRef(input.to_owned()))
    } else if let Ok(num) = input.parse::<f32>() {
        Ok(Token::Number(num))
    } else if valid_symbol(input) {
        Ok(Token::Symbol(input.to_owned()))
    } else {
        Err(Error::Parse(format!("unexpected token '{input}'")))
    }
}

pub(super) fn tokenize(input: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut buffer = Vec::new();
    // hack to allow '-' in id-based element references
    let mut in_elref_id = false;
    let mut in_quote = None;
    // expr fragments contained in [...] are collected and treated as atoms
    // rather than being tokenized further. This allows element references
    // containing operator characters such as '+', and strings with unescaped
    // quote chars.
    let mut in_delimited_atom = false;

    let mut string_escape = false;
    let mut atom_escape = false;
    for ch in input.chars() {
        if in_delimited_atom {
            match (ch, atom_escape) {
                (']', false) => {
                    in_delimited_atom = false;
                    atom_escape = false;
                    let buffer_token = tokenize_atom(&buffer.iter().collect::<String>())?;
                    buffer.clear();
                    tokens.push(buffer_token);
                }
                ('\\', false) => {
                    atom_escape = true;
                }
                _ => {
                    buffer.push(ch);
                    atom_escape = false;
                }
            }
            continue;
        }
        if let Some(qt) = in_quote {
            // Avoid considering other tokens within a string
            // Strings are surrounded with either ' or " and may contain
            // escaped quotes, newlines as '\n', and backslashes as '\\'.
            match (ch, string_escape) {
                (x, false) if x == qt => {
                    tokens.push(Token::String(buffer.iter().collect::<String>()));
                    buffer.clear();
                    in_quote = None;
                    string_escape = false;
                }
                ('\\', false) => {
                    string_escape = true;
                }
                ('n', true) => {
                    buffer.push('\n');
                    string_escape = false;
                }
                _ => {
                    buffer.push(ch);
                    string_escape = false;
                }
            }
            continue;
        }
        let next_token = match ch {
            '[' => {
                in_delimited_atom = true;
                continue;
            }
            '(' => Token::OpenParen,
            ')' => Token::CloseParen,
            '+' => Token::Add,
            '-' if !in_elref_id => Token::Sub, // '-' is valid in an ElRef::Id
            '*' => Token::Mul,
            '/' => {
                if buffer.is_empty() && tokens.last() == Some(&Token::Div) {
                    tokens.pop();
                    Token::IntDiv
                } else {
                    Token::Div
                }
            }
            '%' => Token::Mod,
            ',' => Token::Comma,
            ' ' | '\t' => Token::Whitespace,
            '\'' | '"' => {
                in_quote = Some(ch);
                continue;
            }
            ELREF_ID_PREFIX => {
                in_elref_id = true;
                Token::Other
            }
            _ => Token::Other,
        };
        if next_token == Token::Other {
            buffer.push(ch);
        } else {
            in_elref_id = false;
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
        if in_quote.is_some() {
            return Err(Error::Parse(format!("missing closing quote in '{input}'")));
        }
        let buffer_token = tokenize_atom(&buffer.iter().collect::<String>())?;
        buffer.clear();
        tokens.push(buffer_token);
    }
    Ok(tokens)
}

pub struct EvalState<'a> {
    tokens: Vec<Token>,
    index: usize,
    pub context: &'a dyn ContextView,
    // Used to check for circular variable references
    // Vec - likely to be few vars, and need stack behaviour
    checked_vars: Vec<String>,
}

impl<'a> EvalState<'a> {
    pub(super) fn new(
        tokens: impl IntoIterator<Item = Token>,
        context: &'a dyn ContextView,
        checked_vars: &[String],
    ) -> Self {
        Self {
            tokens: tokens.into_iter().collect(),
            index: 0,
            context,
            checked_vars: Vec::from(checked_vars),
        }
    }

    /// Peek the next token without advancing
    pub(super) fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    /// Peek the previous token
    fn prev(&self) -> Option<&Token> {
        if self.index == 0 {
            return None;
        }
        self.tokens.get(self.index - 1)
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
        if self.peek() == Some(&token) {
            self.advance();
            Ok(())
        } else {
            Err(Error::Parse(format!(
                "expected token '{token:?}' (got '{:?}')",
                self.peek()
            )))
        }
    }

    fn lookup(&mut self, v: &str) -> Result<ExprValue> {
        if self.checked_vars.iter().any(|var| var == v) {
            return Err(Error::CircularRef(v.to_owned()));
        }
        self.checked_vars.push(v.to_string());
        let result = if let Some(inner) = self.context.get_var(v) {
            let tokens = tokenize(&inner)?;
            if tokens.is_empty() {
                Ok(ExprValue::List(Vec::new()))
            } else {
                let mut es = EvalState::new(tokens, self.context, &self.checked_vars);
                let e = expr_list(&mut es)?;
                if es.peek().is_none() {
                    Ok(e)
                } else {
                    return Err(Error::Parse(format!(
                        "unexpected trailing tokens evaluating '{v}'"
                    )));
                }
            }
        } else {
            return Err(Error::Parse(format!("could not evaluate variable '{v}'")));
        };
        // Need this to allow e.g. "$var + $var"
        self.checked_vars.pop();
        result
    }

    /// Generate a scalar, coordinate, or bbox from an element reference
    ///
    /// Examples:
    ///  `#abc~h` - `h` height of element #abc
    ///  `#abc@bl` - `x, y` coord of bottom left corner of element #abc
    ///  `#abc` - `x1, y1, x2, y2` bounding box of element #abc
    fn element_ref(&self, v: &str) -> Result<ExprValue> {
        let (elref, remain) = extract_elref(v)?;
        let elem = self
            .context
            .get_element(&elref)
            .ok_or_else(|| Error::Reference(elref))?;
        let bb = self
            .context
            .get_element_bbox(elem)?
            .ok_or_else(|| Error::MissingBBox(elem.to_string()))?;
        if remain.is_empty() {
            return Ok([bb.x1, bb.y1, bb.x2, bb.y2].into());
        } else if let Some(ss) = remain.strip_prefix(SCALARSPEC_SEP) {
            return Ok(bb.scalarspec(ScalarSpec::from_str(ss)?).into());
        } else if let Some(ls) = remain.strip_prefix(LOCSPEC_SEP) {
            let (x, y) = bb.locspec(LocSpec::from_str(ls)?);
            return Ok([x, y].into());
        }
        Err(Error::Parse(format!(
            "expected locspec or scalarspec: '{v}'"
        )))
    }
}

fn evaluate(
    tokens: impl IntoIterator<Item = Token> + std::fmt::Debug + Clone,
    context: &impl ContextView,
) -> Result<ExprValue> {
    // This just forwards with initial empty checked_vars
    evaluate_inner(tokens, context, &[])
}

fn evaluate_inner(
    tokens: impl IntoIterator<Item = Token> + std::fmt::Debug + Clone,
    context: &impl ContextView,
    checked_vars: &[String],
) -> Result<ExprValue> {
    let mut eval_state = EvalState::new(tokens.clone(), context, checked_vars);
    let e = expr_list(&mut eval_state)?;
    if eval_state.peek().is_none() {
        Ok(e)
    } else {
        Err(Error::Parse(format!(
            "unexpected trailing tokens: {tokens:?}"
        )))
    }
}

fn expr_list(eval_state: &mut EvalState) -> Result<ExprValue> {
    let mut out: Vec<ExprValue> = Vec::new();
    if let (Some(Token::OpenParen), Some(Token::CloseParen)) =
        (eval_state.prev(), eval_state.peek())
    {
        // Support empty expr_list for function calls
        return Ok(out.into());
    }
    loop {
        let e = expr(eval_state)?;
        out.extend(e.flatten());
        match eval_state.peek() {
            Some(Token::Comma) => {
                eval_state.advance();
            }
            _ => {
                break;
            }
        }
    }
    Ok(out.into())
}

pub(super) fn expr(eval_state: &mut EvalState) -> Result<ExprValue> {
    logical(eval_state)
}

// Handle `or`, `and`, 'xor' operators, all left-to-right associative
fn logical(eval_state: &mut EvalState) -> Result<ExprValue> {
    let mut e = comparison(eval_state)?;
    while let Some(Token::Symbol(s)) = eval_state.peek() {
        match s.parse::<LogicalOp>() {
            Ok(logop) => {
                eval_state.advance();
                // Note: in order to avoid leaving tokens un-pulled, we don't
                // short-circuit evaluation of logical operators.
                let other = comparison(eval_state)?.one_number()?;
                e = match logop {
                    LogicalOp::And => ((e.one_number()? != 0.) && (other != 0.)).into(),
                    LogicalOp::Or => ((e.one_number()? != 0.) || (other != 0.)).into(),
                    LogicalOp::Xor => ((e.one_number()? != 0.) != (other != 0.)).into(),
                };
            }
            _ => break,
        }
    }
    Ok(e)
}

fn comparison(eval_state: &mut EvalState) -> Result<ExprValue> {
    let t = term(eval_state)?;
    if let Ok(mut first) = t.one_number() {
        if let Some(Token::Symbol(s)) = eval_state.peek().cloned() {
            if let Ok(op) = s.parse::<ComparisonOp>() {
                eval_state.advance();
                let second = term(eval_state)?.one_number()?;
                let comp = match op {
                    ComparisonOp::Eq => first == second,
                    ComparisonOp::Ne => first != second,
                    ComparisonOp::Gt => first > second,
                    ComparisonOp::Ge => first >= second,
                    ComparisonOp::Lt => first < second,
                    ComparisonOp::Le => first <= second,
                };
                first = comp as i32 as f32;
            }
        }
        Ok(first.into())
    } else {
        Ok(t)
    }
}

fn term(eval_state: &mut EvalState) -> Result<ExprValue> {
    let t = factor(eval_state)?;
    if let Ok(mut e) = t.one_number() {
        loop {
            match eval_state.peek() {
                Some(Token::Add) => {
                    eval_state.advance();
                    e += factor(eval_state)?.one_number()?;
                }
                Some(Token::Sub) => {
                    eval_state.advance();
                    e -= factor(eval_state)?.one_number()?;
                }
                _ => {
                    break;
                }
            }
        }
        Ok(e.into())
    } else {
        Ok(t)
    }
}

fn factor(eval_state: &mut EvalState) -> Result<ExprValue> {
    let f = primary(eval_state)?;
    if let Ok(mut e) = f.one_number() {
        loop {
            match eval_state.peek() {
                Some(Token::Mul) => {
                    eval_state.advance();
                    e *= primary(eval_state)?.one_number()?;
                }
                Some(Token::Div) => {
                    eval_state.advance();
                    e /= primary(eval_state)?.one_number()?;
                }
                Some(Token::IntDiv) => {
                    eval_state.advance();
                    e = e.div_euclid(primary(eval_state)?.one_number()?);
                }
                Some(Token::Mod) => {
                    eval_state.advance();
                    // note euclid remainder rather than '%' operator
                    // to ensure positive result useful for indexing
                    e = e.rem_euclid(primary(eval_state)?.one_number()?);
                }
                _ => {
                    break;
                }
            }
        }
        Ok(e.into())
    } else {
        Ok(f)
    }
}

fn primary(eval_state: &mut EvalState) -> Result<ExprValue> {
    match eval_state.next() {
        Some(Token::Number(x)) => Ok(ExprValue::Number(x)),
        Some(Token::String(s)) => Ok(ExprValue::String(s)),
        Some(Token::Var(v)) => eval_state.lookup(&v),
        Some(Token::ElementRef(v)) => eval_state.element_ref(&v),
        Some(Token::OpenParen) => {
            let e = expr_list(eval_state)?;
            eval_state.require(Token::CloseParen)?;
            Ok(e)
        }
        // unary minus
        Some(Token::Sub) => Ok(ExprValue::Number(-primary(eval_state)?.one_number()?)),
        Some(Token::Symbol(fun)) => {
            let fun = fun.parse::<Function>()?;
            eval_state.require(Token::OpenParen)?;
            let args = expr_list(eval_state)?;
            let e = eval_function(fun, &args, eval_state)?;
            eval_state.require(Token::CloseParen)?;
            Ok(e)
        }
        Some(tok) => Err(Error::Parse(format!("invalid token in primary(): {tok:?}"))),
        None => Err(Error::Parse("unexpected end of input".to_owned())),
    }
}

/// Convert unescaped '$var' or '${var}' in given input according
/// to the supplied variables. Missing variables are left as-is.
pub fn eval_vars(value: &str, context: &impl VariableMap) -> String {
    let mut result = String::new();
    let mut value = value;
    while !value.is_empty() {
        if let Some(idx) = value.find(VAR_PREFIX) {
            let (prefix, remain) = value.split_at(idx);
            if let Some(esc_prefix) = prefix.strip_prefix('\\') {
                // Escaped '$'; ignore '\' and add '$' to result
                result.push_str(esc_prefix);
                result.push(VAR_PREFIX);
                value = &remain[1..];
                continue;
            }
            result.push_str(prefix);
            let remain = &remain[1..]; // skip '$'
            if let Some(inner) = remain.strip_prefix(VAR_OPEN_BRACE) {
                value = inner;
                if let Some(end_idx) = value.find(VAR_END_BRACE) {
                    let inner = &value[..end_idx];
                    if let Some(value) = context.get_var(inner) {
                        result.push_str(&value);
                    } else {
                        result.push(VAR_PREFIX);
                        result.push(VAR_OPEN_BRACE);
                        result.push_str(inner);
                        result.push(VAR_END_BRACE);
                    }
                    value = &value[end_idx + 1..]; // skip '}'
                } else {
                    result.push(VAR_PREFIX);
                    result.push(VAR_OPEN_BRACE);
                    result.push_str(value);
                    break;
                }
            } else {
                // un-braced reference; consume until non-alphanumeric/_ character
                if let Some(idx) = remain.find(|c: char| !(c.is_alphanumeric() || c == '_')) {
                    let (var, remain) = remain.split_at(idx);
                    if let Some(value) = context.get_var(var) {
                        result.push_str(&value);
                    } else {
                        result.push(VAR_PREFIX);
                        result.push_str(var);
                    }
                    value = remain;
                } else {
                    if let Some(value) = context.get_var(remain) {
                        result.push_str(&value);
                    } else {
                        result.push(VAR_PREFIX);
                        result.push_str(remain);
                    }
                    break;
                }
            }
        } else {
            result.push_str(value);
            break;
        }
    }
    result
}

/// Expand arithmetic expressions (including numeric variable lookup) in {{...}}
pub(super) fn eval_expr(value: &str, context: &impl ContextView) -> Result<String> {
    // Note - must catch "{{a}} {{b}}" as 'a' & 'b', rather than 'a}} {{b'
    let mut result = String::new();
    let mut value = value;
    loop {
        if let Some(idx) = value.find(EXPR_START) {
            result.push_str(&value[..idx]);
            value = &value[idx + EXPR_START.len()..];
            if let Some(end_idx) = value.find(EXPR_END) {
                let inner = &value[..end_idx];
                result.push_str(&eval_str(inner, context)?);
                value = &value[end_idx + EXPR_END.len()..];
            } else {
                result.push_str(value);
                break;
            }
        } else {
            result.push_str(value);
            break;
        }
    }
    Ok(result)
}

/// Evaluate an expression.
fn eval_str(value: &str, context: &impl ContextView) -> Result<String> {
    tokenize(value)
        .and_then(|tokens| evaluate(tokens, context))
        .map(|v| v.to_string())
}

/// Evaluate attribute value including {{arithmetic}} and ${variable} expressions
pub fn eval_attr(value: &str, context: &impl ContextView) -> Result<String> {
    let mut orig_value = value.to_string();
    let mut value = orig_value.clone();

    // Repeat until we've had 10 passes or it didn't change.
    // This is to allow for nested variables, e.g. "$a + $b" where $a or $b
    // could be a variable that contains an expression, or "$$a" where $a is
    // the name of another variable.
    //
    // TODO: would be better to move repeated evaluation into the recursive descent
    // parser and support 'indirect' variables only in eval_expr(); currently tokenisation
    // gets confused by e.g. "$$$a".
    for _ in 0..10 {
        // Step 1: Replace variables (which may contain element references, for example).
        // Note this is only a single pass, so variables could potentially reference other
        // variables which are resolved in eval_expr - provided they hold numeric values.
        value = eval_vars(&value, context);
        // Step 2: Evaluate expressions, which could fail with e.g. ReferenceError
        value = eval_expr(&value, context)?;

        if value == orig_value {
            // No change, so we can stop
            break;
        }

        // Update value for next iteration
        orig_value = value.clone();
    }

    Ok(value)
}

/// Evaluate a condition expression, returning true iff the result is non-zero
pub fn eval_condition(value: &str, context: &impl ContextView) -> Result<bool> {
    // Conditions don't need surrounding by {{...}} since they always evaluate to
    // a single numeric expression, but allow for consistency with other attr values.
    let mut value = value;
    if let Some(inner) = value.strip_prefix(EXPR_START) {
        value = inner
            .strip_suffix(EXPR_END)
            .ok_or_else(|| Error::Parse(format!("expected closing '{EXPR_END}': '{value}'")))?;
    }
    eval_str(value, context)?
        .parse::<f32>()
        .map(|v| v != 0.)
        .map_err(|_| Error::Parse(format!("invalid condition: '{value}'")))
}

pub fn eval_list(value: &str, context: &impl ContextView) -> Result<Vec<String>> {
    // Lists don't need surrounding by {{...}} since they always evaluate to
    // a list of Strings, but allow for consistency with other attr values.
    let mut value = value;
    if let Some(inner) = value.strip_prefix(EXPR_START) {
        value = inner
            .strip_suffix(EXPR_END)
            .ok_or_else(|| Error::Parse(format!("expected closing '{EXPR_END}': '{value}'")))?;
    }
    let tokens = tokenize(value)?;
    Ok(evaluate(tokens, context)?.to_string_vec())
}
