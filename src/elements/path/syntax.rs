use crate::errors::{Error, Result};

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
}

impl PathSyntax for SvgPathSyntax {
    fn at_command(&self) -> Result<bool> {
        self.check_not_end()?;
        let c = self
            .current()
            .ok_or_else(|| Error::Parse("no data".to_string()))?;
        Ok("MmLlHhVvZzCcSsQqTtAa".contains(c))
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

pub trait PathSyntax {
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
                ))
            }
        };
        self.advance();
        self.skip_wsp_comma();
        Ok(res)
    }

    fn read_count(&mut self) -> Result<usize> {
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
        Ok(c.parse()?)
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
        Ok(s.parse()?)
    }

    fn read_coord(&mut self) -> Result<(f32, f32)> {
        let x = self.read_number()?;
        self.skip_wsp_comma();
        let y = self.read_number()?;
        self.skip_wsp_comma();
        Ok((x, y))
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
