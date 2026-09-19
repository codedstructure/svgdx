use super::command::Command;
use super::repeat::RepeatAction;
use super::setvar::PathEvalContext;
use super::state::PathState;
use super::{SvgElement, Vec2};
use crate::TransformConfig;
use crate::context::{ContextView, TransformerContext};
use crate::errors::{Error, Result};
use crate::geometry::{BoundingBox, Length};
use std::collections::HashMap;

use super::syntax::{PathSyntax, SourcePatch, SvgPathSyntax};

struct ParsedInstruction {
    command: char,
    source: String,
    patches: Vec<SourcePatch>,
    state_before: PathState,
    instruction: Command,
}

#[derive(Clone)]
pub(super) struct PathParser {
    tokens: SvgPathSyntax, // TODO: ref to make clone cheaper?
    state: PathState,
    vars: HashMap<String, String>,
}

impl PathParser {
    pub fn new(data: &str, cfg: &TransformConfig) -> Self {
        PathParser {
            tokens: SvgPathSyntax::new(data),
            state: PathState::new_with_repeat_limit(cfg.path_repeat_limit),
            vars: HashMap::new(),
        }
    }

    fn reset(&mut self) {
        self.tokens.reset();
        self.state.reset();
        self.vars.clear();
    }

    pub fn get_bbox(&self) -> Option<BoundingBox> {
        self.state.get_bbox()
    }

    pub fn length_so_far(&self) -> f32 {
        self.state.length_so_far()
    }

    fn process_instruction(&mut self, ctx: &impl ContextView) -> Result<()> {
        let parsed = self.parse_instruction(ctx)?;
        self.apply_instruction(&parsed.instruction)?;
        Ok(())
    }

    fn parse_instruction(&mut self, ctx: &impl ContextView) -> Result<ParsedInstruction> {
        self.tokens.skip_whitespace();
        let source_start = self.tokens.index();
        let patch_start = self.tokens.patch_count();
        let state_before = self.state.clone();
        // Smooth curve reflection only applies when the immediately preceding
        // instruction was the matching bezier type, so every other instruction
        // must clear the stored control points after it is processed.
        let command = self.state.read_instruction_command(&mut self.tokens)?;

        let path_ctx = PathEvalContext {
            base: ctx,
            vars: &self.vars,
        };
        let instruction = Command::from_tokens(&mut self.tokens, &path_ctx, command, &self.state)?;
        let source_end = self.tokens.index();
        let source = self.tokens.slice(source_start, source_end);
        let patches = self.tokens.patches_since(patch_start, source_start);

        Ok(ParsedInstruction {
            command,
            source,
            patches,
            state_before,
            instruction,
        })
    }

    fn apply_instruction(&mut self, instruction: &Command) -> Result<()> {
        let next_cubic_cp2 = instruction.next_cubic_cp2();
        let next_quadratic_cp = instruction.next_quadratic_cp();

        match instruction {
            Command::Bearing(bearing) => {
                self.state.set_bearing(bearing.bearing());
            }
            Command::SetVar(set_var) => {
                self.vars
                    .insert(set_var.name().to_string(), set_var.value().to_string());
            }
            Command::Repeat(repeat) => {
                self.state.clear_command();
                if !self
                    .state
                    .enter_repeat(self.tokens.index(), repeat.count())?
                {
                    // skip ahead to the matching closing bracket;
                    self.tokens.skip_to_matching_repeat_end()?;
                }
            }
            Command::EndRepeat => {
                self.state.clear_command();
                match self.state.end_repeat()? {
                    RepeatAction::Loop(start_index) => self.tokens.set_index(start_index),
                    RepeatAction::Exit => {} //self.tokens.advance(),
                }
                self.state.clear_command();
            }
            Command::MoveTo(jump) => {
                // 'Subsequent "moveto" commands (i.e., when the "moveto" is not
                // the first command) represent the start of a new subpath'
                // (the first moveto is also the start of a subpath)
                self.state.new_subpath(jump.end());
            }
            Command::LineTo(line) => {
                self.state.extend_subpath(line.end());
            }
            Command::HorizontalLineTo(line) => {
                self.state.extend_subpath(line.end());
            }
            Command::VerticalLineTo(line) => {
                self.state.extend_subpath(line.end());
            }
            Command::ClosePath(line) => {
                self.state.extend_subpath(line.end());
                // since this doesn't consume further tokens, we must clear the command
                // to force getting a new command token, or we could loop forever
                self.state.clear_command();
            }
            Command::CubicBezier(curve) => {
                self.state
                    .extend_curve(curve.end(), curve.extrema(), curve.approx_length());
            }
            Command::QuadraticBezier(curve) => {
                self.state
                    .extend_curve(curve.end(), curve.extrema(), curve.approx_length());
            }
            Command::Arc(arc) => {
                self.state
                    .extend_curve(arc.end(), arc.extrema(), arc.approx_length());
            }
        }

        self.state
            .set_previous_control_points(next_cubic_cp2, next_quadratic_cp);

        Ok(())
    }

    fn evaluate_and_render(&mut self, ctx: &impl ContextView) -> Result<String> {
        self.reset();
        self.tokens.skip_whitespace();
        let mut output = String::new();

        while !self.tokens.at_end() {
            let parsed = self.parse_instruction(ctx)?;
            output.push_str(&parsed.instruction.render(
                &parsed.source,
                parsed.command,
                &parsed.state_before,
                &parsed.patches,
            ));
            self.apply_instruction(&parsed.instruction)?;
        }

        Ok(output.trim_end().to_string())
    }

    fn evaluate(&mut self, ctx: &impl ContextView) -> Result<()> {
        self.tokens.skip_whitespace();
        while !self.tokens.at_end() {
            self.process_instruction(ctx)?;
        }
        Ok(())
    }

    fn full_length(&mut self, ctx: &impl ContextView) -> Result<f32> {
        // TODO: memoize?
        self.evaluate(ctx)?;
        Ok(self.length_so_far())
    }

    fn probe_instruction_at_ratio(&mut self, ratio: f32, ctx: &impl ContextView) -> Result<Vec2> {
        let command = self.state.read_instruction_command(&mut self.tokens)?;

        let instruction = Command::from_tokens(&mut self.tokens, ctx, command, &self.state)?;

        instruction.point_at_ratio(ratio)
    }

    pub fn point_at_offset(&mut self, offset: Length) -> Result<Vec2> {
        let ctx = TransformerContext::default();
        self.reset();

        // if offset is ratio, get path length and convert to absolute
        let target_distance = match offset {
            Length::Absolute(v) => v,
            Length::Ratio(_) | Length::Rational(_, _) => {
                let td = offset.evaluate(self.full_length(&ctx)?);
                self.reset();
                td
            }
        };

        self.tokens.skip_whitespace();
        while !self.tokens.at_end() {
            let snapshot = self.clone();
            let old_length = self.length_so_far();
            self.process_instruction(&ctx)?;
            let new_length = self.length_so_far();
            if (new_length - target_distance).abs() < 1e-6 {
                // target point is ~exactly at the end of a command.
                break;
            } else if new_length > target_distance {
                let contribution = new_length - old_length;
                if contribution <= f32::EPSILON {
                    // Zero-length command (e.g. move/bearing) can still satisfy
                    // clamping for negative offsets. Probe this command directly.
                    *self = snapshot;
                    return self.probe_instruction_at_ratio(0., &ctx);
                }
                // how far into the command to reach the target offset
                let ratio = (target_distance - old_length) / contribution;
                if ratio < 0. {
                    // may happen if offset is negative; we'll return the first
                    // position point since we've processed a command.
                    break;
                }
                // gone too far; rewind to snapshot and evaluate
                // just this command to find exact point at offset
                *self = snapshot;
                return self.probe_instruction_at_ratio(ratio, &ctx);
            }
        }

        Ok(self.state.position().unwrap_or_default())
    }
}

// TODO: update return type when callers know about Vec2
pub fn get_point_along_path(element: &SvgElement, offset: Length) -> Result<(f32, f32)> {
    if let Some(path_data) = element.get_attr("d") {
        let mut pp = PathParser::new(path_data, &TransformConfig::default());
        pp.point_at_offset(offset).map(|p| (p.x, p.y))
    } else {
        Err(Error::MissingAttr("d".to_string()))
    }
}

pub fn process_path_data(
    data: &str,
    ctx: &TransformerContext,
) -> Result<(String, Option<BoundingBox>)> {
    let mut parser = PathParser::new(data, &ctx.config);
    let d = parser.evaluate_and_render(ctx)?;
    Ok((d, parser.get_bbox()))
}

#[cfg(test)]
impl PathParser {
    pub fn process_instruction_default(&mut self) -> Result<()> {
        let ctx = TransformerContext::default();
        self.process_instruction(&ctx)
    }

    pub fn evaluate_default(&mut self) -> Result<()> {
        let ctx = TransformerContext::default();
        self.evaluate(&ctx)
    }

    pub fn full_length_default(&mut self) -> Result<f32> {
        let ctx = TransformerContext::default();
        self.full_length(&ctx)
    }

    pub fn at_end(&self) -> bool {
        self.tokens.at_end()
    }

    pub fn skip_whitespace(&mut self) {
        self.tokens.skip_whitespace();
    }

    pub fn position(&self) -> Option<Vec2> {
        self.state.position()
    }
}
