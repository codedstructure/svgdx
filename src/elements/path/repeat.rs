//! Support for 'repeat' commands in SVG paths.
//!
//! A block of path commands delimited by square brackets is repeated `N` times
//! when preceded by an `r N` command. Together with the bearing commands, this
//! allows turtle-graphics style path definitions.
//!
//! - repeated blocks may be nested.
//! - analogous to the 'z' command, 'r' may be upper or lower case.
//!
//! Expansion accounting, limited by path-repeat-limit config value:
//! - Nested repeats multiply (`r9[r9[...]]` => 81)
//! - Sequential repeats sum (`r9[...] r9[...]` => 18)
//!
//! Example: `<path d="M 0 0 r 6 [ h 10 b 60 ]"/>`

use super::syntax::SvgPathSyntax;
use crate::context::ContextView;
use crate::elements::path::syntax::PathSyntax;
use crate::errors::{Error, Result};

#[derive(Clone, Copy)]
struct RepeatFrame {
    start_index: usize,
    repeat_product: u32,
    remaining: u32,
}

#[derive(Clone)]
pub struct RepeatStack {
    // supports nested repeat blocks
    stack: Vec<RepeatFrame>,
    // configured repeat expansion limit.
    repeat_limit: u32,
}

impl RepeatStack {
    pub fn new(repeat_limit: u32) -> Self {
        Self {
            stack: Vec::new(),
            repeat_limit,
        }
    }

    pub fn reset(&mut self) {
        self.stack.clear();
    }

    pub fn enter_repeat(&mut self, start_index: usize, count: u32) -> Result<bool> {
        if count == 0 {
            return Ok(false);
        }

        let repeat_product = self.stack.last().map(|rf| rf.repeat_product).unwrap_or(1);
        let next_repeat_product = repeat_product
            .checked_mul(count)
            .ok_or(Error::PathRepeatLimit(u32::MAX, self.repeat_limit))?;
        if next_repeat_product > self.repeat_limit {
            return Err(Error::PathRepeatLimit(
                next_repeat_product,
                self.repeat_limit,
            ));
        }

        self.stack.push(RepeatFrame {
            start_index,
            repeat_product: next_repeat_product,
            remaining: count,
        });
        Ok(true)
    }

    pub fn end_repeat(&mut self) -> Result<RepeatAction> {
        let frame = self
            .stack
            .last_mut()
            .ok_or_else(|| Error::Parse("unexpected ']' in path data".to_string()))?;

        if frame.remaining > 1 {
            frame.remaining -= 1;
            Ok(RepeatAction::Loop(frame.start_index))
        } else {
            self.stack.pop().expect("repeat frame should still exist");
            Ok(RepeatAction::Exit)
        }
    }
}

pub enum RepeatAction {
    Loop(usize),
    Exit,
}

pub struct Repeat {
    count: u32,
}

impl Repeat {
    pub fn from_tokens(tokens: &mut SvgPathSyntax, ctx: &impl ContextView) -> Result<Self> {
        let count = tokens.read_count(ctx)?;
        tokens.read_expected('[')?;

        Ok(Self { count })
    }

    pub fn count(&self) -> u32 {
        self.count
    }
}
