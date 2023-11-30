/// Recursive descent expression parser

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f32),
    Var(String),
    OpenParen,
    CloseParen,
    Add,
    Sub,
    Mul,
    Div,
    Whitespace,
    Other,
}

fn tokenize_atom(input: &str) -> Option<Token> {
    if let Some(input) = input.strip_prefix('$') {
        let var_name = if let Some(input) = input.strip_prefix('{') {
            input.strip_suffix('}')
        } else {
            Some(input)
        };
        var_name.map(|v| Token::Var(v.to_string()))
    } else {
        input.parse::<f32>().ok().map(Token::Number)
    }
}

fn tokenize(input: &str) -> Vec<Token> {
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
                    let buffer_token = tokenize_atom(&buffer.iter().collect::<String>()).unwrap();
                    buffer.clear();
                    tokens.push(buffer_token);
                }
                tokens.push(next_token);
            }
        }
    }

    if !buffer.is_empty() {
        let buffer_token = tokenize_atom(&buffer.iter().collect::<String>()).unwrap();
        buffer.clear();
        tokens.push(buffer_token);
    }
    tokens
}


#[derive(Clone, Debug, PartialEq)]
struct EvalState {
    tokens: Vec<Token>,
    index: usize,
}

impl EvalState {
    fn new(tokens: impl IntoIterator<Item = Token>) -> Self {
        Self {
            tokens: tokens.into_iter().collect(),
            index: 0,
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
}

fn evaluate(tokens: impl IntoIterator<Item = Token>) -> Result<f32, &'static str> {
    let mut eval_state = EvalState::new(tokens);
    let e = expr(&mut eval_state);
    if eval_state.peek().is_none() {
        e
    } else {
        Err("Unexpected trailing tokens")
    }
}

fn expr(eval_state: &mut EvalState) -> Result<f32, &'static str> {
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

fn term(eval_state: &mut EvalState) -> Result<f32, &'static str> {
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
            _ => {
                break;
            }
        }
    }
    Ok(e)
}

fn factor(eval_state: &mut EvalState) -> Result<f32, &'static str> {
    match eval_state.next() {
        Some(Token::Number(x)) => Ok(x),
        Some(Token::OpenParen) => {
            let e = expr(eval_state)?;
            if eval_state.next() != Some(Token::CloseParen) {
                return Err("Expected closing parenthesis");
            }
            Ok(e)
        }
        Some(Token::Sub) => match eval_state.peek() {
            Some(Token::OpenParen) | Some(Token::Number(_)) => Ok(-expr(eval_state)?),
            _ => Err("Invalid unary minus"),
        },
        _ => Err("Invalid token in factor()"),
    }
}

#[test]
fn test_valid_expressions() {
    for (expr, expected) in [
        ("2 * 2", Ok(4.)),
        ("2 * 2 + 1", Ok(5.)),
        ("2 * (2 + 1)", Ok(6.)),
        ("(2+1)/2", Ok(1.5)),
        ("   (   2 +   1)   / 2", Ok(1.5)),
        ("(1+(1+(1+(1+(2+1)))))/2", Ok(3.5)),
    ] {
        assert_eq!(evaluate(tokenize(&expr)), expected);
    }
}
