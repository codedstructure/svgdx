use super::arc::Arc;
use super::bezier::{CubicBezier, QuadraticBezier};
use super::{SvgElement, Vec2};
use crate::errors::{Error, Result};
use crate::geometry::{BoundingBox, Length};

use super::syntax::{PathSyntax, SvgPathSyntax};

#[derive(Clone)]
pub(super) struct PathParser {
    tokens: SvgPathSyntax, // TODO: ref to make clone cheaper?
    // current position, updated as commands are processed
    position: Option<Vec2>,
    // location to return to for 'Z'/'z' commands
    subpath_start: Option<Vec2>,
    // current command being processed; most commands take multiple parameter
    // sets without repeating the command character
    command: Option<char>,
    // previous second control point (if any) for evaluating 'S' and 's'
    cubic_cp2: Option<Vec2>,
    // previous control point (if any) for evaluating 'T' and 't'
    quadratic_cp: Option<Vec2>,
    // distance along the path so far, for line-offset
    elapsed_distance: f32,
    // extrema, updated as path is processed
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

impl PathParser {
    pub fn new(data: &str) -> Self {
        PathParser {
            tokens: SvgPathSyntax::new(data),
            position: None,
            subpath_start: None,
            command: None,
            cubic_cp2: None,
            quadratic_cp: None,
            elapsed_distance: 0.,
            min_x: 0.,
            min_y: 0.,
            max_x: 0.,
            max_y: 0.,
        }
    }

    fn reset(&mut self) {
        self.tokens.reset();
        self.position = None;
        self.subpath_start = None;
        self.command = None;
        self.cubic_cp2 = None;
        self.quadratic_cp = None;
        self.elapsed_distance = 0.;
        self.min_x = 0.;
        self.min_y = 0.;
        self.max_x = 0.;
        self.max_y = 0.;
    }

    fn extend_extrema(&mut self, pos: Vec2) {
        let (x, y) = (pos.x, pos.y);
        if self.position.is_none() {
            self.min_x = x;
            self.min_y = y;
            self.max_x = x;
            self.max_y = y;
        } else {
            self.min_x = self.min_x.min(x);
            self.min_y = self.min_y.min(y);
            self.max_x = self.max_x.max(x);
            self.max_y = self.max_y.max(y);
        }
    }

    fn extend_subpath(&mut self, pos: Vec2) {
        self.extend_extrema(pos);

        let old = self.position.unwrap_or(pos);
        self.elapsed_distance += pos.distance(old);

        self.position = Some(pos);
    }

    fn extend_curve<I>(&mut self, end: Vec2, extrema: I, length: f32)
    where
        I: IntoIterator<Item = Vec2>,
    {
        for point in extrema {
            self.extend_extrema(point);
        }
        self.extend_extrema(end);
        self.elapsed_distance += length;
        self.position = Some(end);
    }

    fn new_subpath(&mut self, pos: Vec2) {
        self.extend_extrema(pos);
        // note we don't add to elapsed_distance here; instantly jump to the new position.
        self.position = Some(pos);

        self.subpath_start = Some(pos);
    }

    pub fn get_bbox(&self) -> Option<BoundingBox> {
        if self.position.is_some() {
            Some(BoundingBox::new(
                self.min_x, self.min_y, self.max_x, self.max_y,
            ))
        } else {
            None // we've never called extend_subpath()
        }
    }

    pub fn length_so_far(&self) -> f32 {
        self.elapsed_distance
    }

    fn read_instruction_command(&mut self) -> Result<char> {
        if self.command.is_none() || self.tokens.at_command()? {
            // "The command letter can be eliminated on subsequent commands if the same
            // command is used multiple times in a row (e.g., you can drop the second
            // "L" in "M 100 200 L 200 100 L -100 -200" and use "M 100 200 L 200 100
            // -100 -200" instead)."
            self.command = Some(self.tokens.read_command()?);
        } else {
            // this will only happen for subsequent values to an existing command
            match self.command {
                // "If a moveto is followed by multiple pairs of coordinates,
                // the subsequent pairs are treated as implicit lineto commands."
                Some('m') => {
                    self.command = Some('l');
                }
                Some('M') => {
                    self.command = Some('L');
                }
                _ => {}
            }
        }

        Ok(self.command.expect("Command should be already set"))
    }

    pub fn process_instruction(&mut self) -> Result<()> {
        let previous_cubic_cp2 = self.cubic_cp2;
        let previous_quadratic_cp = self.quadratic_cp;
        // Smooth curve reflection only applies when the immediately preceding
        // instruction was the matching bezier type, so every other instruction
        // must clear the stored control points after it is processed.
        let mut next_cubic_cp2: Option<Vec2> = None;
        let mut next_quadratic_cp: Option<Vec2> = None;

        let command = self.read_instruction_command()?;
        let is_relative = command.is_lowercase();
        let pos = self.position.unwrap_or_default();

        match command {
            'M' => {
                // "(x y)+"
                let xy = self.tokens.read_coord()?;
                // 'Subsequent "moveto" commands (i.e., when the "moveto" is not
                // the first command) represent the start of a new subpath'
                // (the first moveto is also the start of a subpath)
                self.new_subpath(xy);
            }
            'm' => {
                // "(x y)+"
                let delta = self.tokens.read_coord()?;
                // 'Subsequent "moveto" commands (i.e., when the "moveto" is not
                // the first command) represent the start of a new subpath'
                self.new_subpath(pos + delta);
            }
            'L' => {
                // "(x y)+"
                let xy = self.tokens.read_coord()?;
                self.extend_subpath(xy);
            }
            'l' => {
                // "(x y)+"
                let delta = self.tokens.read_coord()?;
                self.extend_subpath(pos + delta);
            }
            'H' => {
                // "x+"
                let new_x = self.tokens.read_number()?;
                self.extend_subpath(Vec2::new(new_x, pos.y));
            }
            'h' => {
                // "x+"
                let dx = self.tokens.read_number()?;
                self.extend_subpath(pos + Vec2::new(dx, 0.));
            }
            'V' => {
                // "y+"
                let new_y = self.tokens.read_number()?;
                self.extend_subpath(Vec2::new(pos.x, new_y));
            }
            'v' => {
                // "y+"
                let dy = self.tokens.read_number()?;
                self.extend_subpath(pos + Vec2::new(0., dy));
            }
            'Z' | 'z' => {
                self.extend_subpath(self.subpath_start.unwrap_or_default());
                // since this doesn't consume further tokens, we must clear the command
                // to force getting a new command token, or we could loop forever
                self.command = None;
            }
            'C' | 'c' => {
                let curve = CubicBezier::from_tokens(&mut self.tokens, pos, is_relative)?;

                next_cubic_cp2 = Some(curve.control_point_2());
                self.extend_curve(curve.end(), curve.extrema(), curve.approx_length());
            }
            'S' | 's' => {
                let curve = CubicBezier::from_smooth_tokens(
                    &mut self.tokens,
                    pos,
                    previous_cubic_cp2,
                    is_relative,
                )?;

                next_cubic_cp2 = Some(curve.control_point_2());
                self.extend_curve(curve.end(), curve.extrema(), curve.approx_length());
            }
            'Q' | 'q' => {
                let curve = QuadraticBezier::from_tokens(&mut self.tokens, pos, is_relative)?;

                next_quadratic_cp = Some(curve.control_point());
                self.extend_curve(curve.end(), curve.extrema(), curve.approx_length());
            }
            'T' | 't' => {
                let curve = QuadraticBezier::from_smooth_tokens(
                    &mut self.tokens,
                    pos,
                    previous_quadratic_cp,
                    is_relative,
                )?;

                next_quadratic_cp = Some(curve.control_point());
                self.extend_curve(curve.end(), curve.extrema(), curve.approx_length());
            }
            'A' | 'a' => {
                // "(rx ry x-axis-rotation large-arc-flag sweep-flag x y)+"
                let arc = Arc::from_tokens(&mut self.tokens, pos, is_relative)?;

                self.extend_curve(arc.end(), arc.extrema(), arc.approx_length());
            }
            _ => Err(Error::InvalidValue(
                "path command".to_string(),
                self.command.unwrap_or_default().to_string(),
            ))?,
        }
        self.cubic_cp2 = next_cubic_cp2;
        self.quadratic_cp = next_quadratic_cp;
        Ok(())
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
        let command = self.read_instruction_command()?;

        let is_relative = command.is_lowercase();

        // point_at_offset rewinds to the parser snapshot taken before this instruction,
        // so the stored control-point state already matches the smooth-curve context.
        match command {
            'L' => {
                let pos = self.position.unwrap_or_default();
                let target = self.tokens.read_coord()?;
                let delta = target - pos;
                Ok(pos + ratio * delta)
            }
            'l' => {
                let pos = self.position.unwrap_or_default();
                let delta = self.tokens.read_coord()?;
                Ok(pos + ratio * delta)
            }
            'H' => {
                let pos = self.position.unwrap_or_default();
                let new_x = self.tokens.read_number()?;
                let target = Vec2::new(new_x, pos.y);
                Ok(pos + ratio * (target - pos))
            }
            'h' => {
                let pos = self.position.unwrap_or_default();
                let dx = self.tokens.read_number()?;
                Ok(pos + ratio * Vec2::new(dx, 0.))
            }
            'V' => {
                let pos = self.position.unwrap_or_default();
                let new_y = self.tokens.read_number()?;
                let target = Vec2::new(pos.x, new_y);
                Ok(pos + ratio * (target - pos))
            }
            'v' => {
                let pos = self.position.unwrap_or_default();
                let dy = self.tokens.read_number()?;
                Ok(pos + ratio * Vec2::new(0., dy))
            }
            'z' | 'Z' => {
                let pos = self.position.unwrap_or_default();
                let target = self.subpath_start.unwrap_or(pos);
                Ok(pos + ratio * (target - pos))
            }
            'C' | 'c' => Ok(CubicBezier::from_tokens(
                &mut self.tokens,
                self.position.unwrap_or_default(),
                is_relative,
            )?
            .approx_point_at_ratio(ratio)),
            'S' | 's' => Ok(CubicBezier::from_smooth_tokens(
                &mut self.tokens,
                self.position.unwrap_or_default(),
                self.cubic_cp2,
                is_relative,
            )?
            .approx_point_at_ratio(ratio)),
            'Q' | 'q' => Ok(QuadraticBezier::from_tokens(
                &mut self.tokens,
                self.position.unwrap_or_default(),
                is_relative,
            )?
            .approx_point_at_ratio(ratio)),
            'T' | 't' => Ok(QuadraticBezier::from_smooth_tokens(
                &mut self.tokens,
                self.position.unwrap_or_default(),
                self.quadratic_cp,
                is_relative,
            )?
            .approx_point_at_ratio(ratio)),
            'A' | 'a' => {
                let arc = Arc::from_tokens(
                    &mut self.tokens,
                    self.position.unwrap_or_default(),
                    is_relative,
                )?;

                Ok(arc.approx_point_at_ratio(ratio))
            }
            _ => Err(Error::InvalidValue(
                "path command".to_string(),
                self.command.unwrap_or_default().to_string(),
            ))?,
        }
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

        Ok(self.position.unwrap_or_default())
    }
}

pub fn path_bbox(element: &SvgElement) -> Result<Option<BoundingBox>> {
    if let Some(path_data) = element.get_attr("d") {
        let mut pp = PathParser::new(path_data);
        pp.evaluate()?;
        Ok(pp.get_bbox())
    } else {
        Ok(None)
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

#[cfg(test)]
impl PathParser {
    pub fn at_end(&self) -> bool {
        self.tokens.at_end()
    }

    pub fn skip_whitespace(&mut self) {
        self.tokens.skip_whitespace();
    }

    pub fn position(&self) -> Option<Vec2> {
        self.position
    }
}
