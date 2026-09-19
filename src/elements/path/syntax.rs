use super::types::Vec2;
use crate::constants::{EDGESPEC_SEP, EXPR_START, LOCSPEC_SEP, SCALARSPEC_SEP, VAR_PREFIX};
use crate::context::ContextView;
use crate::elements::layout::expand_single_relspec;
use crate::errors::{Error, Result};
use crate::expr::{eval_attr, extract_expr, extract_var};
use crate::types::{extract_elref_token, parse_float, parse_int};

// https://www.w3.org/TR/SVG11/paths.html#PathDataBNF
// plus svgdx bearing and repeat extensions.
pub const PATH_COMMANDS: [char; 27] = [
    'M', 'm', 'Z', 'z', 'L', 'l', 'H', 'h', 'V', 'v', // line and move commands
    'C', 'c', 'S', 's', 'Q', 'q', 'T', 't', 'A', 'a', // curve commands
    'B', 'b', // svgdx-specific bearing commands
    'R', 'r', '[', ']', // svgdx-specific repeat controls
    ':', // svgdx-specific variable assignment command
];

#[derive(Clone, Debug, PartialEq)]
// Deferred path expressions/variables are recorded as source-local replacements so
// rendering can splice only the dynamic fragment back into the original command text.
pub struct SourcePatch {
    pub start: usize,
    pub end: usize,
    pub replacement: String,
}

#[derive(Clone)]
pub struct SvgPathSyntax {
    data: Vec<char>,
    index: usize,
    patches: Vec<SourcePatch>,
}

impl SvgPathSyntax {
    pub fn new(data: &str) -> Self {
        Self {
            data: data.chars().collect(),
            index: 0,
            patches: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.index = 0;
        self.patches.clear();
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

    fn remaining(&self) -> String {
        self.data[self.index..].iter().collect()
    }

    pub fn patch_count(&self) -> usize {
        self.patches.len()
    }

    pub fn patches_since(&self, patch_start: usize, source_start: usize) -> Vec<SourcePatch> {
        // Patches are collected against the full path input while parsing. Convert
        // them back to instruction-local offsets before handing them to render().
        self.patches[patch_start..]
            .iter()
            .map(|patch| SourcePatch {
                start: patch.start - source_start,
                end: patch.end - source_start,
                replacement: patch.replacement.clone(),
            })
            .collect()
    }

    fn apply_patch(&mut self, patch: SourcePatch) {
        self.index = patch.end;
        self.patches.push(patch);
    }

    fn parse_literal_number_value(value: &str) -> Result<f32> {
        let mut syntax = SvgPathSyntax::new(value);
        let number = syntax.read_number_literal()?;
        syntax.skip_whitespace();
        if syntax.at_end() {
            Ok(number)
        } else {
            Err(Error::Parse(format!("expected numeric value: '{value}'")))
        }
    }

    fn parse_literal_coord_value(value: &str) -> Result<Vec2> {
        let mut syntax = SvgPathSyntax::new(value);
        let coord = syntax.read_coord_literal()?;
        syntax.skip_whitespace();
        if syntax.at_end() {
            Ok(coord)
        } else {
            Err(Error::Parse(format!("expected coordinate pair: '{value}'")))
        }
    }

    fn relspec_suffix_len(remainder: &str) -> usize {
        if !matches!(
            remainder.chars().next(),
            Some(LOCSPEC_SEP | SCALARSPEC_SEP | EDGESPEC_SEP)
        ) {
            return 0;
        }

        let fake = format!("#a{remainder}");
        extract_elref_token(&fake)
            .map(|token| token.len().saturating_sub(2))
            .unwrap_or(0)
    }

    fn peek_dynamic_patch<T: ContextView>(&self, ctx: &T) -> Result<Option<SourcePatch>> {
        let remaining = self.remaining();
        if remaining.starts_with(EXPR_START) {
            let (_, remain) = extract_expr(&remaining)?;
            let consumed = remaining.len() - remain.len();
            let suffix_len = Self::relspec_suffix_len(remain);
            let token = &remaining[..consumed + suffix_len];
            return Ok(Some(SourcePatch {
                start: self.index,
                end: self.index + token.chars().count(),
                replacement: expand_single_relspec(&eval_attr(token, ctx)?, ctx),
            }));
        }

        if remaining.starts_with(VAR_PREFIX) {
            let (_, remain) = extract_var(&remaining)?;
            let consumed = remaining.len() - remain.len();
            let suffix_len = Self::relspec_suffix_len(remain);
            let token = &remaining[..consumed + suffix_len];

            return Ok(Some(SourcePatch {
                start: self.index,
                end: self.index + token.chars().count(),
                replacement: expand_single_relspec(&eval_attr(token, ctx)?, ctx),
            }));
        }

        Ok(None)
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

    fn read_count_literal(&mut self) -> Result<u32> {
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

    fn read_count<T: ContextView>(&mut self, ctx: &T) -> Result<u32>;

    fn read_number_literal(&mut self) -> Result<f32> {
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
        Ok(mult * self.read_non_negative_literal()?)
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

    fn read_identifier(&mut self) -> Result<String> {
        self.check_not_end()?;

        let first = self.current().unwrap();
        if !(first.is_ascii_alphabetic() || first == '_') {
            return Err(Error::Parse(format!(
                "expected identifier, got '{}'",
                first
            )));
        }

        let mut ident = String::new();
        ident.push(first);
        self.advance();

        while let Some(ch) = self.current() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        Ok(ident)
    }

    fn read_non_negative_literal(&mut self) -> Result<f32> {
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

    fn read_non_negative<T: ContextView>(&mut self, ctx: &T) -> Result<f32>;

    fn read_number<T: ContextView>(&mut self, ctx: &T) -> Result<f32>;

    fn read_coord_literal(&mut self) -> Result<Vec2> {
        let x = self.read_number_literal()?;
        self.skip_wsp_comma();
        let y = self.read_number_literal()?;
        self.skip_wsp_comma();
        Ok(Vec2::new(x, y))
    }

    fn read_coord<T: ContextView>(&mut self, ctx: &T) -> Result<Vec2>;

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

    fn read_count<T: ContextView>(&mut self, ctx: &T) -> Result<u32> {
        if let Some(patch) = self.peek_dynamic_patch(ctx)? {
            let count = parse_int(patch.replacement.trim())?;
            self.apply_patch(patch);
            self.skip_wsp_comma();
            return Ok(count);
        }

        self.read_count_literal()
    }

    fn read_non_negative<T: ContextView>(&mut self, ctx: &T) -> Result<f32> {
        self.check_not_end()?;
        if let Some(patch) = self.peek_dynamic_patch(ctx)? {
            let value = SvgPathSyntax::parse_literal_number_value(&patch.replacement)?;
            self.apply_patch(patch);
            self.skip_wsp_comma();
            return Ok(value);
        }

        self.read_non_negative_literal()
    }

    fn read_number<T: ContextView>(&mut self, ctx: &T) -> Result<f32> {
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
        Ok(mult * self.read_non_negative(ctx)?)
    }

    fn read_coord<T: ContextView>(&mut self, ctx: &T) -> Result<Vec2> {
        if let Some(patch) = self.peek_dynamic_patch(ctx)?
            && let Ok(coord) = SvgPathSyntax::parse_literal_coord_value(&patch.replacement)
        {
            self.apply_patch(patch);
            self.skip_wsp_comma();
            return Ok(coord);
        }

        let x = self.read_number(ctx)?;
        self.skip_wsp_comma();
        let y = self.read_number(ctx)?;
        self.skip_wsp_comma();
        Ok(Vec2::new(x, y))
    }
}
