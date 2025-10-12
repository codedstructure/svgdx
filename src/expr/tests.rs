use super::expression::{
    eval_expr, eval_vars, expr, tokenize, valid_symbol, EvalState, ExprValue, Token,
};
use crate::errors::{Error, Result};

use std::collections::HashMap;

use crate::context::{ContextView, ElementMap, VariableMap};
use crate::elements::SvgElement;
use crate::geometry::{BoundingBox, Size};
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

    fn get_element_size(&self, el: &SvgElement) -> Result<Option<Size>> {
        el.size(self)
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
    let e = expr(&mut eval_state)?;
    if eval_state.peek().is_none() {
        Ok(e.one_number()?)
    } else {
        Err(Error::Parse("unexpected trailing tokens".to_owned()))
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
fn test_valid_symbol() {
    for s in ["abc", "a1", "a_b", "_abc", "__", "a_b_c", "a_b_c_"] {
        assert!(valid_symbol(s));
    }
    for s in ["", "1", "123", "1abc", "1_a", "1a_", "1_"] {
        assert!(!valid_symbol(s));
    }
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
        ("11 // 4", 2.),
        ("-11 // 4", -3.), // ensure -a // b is rounds down
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
        "'thing'",   // string
        "\"thing\"", // string
        "thing",     // symbol
        "\"one\", 'two', 3",
        "2 eq 2",
    ] {
        let t = tokenize(expr);
        assert!(t.is_ok(), "Should succeed: {expr} => {t:?}");
    }
}

#[test]
fn test_infix_comparison() {
    for expr in [
        ("1 eq 1", 1.),
        ("1 ne 1", 0.),
        ("1 ne 2", 1.),
        ("1 lt 2", 1.),
        ("2 lt 1", 0.),
        ("1 le 2", 1.),
        ("2 le 1", 0.),
        ("1 le 1", 1.),
        ("2 gt 1", 1.),
        ("1 gt 2", 0.),
        ("1 ge 2", 0.),
        ("2 ge 1", 1.),
        ("1 ge 1", 1.),
    ] {
        let tokens = tokenize(expr.0).expect("test");
        let res = evaluate_one(tokens, &TestContext::new()).unwrap();
        assert_eq!(
            res, expr.1,
            "{} => Got: {}; Expected: {}",
            res, expr.0, expr.1
        );
    }
}

#[test]
fn test_infix_logical() {
    for expr in [
        ("1 and 1", 1.),
        ("1 and 0", 0.),
        ("0 and 1", 0.),
        ("0 and 0", 0.),
        ("2.5 and 0.1", 1.),
        ("1 or 1", 1.),
        ("1 or 0", 1.),
        ("0 or 1", 1.),
        ("0 or 0", 0.),
        ("2.5 and 0.1", 1.),
        ("1 xor 1", 0.),
        ("1 xor 0", 1.),
        ("0 xor 1", 1.),
        ("0 xor 0", 0.),
        ("2.5 xor 0.1", 0.),
        ("1 and 1 and 1 and 1 and 1", 1.),
        ("1 and 1 and 0 and 1 and 1", 0.),
        ("0 and 1 and 0 and 1 and 1", 0.),
        ("0 or 0 or 0 or 0 or 0", 0.),
        ("1 or 0 or 0 or 0 or 0", 1.),
        ("0 or 0 or 0 or 1 or 0", 1.),
        ("1 or 1 or 1 or 1 or 1", 1.),
        ("1 le 2 and 3 eq 3", 1.),
        ("1 le 2 and 3 eq 4", 0.),
        ("1 le 2 or 3 eq 4", 1.),
        ("1 gt 2 or 3 eq 4", 0.),
        ("1 gt 2 or 3 eq 3", 1.),
        ("1 gt 2 or 3 eq 2", 0.),
        ("1 gt 2 xor 3 eq 4", 0.),
        ("1 gt 2 xor 3 eq 3", 1.),
        ("1 gt 2 xor 3 eq 2", 0.),
        ("1 le 2 xor 3 eq 4", 1.),
        ("1 le 2 xor 3 eq 3", 0.),
        ("1 le 2 xor 3 eq 2", 1.),
    ] {
        let tokens = tokenize(expr.0).expect("test");
        let res = evaluate_one(tokens, &TestContext::new()).unwrap();
        assert_eq!(
            res, expr.1,
            "{} => Got: {}; Expected: {}",
            res, expr.0, expr.1
        );
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
        "1thing",
        "'thing",
        "\"thing",
        "thing'",
        "'thing\"",
    ] {
        let tokens = tokenize(expr);
        assert!(tokens.is_err(), "Should have failed: {expr} => {tokens:?}");
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
        eval_attr("Made by ${me} in 20{{20 + ${one} * 3}}", &ctx).unwrap(),
        "Made by Ben in 2023"
    );
    assert_eq!(
        eval_attr("Made by ${me} in {{5*4}}{{20 + ${one} * 3}}", &ctx).unwrap(),
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
        ("abc 2 {{ 4 }} 3", "abc 2 4 3"),
    ] {
        assert_eq!(eval_expr(expr, &ctx).unwrap(), expected);
    }
}

#[test]
fn test_eval_multiple() {
    let ctx = TestContext::new();
    for (expr, expected) in [
        (
            "{{10, 20 + 3, 2+3  , eq(123, 123), 5/2, 3//2}}",
            "10, 23, 5, 1, 2.5, 1",
        ),
        ("{{3, 2, swap(1, 2)}}", "3, 2, 2, 1"),
        ("{{p2r(10, 0)}}", "10, 0"),
        ("{{p2r(10, 180)}}", "-10, 0"),
        ("{{p2r(10, 90)}}", "0, 10"),
        ("{{r2p(p2r(0, 0))}}", "0, 0"),
        ("{{(r2p(1, 1))}}", "1.414, 45"),
        ("{{select(0, 1, 2, 3)}}", "1"),
        ("{{select(2, 1, 2, 3)}}", "3"),
        ("{{divmod(3, 2)}}", "1, 1"),
        ("{{divmod(3, 8)}}", "0, 3"),
        ("{{divmod(28, 8)}}", "3, 4"),
    ] {
        assert_eq!(eval_attr(expr, &ctx).unwrap(), expected);
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
        assert_eq!(eval_attr(expr, &ctx).unwrap(), expected);
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
        assert_eq!(eval_attr(expr, &ctx).unwrap(), expected);
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
        assert_eq!(
            eval_attr(expr, &ctx).unwrap(),
            expected,
            "'{expr}' != '{expected}'"
        );
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
        assert_eq!(
            eval_attr(expr, &ctx).unwrap(),
            expected,
            "'{expr}' != '{expected}'"
        );
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
        assert_eq!(
            eval_attr(expr, &ctx).unwrap(),
            expected,
            "'{expr}' != '{expected}'"
        );
    }
}

#[test]
fn test_string_escape() {
    let ctx = TestContext::new();
    for (expr, expected) in [
        ("{{'abc'}}", "'abc'"),
        (r#"{{'a\', \'bc'}}"#, r#"'a\', \'bc'"#),
        (r#"{{_('abc')}}"#, r#"abc"#),
        (r#"{{_('a\', \'bc')}}"#, r#"a', 'bc"#),
        (r#"{{'a\nb'}}"#, r#"'a\nb'"#),
        (r#"{{'a\\b'}}"#, r#"'a\\b'"#),
        (r#"{{_('a\nb')}}"#, "a\nb"),
        (r#"{{_('a\\b')}}"#, "a\\b"),
    ] {
        assert_eq!(
            eval_attr(expr, &ctx).unwrap(),
            expected,
            "'{expr}' != '{expected}'"
        );
    }
}
