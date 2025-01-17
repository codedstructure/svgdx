/// Recursive descent expression parser
use std::fmt::{self, Display, Formatter};

use itertools::Itertools;

use crate::context::{ContextView, VariableMap};
use crate::errors::{Result, SvgdxError};
use crate::functions::{eval_function, Function};
use crate::position::parse_el_scalar;
use crate::types::fstr;

#[derive(Debug, Clone, PartialEq)]
pub enum ExprValue {
    Number(f32),
    String(String),
    Text(String),
    List(Vec<ExprValue>),
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
            Self::Text(t) => write!(f, "{}", t),
            Self::List(_) => {
                write!(f, "{}", self.flatten().into_iter().join(", "))
            }
        }
    }
}

impl ExprValue {
    pub fn empty() -> Self {
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

    pub fn pair(&self) -> Result<(ExprValue, ExprValue)> {
        let nl = self.flatten();
        if nl.len() == 2 {
            return Ok((nl[0].to_owned(), nl[1].to_owned()));
        }
        Err(SvgdxError::ParseError(
            "Expected exactly two arguments".to_owned(),
        ))
    }

    pub fn string_list(&self) -> Result<Vec<String>> {
        match self {
            Self::Number(_) => Err(SvgdxError::ParseError(
                "Expected a list of strings".to_owned(),
            )),
            Self::String(s) | Self::Text(s) => Ok(vec![s.clone()]),
            Self::List(v) => {
                let mut out = Vec::new();
                for e in v {
                    if let Self::String(s) = e {
                        out.push(s.clone());
                    } else {
                        return Err(SvgdxError::ParseError(
                            "Expected a list of strings".to_owned(),
                        ));
                    }
                }
                Ok(out)
            }
        }
    }

    pub fn one_string(&self) -> Result<String> {
        match self {
            Self::Number(_) => Err(SvgdxError::ParseError(
                "Expected a string, got a number".to_owned(),
            )),
            Self::String(s) | Self::Text(s) => Ok(s.clone()),
            Self::List(l) => {
                if l.len() != 1 {
                    return Err(SvgdxError::ParseError(
                        "Expected exactly one argument".to_owned(),
                    ));
                }
                if let Self::String(s) = &l[0] {
                    Ok(s.clone())
                } else {
                    Err(SvgdxError::ParseError(
                        "Expected a single string argument".to_owned(),
                    ))
                }
            }
        }
    }

    pub fn string_pair(&self) -> Result<(String, String)> {
        let nl = self.string_list()?;
        if nl.len() == 2 {
            return Ok((nl[0].clone(), nl[1].clone()));
        }
        Err(SvgdxError::ParseError(
            "Expected exactly two arguments".to_owned(),
        ))
    }

    pub fn number_list(&self) -> Result<Vec<f32>> {
        match self {
            Self::Number(v) => Ok(vec![*v]),
            Self::String(s) | Self::Text(s) => Err(SvgdxError::ParseError(format!(
                "Expected a list of numbers, got '{}'",
                s
            ))),
            Self::List(v) => {
                let mut out = Vec::new();
                for e in v {
                    if let Self::Number(n) = e {
                        out.push(*n);
                    } else {
                        return Err(SvgdxError::ParseError(
                            "Expected a list of numbers".to_owned(),
                        ));
                    }
                }
                Ok(out)
            }
        }
    }

    pub fn one_number(&self) -> Result<f32> {
        match self {
            ExprValue::Number(n) => Ok(*n),
            ExprValue::String(s) | ExprValue::Text(s) => Err(SvgdxError::ParseError(format!(
                "Expected a number, got '{}'",
                s
            ))),
            ExprValue::List(l) => {
                if l.len() != 1 {
                    return Err(SvgdxError::ParseError(
                        "Expected exactly one argument".to_owned(),
                    ));
                }
                if let ExprValue::Number(n) = &l[0] {
                    Ok(*n)
                } else {
                    Err(SvgdxError::ParseError(
                        "Expected a single numeric argument".to_owned(),
                    ))
                }
            }
        }
    }

    pub fn number_pair(&self) -> Result<(f32, f32)> {
        let nl = self.number_list()?;
        if nl.len() == 2 {
            return Ok((nl[0], nl[1]));
        }
        Err(SvgdxError::ParseError(
            "Expected exactly two arguments".to_owned(),
        ))
    }

    pub fn number_triple(&self) -> Result<(f32, f32, f32)> {
        let nl = self.number_list()?;
        if nl.len() == 3 {
            return Ok((nl[0], nl[1], nl[2]));
        }
        Err(SvgdxError::ParseError(
            "Expected exactly three arguments".to_owned(),
        ))
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    /// A numeric literal
    Number(f32),
    /// A variable reference, beginning with '$'
    Var(String),
    /// Reference to an element-derived value, beginning with '#'
    ElementRef(String),
    /// String surrounded by single or double quotes
    String(String),
    /// A function reference
    FnRef(Function),
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
        return Err(SvgdxError::ParseError("Invalid variable name".to_owned()));
    }
    if !var[1..]
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(SvgdxError::ParseError("Invalid variable name".to_owned()));
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
            Err(SvgdxError::ParseError("Invalid variable".to_owned()))
        }
    } else if input.starts_with(['#', '^']) {
        Ok(Token::ElementRef(input.to_owned()))
    } else if let Ok(func) = input.parse() {
        Ok(Token::FnRef(func))
    } else {
        Ok(input
            .parse::<f32>()
            .ok()
            .map(Token::Number)
            .ok_or_else(|| {
                SvgdxError::ParseError(format!("Invalid number or function '{input}"))
            })?)
    }
}

fn tokenize(input: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut buffer = Vec::new();
    // hack to allow '-' in id-based element references
    let mut in_elref_id = false;
    let mut in_quote = None;

    let mut string_escape = false;
    for ch in input.chars() {
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
            '(' => Token::OpenParen,
            ')' => Token::CloseParen,
            '+' => Token::Add,
            '-' if !in_elref_id => Token::Sub, // '-' is valid in an ElRef::Id
            '*' => Token::Mul,
            '/' => Token::Div,
            '%' => Token::Mod,
            ',' => Token::Comma,
            ' ' | '\t' => Token::Whitespace,
            '\'' | '"' => {
                in_quote = Some(ch);
                continue;
            }
            '#' => {
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
    fn new(
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
    fn peek(&self) -> Option<&Token> {
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
            Err(SvgdxError::ParseError(format!(
                "Expected token '{token:?}' not matched (got '{:?}')",
                self.peek()
            )))
        }
    }

    fn lookup(&mut self, v: &str) -> Result<ExprValue> {
        if self.checked_vars.iter().contains(&String::from(v)) {
            return Err(SvgdxError::CircularRefError(v.to_owned()));
        }
        self.checked_vars.push(v.to_string());
        let result = if let Some(inner) = self.context.get_var(v) {
            let tokens = tokenize(&inner)?;
            if tokens.is_empty() {
                Ok(ExprValue::List(Vec::new()))
            } else {
                let mut es = EvalState::new(tokens, self.context, &self.checked_vars);
                let e = expr_list(&mut es);
                if es.peek().is_none() {
                    e
                } else {
                    return Err(SvgdxError::ParseError(
                        "Unexpected trailing tokens".to_owned(),
                    ));
                }
            }
        } else {
            return Err(SvgdxError::ParseError(
                "Could not evaluate variable".to_owned(),
            ));
        };
        // Need this to allow e.g. "$var + $var"
        self.checked_vars.pop();
        result
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
    fn element_ref(&self, v: &str) -> Result<ExprValue> {
        // TODO: perhaps this should be in the SvgElement impl, so it can
        // be re-used by other single-value attribute references, e.g.
        // <line x1="#abc.l" .../>
        if let Ok((elref, Some(scalar))) = parse_el_scalar(v) {
            if let Some(elem) = self.context.get_element(&elref) {
                if let Some(bb) = self.context.get_element_bbox(elem)? {
                    Ok(bb.scalarspec(scalar).into())
                } else {
                    Err(SvgdxError::MissingBoundingBox(elem.to_string()))
                }
            } else {
                Err(SvgdxError::ReferenceError(elref))
            }
        } else {
            Err(SvgdxError::ParseError(format!("Invalid element_ref: {v}")))
        }
    }
}

fn evaluate(
    tokens: impl IntoIterator<Item = Token>,
    context: &impl ContextView,
) -> Result<ExprValue> {
    // This just forwards with initial empty checked_vars
    evaluate_inner(tokens, context, &[])
}

fn evaluate_inner(
    tokens: impl IntoIterator<Item = Token>,
    context: &impl ContextView,
    checked_vars: &[String],
) -> Result<ExprValue> {
    let mut eval_state = EvalState::new(tokens, context, checked_vars);
    let e = expr_list(&mut eval_state);
    if eval_state.peek().is_none() {
        e
    } else {
        Err(SvgdxError::ParseError(
            "Unexpected trailing tokens".to_owned(),
        ))
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

fn expr(eval_state: &mut EvalState) -> Result<ExprValue> {
    let t = term(eval_state)?;
    if let Ok(mut e) = t.one_number() {
        loop {
            match eval_state.peek() {
                Some(Token::Add) => {
                    eval_state.advance();
                    e += term(eval_state)?.one_number()?;
                }
                Some(Token::Sub) => {
                    eval_state.advance();
                    e -= term(eval_state)?.one_number()?;
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

fn term(eval_state: &mut EvalState) -> Result<ExprValue> {
    let f = factor(eval_state)?;
    if let Ok(mut e) = f.one_number() {
        loop {
            match eval_state.peek() {
                Some(Token::Mul) => {
                    eval_state.advance();
                    e *= factor(eval_state)?.one_number()?;
                }
                Some(Token::Div) => {
                    eval_state.advance();
                    e /= factor(eval_state)?.one_number()?;
                }
                Some(Token::Mod) => {
                    eval_state.advance();
                    // note euclid remainder rather than '%' operator
                    // to ensure positive result useful for indexing
                    e = e.rem_euclid(factor(eval_state)?.one_number()?);
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

fn factor(eval_state: &mut EvalState) -> Result<ExprValue> {
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
        Some(Token::Sub) => Ok(ExprValue::Number(-factor(eval_state)?.one_number()?)),
        Some(Token::FnRef(fun)) => {
            eval_state.require(Token::OpenParen)?;
            let args = expr_list(eval_state)?;
            let e = eval_function(fun, &args, eval_state)?;
            eval_state.require(Token::CloseParen)?;
            Ok(e)
        }
        _ => Err(SvgdxError::ParseError(
            "Invalid token in factor()".to_owned(),
        )),
    }
}

/// Convert unescaped '$var' or '${var}' in given input according
/// to the supplied variables. Missing variables are left as-is.
pub fn eval_vars(value: &str, context: &impl VariableMap) -> String {
    let mut result = String::new();
    let mut value = value;
    while !value.is_empty() {
        if let Some(idx) = value.find('$') {
            let (prefix, remain) = value.split_at(idx);
            if let Some(esc_prefix) = prefix.strip_prefix('\\') {
                // Escaped '$'; ignore '\' and add '$' to result
                result.push_str(esc_prefix);
                result.push('$');
                value = &remain[1..];
                continue;
            }
            result.push_str(prefix);
            let remain = &remain[1..]; // skip '$'
            if let Some(inner) = remain.strip_prefix('{') {
                value = inner;
                if let Some(end_idx) = value.find('}') {
                    let inner = &value[..end_idx];
                    if let Some(value) = context.get_var(inner) {
                        result.push_str(&value);
                    } else {
                        result.push_str("${");
                        result.push_str(inner);
                        result.push('}');
                    }
                    value = &value[end_idx + 1..]; // skip '}'
                } else {
                    result.push_str("${");
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
                        result.push('$');
                        result.push_str(var);
                    }
                    value = remain;
                } else {
                    if let Some(value) = context.get_var(remain) {
                        result.push_str(&value);
                    } else {
                        result.push('$');
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
fn eval_expr(value: &str, context: &impl ContextView) -> String {
    // Note - must catch "{{a}} {{b}}" as 'a' & 'b', rather than 'a}} {{b'
    let mut result = String::new();
    let mut value = value;
    loop {
        if let Some(idx) = value.find("{{") {
            result.push_str(&value[..idx]);
            value = &value[idx + 2..];
            if let Some(end_idx) = value.find("}}") {
                let inner = &value[..end_idx];
                result.push_str(&eval_str(inner, context));
                value = &value[end_idx + 2..];
            } else {
                result.push_str(value);
                break;
            }
        } else {
            result.push_str(value);
            break;
        }
    }
    result
}

fn eval_str(value: &str, context: &impl ContextView) -> String {
    if let Ok(tokens) = tokenize(value) {
        if let Ok(parsed) = evaluate(tokens, context) {
            parsed.to_string()
        } else {
            value.to_owned()
        }
    } else {
        value.to_owned()
    }
}

/// Evaluate attribute value including {{arithmetic}} and ${variable} expressions
pub fn eval_attr(value: &str, context: &impl ContextView) -> String {
    // Step 1: Replace variables (which may contain element references, for example).
    // Note this is only a single pass, so variables could potentially reference other
    // variables which are resolved in eval_expr - provided they hold numeric values.
    let value = eval_vars(value, context);
    // Step 2: Evaluate arithmetic expressions.
    eval_expr(&value, context)
}

/// Evaluate a condition expression, returning true iff the result is non-zero
pub fn eval_condition(value: &str, context: &impl ContextView) -> Result<bool> {
    // Conditions don't need surrounding by {{...}} since they always evaluate to
    // a single numeric expression, but allow for consistency with other attr values.
    let mut value = value;
    if let Some(inner) = value.strip_prefix("{{") {
        value = inner
            .strip_suffix("}}")
            .ok_or(SvgdxError::ParseError(format!(
                "Expected closing '}}': '{value}'"
            )))?;
    }
    eval_str(value, context)
        .parse::<f32>()
        .map(|v| v != 0.)
        .map_err(|_| SvgdxError::ParseError(format!("Invalid condition: '{value}'")))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::context::{ElementMap, VariableMap};
    use crate::element::SvgElement;
    use crate::position::BoundingBox;
    use crate::types::ElRef;
    use assertables::{assert_in_delta, assert_lt};
    use rand::prelude::*;
    use rand_pcg::Pcg32;
    use std::cell::RefCell;

    use super::*;

    struct TestContext {
        vars: HashMap<String, String>,
        rng: RefCell<Pcg32>,
    }

    impl TestContext {
        fn new() -> Self {
            Self {
                vars: HashMap::new(),
                rng: RefCell::new(Pcg32::seed_from_u64(0)),
            }
        }

        fn with_vars(vars: &[(&str, &str)]) -> Self {
            Self {
                vars: vars
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
                rng: RefCell::new(Pcg32::seed_from_u64(0)),
            }
        }
    }

    impl ElementMap for TestContext {
        fn get_element(&self, _elref: &ElRef) -> Option<&SvgElement> {
            None
        }

        fn get_element_bbox(&self, el: &SvgElement) -> Result<Option<BoundingBox>> {
            el.bbox()
        }
    }

    impl VariableMap for TestContext {
        fn get_var(&self, name: &str) -> Option<String> {
            self.vars.get(name).cloned()
        }

        fn get_rng(&self) -> &RefCell<Pcg32> {
            &self.rng
        }
    }

    impl ContextView for TestContext {}

    fn evaluate_one(
        tokens: impl IntoIterator<Item = Token>,
        context: &impl ContextView,
    ) -> Result<f32> {
        let mut eval_state = EvalState::new(tokens, context, &[]);
        let e = expr(&mut eval_state);
        if eval_state.peek().is_none() {
            Ok(e?.one_number()?)
        } else {
            Err(SvgdxError::ParseError(
                "Unexpected trailing tokens".to_owned(),
            ))
        }
    }

    fn expr_check(
        tokens: impl IntoIterator<Item = Token>,
        context: &impl ContextView,
    ) -> Result<ExprValue> {
        let mut eval_state = EvalState::new(tokens, context, &[]);
        expr(&mut eval_state)
    }

    #[test]
    fn test_expr() {
        let ctx = TestContext::with_vars(&[
            ("list", "1, 2, 3"),
            ("kilo", "1000"),
            ("mega", "($kilo * $kilo)"),
        ]);
        for (expr, expected) in [
            ("2 * 2", Some(ExprValue::Number(4.))),
            ("swap(2, 1)", Some(vec![1., 2.].into())),
            ("2 * 2, 5", Some(ExprValue::Number(4.))),
            ("$mega", Some(ExprValue::Number(1000000.))),
            ("$list", Some(vec![1., 2., 3.].into())),
        ] {
            assert_eq!(
                expr_check(tokenize(expr).expect("test"), &ctx).ok(),
                expected,
            )
        }
    }

    #[test]
    fn test_arithmetic_expr() {
        let ctx = TestContext::new();

        for (expr, expected) in [
            ("1+1", 2.),
            ("6 - 9", -3.),
            ("-4 * 5", -20.),
            ("60 / 12", 5.),
            ("63 % 4", 3.),
            ("-1 % 4", 3.), // ensure -a % b is non-negative
            ("-4 * 4", -16.),
            ("-5 - 8", -13.), // check precedence of unary minus
        ] {
            assert_eq!(
                expr_check(tokenize(expr).expect("test"), &ctx).ok(),
                Some(ExprValue::Number(expected)),
            );
        }
    }

    #[test]
    fn test_eval_var() {
        let ctx = TestContext::with_vars(&[
            ("one", "1"),
            ("this_year", "2023"),
            ("empty", ""),
            ("me", "Ben"),
        ]);

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
        use crate::context::TransformerContext;

        let mut ctx = TransformerContext::new();
        // provide some global variables so we can check they are overridden
        for (name, value) in [("one", "1"), ("this_year", "2023")] {
            ctx.set_var(name, value);
        }

        // Check attributes as locals; this would be something like the attributes
        // of a surrounding <g> element, which can be referenced by child elements.
        ctx.push_element(&SvgElement::new(
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
        ctx.push_element(&SvgElement::new("rect", &[]));
        assert_eq!(
            eval_vars("$this_year: $width.$one$height", &ctx),
            "2024: 3.14"
        );

        ctx.pop_element();
        ctx.pop_element();
        // Now `this_year` isn't overridden by the local variable should revert to
        // the global value.
        assert_eq!(eval_vars("$this_year", &ctx), "2023");

        // Check multiple levels of override
        ctx.push_element(&SvgElement::new(
            "g",
            &[("level".to_string(), "1".to_string())],
        ));
        ctx.push_element(&SvgElement::new(
            "g",
            &[("level".to_string(), "2".to_string())],
        ));
        ctx.push_element(&SvgElement::new(
            "g",
            &[("level".to_string(), "3".to_string())],
        ));
        assert_eq!(eval_vars("$level", &ctx), "3");
        ctx.pop_element();
        assert_eq!(eval_vars("$level", &ctx), "2");
        ctx.pop_element();
        assert_eq!(eval_vars("$level", &ctx), "1");
        ctx.pop_element();
        assert_eq!(eval_vars("$level", &ctx), "$level");
    }

    #[test]
    fn test_valid_expressions() {
        let ctx = TestContext::with_vars(&[
            ("pi", "3.1415927"),
            ("tau", "(2. * $pi)"),
            ("milli", "0.001"),
            ("micro", "($milli * $milli)"),
            ("kilo", "1000"),
            ("mega", "($kilo * $kilo)"),
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
            assert_eq!(
                evaluate_one(tokenize(expr).expect("test"), &ctx).ok(),
                expected
            );
        }
    }

    #[test]
    fn test_func_simple() {
        let ctx = TestContext::with_vars(&[("kilo", "1000"), ("mega", "($kilo * $kilo)")]);
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
                evaluate_one(tokenize(expr).expect("test"), &ctx)
                    .ok()
                    .unwrap(),
                expected,
                0.00001
            );
        }
    }

    #[test]
    fn test_func_trig() {
        let ctx = TestContext::new();
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
                evaluate_one(tokenize(expr).expect("test"), &ctx)
                    .ok()
                    .unwrap(),
                expected,
                0.00001
            );
        }
    }

    #[test]
    fn test_func_random() {
        // Check random() provides reasonable samples
        let ctx = TestContext::new();
        let expr = "random()";
        let tokens = tokenize(expr).unwrap();
        let mut count_a = 0;
        let mut count_b = 0;
        for counter in 0..1000 {
            let sample = evaluate_one(tokens.clone(), &ctx).ok().unwrap();
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
            let sample = evaluate_one(tokens.clone(), &ctx).ok().unwrap();
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
        let ctx = TestContext::new();
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
                evaluate_one(tokenize(expr).expect("test"), &ctx)
                    .ok()
                    .unwrap(),
                expected,
            );
        }
    }

    #[test]
    fn test_func_variadic() {
        let ctx = TestContext::new();
        for (expr, expected) in [
            ("min(-10)", Some(-10.)),
            ("min(1,2)", Some(1.)),
            ("min(2,2.5,3,4,2.25)", Some(2.)),
            ("min(1,min(2,3,4,5),6)", Some(1.)),
            ("min()", None),
            ("max(-10)", Some(-10.)),
            ("max(1,2)", Some(2.)),
            ("max(2,2.5,3,4,2.25)", Some(4.)),
            ("max(1,max(2,3,4,5),6)", Some(6.)),
            ("max()", None),
            ("sum(10)", Some(10.)),
            ("sum(2,2.5,3,4,2.25)", Some(13.75)),
            ("sum(1,sum(2,3,4,5),6)", Some(21.)),
            ("sum()", Some(0.)),
            ("product(10)", Some(10.)),
            ("product(2,2.5,3,4,2.25)", Some(135.)),
            ("product(1,2,3,4,5,6)", Some(720.)),
            ("product()", Some(1.)),
            ("mean(234)", Some(234.)),
            ("mean(2,2.5,3,4,2.25)", Some(2.75)),
            ("mean(1,sum(2,3,4,5),6)", Some(7.)),
            ("mean()", None),
        ] {
            assert_eq!(
                evaluate_one(tokenize(expr).expect("test"), &ctx).ok(),
                expected,
                "Failed: {expr} != {expected:?}"
            );
        }
    }

    #[test]
    fn test_func_logic() {
        let ctx = TestContext::new();
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
                evaluate_one(tokenize(expr).expect("test"), &ctx)
                    .ok()
                    .unwrap(),
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
            "${abcthing}",
            "$abc 1",
            "'- -'",
            "'thing'",
            "\"thing\"",
            "\"one\", 'two', 3",
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
            "$abc.",
            "${-}",
            "${abc-thing}",
            "${abc-thing}",
            "234#",
            "thing",
            "'thing",
            "\"thing",
            "thing'",
            "'thing\"",
        ] {
            assert!(tokenize(expr).is_err(), "Should have failed: {expr}");
        }
    }

    #[test]
    fn test_bad_expressions() {
        let ctx = TestContext::with_vars(&[("numbers", "20 40")]);
        for expr in ["1+", "2++2", "%1", "(1+2", "1+4)", "$numbers"] {
            assert!(
                evaluate_one(tokenize(expr).expect("test"), &ctx).is_err(),
                "Should have failed: {expr}"
            );
        }
    }

    #[test]
    fn test_circular_reference() {
        let ctx = TestContext::with_vars(&[("k", "$k - 1"), ("a", "$b"), ("b", "$a")]);
        // These should successfully return error rather than cause stack overflow.
        for expr in ["$k - 1", "$a"] {
            assert!(
                evaluate_one(tokenize(expr).expect("test"), &ctx).is_err(),
                "Should have failed: {expr}"
            );
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
                evaluate_one(tokenize(expr).expect("test"), &TestContext::new()).ok(),
                expected
            );
        }
    }

    #[test]
    fn test_eval_attr() {
        let ctx = TestContext::with_vars(&[
            ("one", "1"),
            ("this_year", "2023"),
            ("empty", ""),
            ("me", "Ben"),
            ("numbers", "20  40"),
        ]);

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

    #[test]
    fn test_eval_condition() {
        let ctx = TestContext::new();
        for (expr, expected) in [
            ("0.", false),
            ("0", false),
            ("0.0", false),
            ("0.001", true),
            ("eq(1, 1)", true),
            ("eq(1, 2)", false),
            ("ne(1, 1)", false),
            ("ne(1, 2)", true),
            ("lt(1, 2)", true),
            ("lt(2, 1)", false),
            ("lt(1, 1)", false),
            ("le(1, 2)", true),
            ("le(2, 1)", false),
            ("le(1, 1)", true),
            ("gt(2, 1)", true),
            ("gt(1, 2)", false),
            ("gt(1, 1)", false),
            ("ge(2, 1)", true),
            ("ge(1, 2)", false),
            ("ge(1, 1)", true),
            ("not(1)", false),
            ("not(0)", true),
            ("not(100)", false),
            ("and(1, 1)", true),
            ("and(1, 0)", false),
            ("and(0, 1)", false),
            ("and(0, 0)", false),
            ("or(1, 1)", true),
            ("or(1, 0)", true),
            ("or(0, 1)", true),
            ("or(0, 0)", false),
            ("xor(1, 1)", false),
            ("xor(1, 0)", true),
            ("xor(0, 1)", true),
            ("xor(0, 0)", false),
        ] {
            assert_eq!(eval_condition(expr, &ctx).expect("test"), expected);
        }
    }

    #[test]
    fn test_eval_expr() {
        let ctx = TestContext::new();
        for (expr, expected) in [
            ("{{1 + 2}} + {{3 + 4}}", "3 + 7"),
            // TODO: following may be unexpected; basically '{{' inside
            // an expression is treated as any other text literal. This
            // may change in future.
            ("abc {{2 {{ 4 }} 3}}", "abc 2 {{ 4  3}}"),
            ("abc 2 {{ 4 }} 3", "abc 2 4 3"),
        ] {
            assert_eq!(eval_expr(expr, &ctx), expected);
        }
    }

    #[test]
    fn test_eval_multiple() {
        let ctx = TestContext::new();
        for (expr, expected) in [
            (
                "{{10, 20 + 3, 2+3  , eq(123, 123), 5/2}}",
                "10, 23, 5, 1, 2.5",
            ),
            ("{{3, 2, swap(1, 2)}}", "3, 2, 2, 1"),
            ("{{p2r(10, 0)}}", "10, 0"),
            ("{{p2r(10, 180)}}", "-10, 0"),
            ("{{p2r(10, 90)}}", "0, 10"),
            ("{{r2p(p2r(0, 0))}}", "0, 0"),
            ("{{(r2p(1, 1))}}", "1.414, 45"),
            ("{{select(0, 1, 2, 3)}}", "1"),
            ("{{select(2, 1, 2, 3)}}", "3"),
        ] {
            assert_eq!(eval_attr(expr, &ctx), expected);
        }
    }

    #[test]
    fn test_eval_vector() {
        let ctx = TestContext::new();
        for (expr, expected) in [
            ("{{addv(1,2)}}", "3"),
            ("{{addv(1,2, 3,4)}}", "4, 6"),
            ("{{addv(1,2,3, 7,8,9)}}", "8, 10, 12"),
            ("{{subv(4, 1)}}", "3"),
            ("{{subv(4,2, 1,0)}}", "3, 2"),
            ("{{scalev(0, 123)}}", "0"),
            ("{{scalev(0.5, 123)}}", "61.5"),
            ("{{scalev(0.5, 1,2,3)}}", "0.5, 1, 1.5"),
        ] {
            assert_eq!(eval_attr(expr, &ctx), expected);
        }
    }

    #[test]
    fn test_eval_list_var() {
        let ctx = TestContext::with_vars(&[("list", "1,2"), ("double", "$list, $list")]);
        for (expr, expected) in [
            ("{{$list}}", "1, 2"),
            ("{{addv(1, $list, 3)}}", "3, 4"),
            ("{{addv(1, $list, 3, 4, 5)}}", "4, 5, 7"),
            ("{{$double}}", "1, 2, 1, 2"),
            ("{{scalev(2, $double)}}", "2, 4, 2, 4"),
        ] {
            assert_eq!(eval_attr(expr, &ctx), expected);
        }
    }

    #[test]
    fn test_eval_head_tail() {
        let ctx = TestContext::new();
        for (expr, expected) in [
            ("{{head(1, 2, 3, 4, 5)}}", "1"),
            ("{{head()}}", ""),
            ("{{head(1)}}", "1"),
            ("{{head(1, 2)}}", "1"),
            ("{{head(1, 2, 3)}}", "1"),
            ("{{tail(1, 2, 3, 4, 5)}}", "2, 3, 4, 5"),
            ("{{tail()}}", ""),
            ("{{tail(1)}}", ""),
            ("{{tail(1, 2)}}", "2"),
            ("{{tail(1, 2, 3)}}", "2, 3"),
            ("{{empty(1)}}", "0"),
            ("{{empty(1, 2, 3)}}", "0"),
            ("{{empty()}}", "1"),
            ("{{empty(tail(1))}}", "1"),
            ("{{count()}}", "0"),
            ("{{count(1)}}", "1"),
            ("{{count(1, 2, 3, 4, 5)}}", "5"),
        ] {
            assert_eq!(eval_attr(expr, &ctx), expected, "'{expr}' != '{expected}'");
        }
    }

    #[test]
    fn test_eval_var_indirect() {
        // A single level of variable lookup is done prior to evaluation,
        // which also performs a variable lookup which must result in a numeric value
        // or the empty string.
        let ctx = TestContext::with_vars(&[
            ("blank", "$null"),
            ("null", ""),
            ("one", "1"),
            ("two", "2"),
            ("choice", "$two"),
        ]);
        for (expr, expected) in [
            ("{{empty($blank)}}", "1"),
            ("{{head($blank)}}", ""),
            ("{{tail($blank)}}", ""),
            ("{{head(4, $blank)}}", "4"),
            ("{{head($blank, 4)}}", "4"),
            ("{{count($blank)}}", "0"),
            ("{{count($blank, $blank)}}", "0"),
            ("{{count($blank, 4)}}", "1"),
            ("{{count(4, $blank)}}", "1"),
            ("{{count($blank, 4, $blank)}}", "1"),
            ("{{$choice}}", "2"),
            ("{{$choice + 1}}", "3"),
        ] {
            assert_eq!(eval_attr(expr, &ctx), expected, "'{expr}' != '{expected}'");
        }
    }

    #[test]
    fn test_string_functions() {
        let ctx = TestContext::new();
        for (expr, expected) in [
            ("{{count('a', 'b', 'c')}}", "3"),
            ("{{select(1, 'a', 'b', 'c', 4, 'd', 9)}}", "'b'"),
            ("{{select(5, 'a', 'b', 'c', 4, 'd', 9)}}", "9"),
            ("{{swap('a', 'b')}}", "'b', 'a'"),
            ("{{swap('a', 1)}}", "1, 'a'"),
            ("{{head('a', 'b', 'c')}}", "'a'"),
            ("{{tail('a', 'b', 'c')}}", "'b', 'c'"),
            ("{{in('c', 'a', 'b', 'c')}}", "1"),
            ("{{in('d', 'a', 'b', 'c')}}", "0"),
            ("{{if(eq('t1', 't2'), 'yes', 'no')}}", "'no'"),
            ("{{if(ne('t1', 't2'), 'yes', 'no')}}", "'yes'"),
            ("{{split(':', 'abc:def:ghi')}}", "'abc', 'def', 'ghi'"),
            ("{{split('def', 'abc:def:ghi')}}", "'abc:', ':ghi'"),
            ("{{split('xyz', 'abc:def:ghi')}}", "'abc:def:ghi'"),
            ("{{split('a', 'abc:def:ghi')}}", "'', 'bc:def:ghi'"),
            ("{{split('i', 'abc:def:ghi')}}", "'abc:def:gh', ''"),
            (
                "{{splitw('  some words  with spaces')}}",
                "'some', 'words', 'with', 'spaces'",
            ),
            ("{{splitw('  ')}}", ""),
            ("{{trim('   a text blob ')}}", "'a text blob'"),
            ("{{trim('  ')}}", "''"),
            ("{{join(':', '01', '02', '03')}}", "'01:02:03'"),
            ("{{join('::', 'base', 'target')}}", "'base::target'"),
            ("{{join('', 'base', 'target')}}", "'basetarget'"),
            ("{{join('* -')}}", "''"),
        ] {
            assert_eq!(eval_attr(expr, &ctx), expected, "'{expr}' != '{expected}'");
        }
    }

    #[test]
    fn test_string_escape() {
        let ctx = TestContext::new();
        for (expr, expected) in [
            ("{{'abc'}}", "'abc'"),
            ("{{'a\'bc'}}", "'a'bc'"),
            (r#"{{'a\', \'bc'}}"#, r#"'a\', \'bc'"#),
            (r#"{{_('abc')}}"#, r#"abc"#),
            (r#"{{_('a\', \'bc')}}"#, r#"a', 'bc"#),
            (r#"{{'a\nb'}}"#, r#"'a\nb'"#),
            (r#"{{'a\\b'}}"#, r#"'a\\b'"#),
            (r#"{{_('a\nb')}}"#, "a\nb"),
            (r#"{{_('a\\b')}}"#, "a\\b"),
        ] {
            assert_eq!(eval_attr(expr, &ctx), expected, "'{expr}' != '{expected}'");
        }
    }
}
