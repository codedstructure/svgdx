use super::command::Command;
use super::state::PathState;
use super::{SvgElement, Vec2};
use crate::errors::{Error, Result};
use crate::geometry::{BoundingBox, Length};

use super::syntax::{PathSyntax, SvgPathSyntax};

struct ParsedInstruction {
    command: char,
    source: String,
    state_before: PathState,
    instruction: Command,
}

#[derive(Clone)]
pub(super) struct PathParser {
    tokens: SvgPathSyntax, // TODO: ref to make clone cheaper?
    state: PathState,
}

impl PathParser {
    pub fn new(data: &str) -> Self {
        PathParser {
            tokens: SvgPathSyntax::new(data),
            state: PathState::new(),
        }
    }

    fn reset(&mut self) {
        self.tokens.reset();
        self.state.reset();
    }

    pub fn get_bbox(&self) -> Option<BoundingBox> {
        self.state.get_bbox()
    }

    pub fn length_so_far(&self) -> f32 {
        self.state.length_so_far()
    }

    pub fn process_instruction(&mut self) -> Result<()> {
        let parsed = self.parse_instruction()?;
        self.apply_instruction(&parsed.instruction);
        Ok(())
    }

    fn parse_instruction(&mut self) -> Result<ParsedInstruction> {
        let source_start = self.tokens.index();
        let state_before = self.state;
        // Smooth curve reflection only applies when the immediately preceding
        // instruction was the matching bezier type, so every other instruction
        // must clear the stored control points after it is processed.
        let command = self.state.read_instruction_command(&mut self.tokens)?;

        let instruction = Command::from_tokens(&mut self.tokens, command, &self.state)?;
        let source_end = self.tokens.index();
        let source = self.tokens.slice(source_start, source_end);

        Ok(ParsedInstruction {
            command,
            source,
            state_before,
            instruction,
        })
    }

    fn apply_instruction(&mut self, instruction: &Command) {
        let next_cubic_cp2 = instruction.next_cubic_cp2();
        let next_quadratic_cp = instruction.next_quadratic_cp();

        match instruction {
            Command::Bearing(bearing) => {
                self.state.set_bearing(bearing.bearing());
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
    }

    pub fn evaluate_and_render(&mut self) -> Result<String> {
        self.reset();
        self.tokens.skip_whitespace();
        let mut output = String::new();

        while !self.tokens.at_end() {
            let parsed = self.parse_instruction()?;
            output.push_str(&parsed.instruction.render(
                &parsed.source,
                parsed.command,
                &parsed.state_before,
            ));
            self.apply_instruction(&parsed.instruction);
        }

        Ok(output)
    }

    pub fn evaluate(&mut self) -> Result<()> {
        self.tokens.skip_whitespace();
        while !self.tokens.at_end() {
            self.process_instruction()?;
        }
        Ok(())
    }

    pub fn full_length(&mut self) -> Result<f32> {
        // TODO: memoize?
        self.evaluate()?;
        Ok(self.length_so_far())
    }

    fn probe_instruction_at_ratio(&mut self, ratio: f32) -> Result<Vec2> {
        let command = self.state.read_instruction_command(&mut self.tokens)?;

        let instruction = Command::from_tokens(&mut self.tokens, command, &self.state)?;

        Ok(instruction.point_at_ratio(ratio))
    }

    pub fn point_at_offset(&mut self, offset: Length) -> Result<Vec2> {
        self.reset();

        // if offset is ratio, get path length and convert to absolute
        let target_distance = match offset {
            Length::Absolute(v) => v,
            Length::Ratio(_) | Length::Rational(_, _) => {
                let td = offset.evaluate(self.full_length()?);
                self.reset();
                td
            }
        };

        self.tokens.skip_whitespace();
        while !self.tokens.at_end() {
            let snapshot = self.clone();
            let old_length = self.length_so_far();
            self.process_instruction()?;
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
                    return self.probe_instruction_at_ratio(0.);
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
                return self.probe_instruction_at_ratio(ratio);
            }
        }

        Ok(self.state.position().unwrap_or_default())
    }
}

// TODO: update return type when callers know about Vec2
pub fn get_point_along_path(element: &SvgElement, offset: Length) -> Result<(f32, f32)> {
    if let Some(path_data) = element.get_attr("d") {
        let mut pp = PathParser::new(path_data);
        pp.point_at_offset(offset).map(|p| (p.x, p.y))
    } else {
        Err(Error::MissingAttr("d".to_string()))
    }
}

pub fn process_path_data(data: &str) -> Result<(String, Option<BoundingBox>)> {
    let mut parser = PathParser::new(data);
    let d = parser.evaluate_and_render()?;
    Ok((d, parser.get_bbox()))
}

#[cfg(test)]
impl PathParser {
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
