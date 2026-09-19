use super::types::Vec2;
use crate::errors::{Error, Result};
use crate::types::{parse_float, parse_int};

// https://www.w3.org/TR/SVG11/paths.html#PathDataBNF
// plus svgdx bearing and repeat extensions.
pub const PATH_COMMANDS: [char; 26] = [
    'M', 'm', 'Z', 'z', 'L', 'l', 'H', 'h', 'V', 'v', // line and move commands
    'C', 'c', 'S', 's', 'Q', 'q', 'T', 't', 'A', 'a', // curve commands
    'B', 'b', // svgdx-specific bearing commands
    'R', 'r', '[', ']', // svgdx-specific repeat controls
];

#[derive(Clone)]
pub struct SvgPathSyntax {
    data: Vec<char>,
    index: usize,
}

impl SvgPathSyntax {
    pub fn new(data: &str) -> Self {
        Self {
            data: data.chars().collect(),
            index: 0,
        }
    }

    pub fn reset(&mut self) {
        self.index = 0;
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn set_index(&mut self, index: usize) {
        self.index = index;
    }

    pub fn slice(&self, start: usize, end: usize) -> String {
        self.data[start..end].iter().collect()
    }

    pub fn skip_to_matching_repeat_end(&mut self) -> Result<()> {
        let mut depth = 0;
        // TODO: this is fragile, as it will count '[' even outside a
        // Repeat command (e.g. inside a string etc).
        // It should probably parse commands as normal but discard both rendered
        // strings and side effects on PathState.
        while self.index < self.data.len() {
            let ch = self.data[self.index];
            self.advance();
            match ch {
                '[' => depth += 1,
                ']' if depth == 0 => return Ok(()),
                ']' => depth -= 1,
                _ => {}
            }
        }

        Err(Error::Parse("expected ']' to close repeat block".into()))
    }
}

impl PathSyntax for SvgPathSyntax {
    fn at_command(&self) -> Result<bool> {
        self.check_not_end()?;
        let c = self
            .current()
            .ok_or_else(|| Error::Parse("no data".to_string()))?;
        Ok(PATH_COMMANDS.contains(&c))
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

pub(super) trait PathSyntax {
    fn at_command(&self) -> Result<bool>;
    fn current(&self) -> Option<char>;
    fn advance(&mut self);
    fn at_end(&self) -> bool;

    fn check_not_end(&self) -> Result<()> {
        if self.at_end() {
            Err(Error::Parse("ran out of data!".to_string()))
        } else {
            Ok(())
        }
    }

    fn skip_whitespace(&mut self) {
        // SVG definition of whitespace is 0x20, 0x9, 0xA, 0xD. Rust's is_ascii_whitespace()
        // also includes 0xC, but is close enough and convenient.
        while !self.at_end() && self.current().unwrap().is_ascii_whitespace() {
            self.advance();
        }
    }

    fn skip_wsp_comma(&mut self) {
        self.skip_whitespace();
        if !self.at_end() && self.current().unwrap() == ',' {
            self.advance();
            self.skip_whitespace();
        }
    }

    fn read_flag(&mut self) -> Result<u32> {
        self.check_not_end()?;
        // per the grammar for `a`/`A`, could have '00' etc for
        // the two adjacent flags...
        let res = match self.current().unwrap() {
            '0' => 0,
            '1' => 1,
            _ => {
                return Err(Error::InvalidValue(
                    "flag".to_string(),
                    self.current().unwrap().to_string(),
                ));
            }
        };
        self.advance();
        self.skip_wsp_comma();
        Ok(res)
    }

    fn read_count(&mut self) -> Result<u32> {
        // non-negative integer; read until non-digit
        let mut c = String::new();
        while let Some(ch) = self.current() {
            match ch {
                '0'..='9' => {
                    c.push(ch);
                    self.advance();
                }
                _ => break,
            }
        }
        self.skip_wsp_comma();
        parse_int(c)
    }

    fn read_number(&mut self) -> Result<f32> {
        self.check_not_end()?;
        let mut mult = 1.;
        match self.current().unwrap() {
            '-' => {
                mult = -1.;
                self.advance();
            }
            '+' => {
                self.advance();
            }
            _ => {}
        };
        Ok(mult * self.read_non_negative()?)
    }

    fn read_expected(&mut self, expected: char) -> Result<()> {
        self.check_not_end()?;
        if self.current().unwrap() == expected {
            self.advance();
            self.skip_whitespace();
            Ok(())
        } else {
            Err(Error::InvalidValue(
                format!("expected '{}'", expected),
                self.current().map(|c| c.to_string()).unwrap_or_default(),
            ))
        }
    }

    fn read_non_negative(&mut self) -> Result<f32> {
        self.check_not_end()?;
        let mut s = String::new();
        let mut dot_valid = true;
        let mut exp_valid = true;
        while let Some(ch) = self.current() {
            match ch {
                '0'..='9' => {
                    s.push(ch);
                    self.advance();
                }
                '.' if dot_valid => {
                    s.push(ch);
                    self.advance();
                    dot_valid = false;
                }
                'e' | 'E' if exp_valid && s.ends_with(|c: char| c.is_ascii_digit()) => {
                    s.push(ch);
                    self.advance();
                    // include sign character if present
                    if self.current() == Some('-') || self.current() == Some('+') {
                        s.push(self.current().unwrap());
                        self.advance();
                    }
                    exp_valid = false;
                    dot_valid = false;
                }
                _ => break,
            }
        }
        self.skip_wsp_comma();
        parse_float(s)
    }

    fn read_coord(&mut self) -> Result<Vec2> {
        let x = self.read_number()?;
        self.skip_wsp_comma();
        let y = self.read_number()?;
        self.skip_wsp_comma();
        Ok(Vec2::new(x, y))
    }

    fn read_command(&mut self) -> Result<char> {
        if self.at_command()? {
            let command = self.current().unwrap();
            self.advance();
            self.skip_whitespace();
            Ok(command)
        } else {
            Err(Error::InvalidValue(
                "invalid path command".to_string(),
                self.current().map(|c| c.to_string()).unwrap_or_default(),
            ))
        }
    }
}
