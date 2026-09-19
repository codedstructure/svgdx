use super::Vec2;
use super::arc::Arc;
use super::bezier::{CubicBezier, QuadraticBezier};
use super::lines::{Bearing, HorizontalLineTo, LineTo, MoveTo, VerticalLineTo};
use super::repeat::Repeat;
use super::state::PathState;
use super::syntax::SvgPathSyntax;
use crate::errors::{Error, Result};

pub(super) enum Command {
    Bearing(Bearing),
    Repeat(Repeat),
    EndRepeat,
    MoveTo(MoveTo),
    LineTo(LineTo),
    HorizontalLineTo(HorizontalLineTo),
    VerticalLineTo(VerticalLineTo),
    ClosePath(LineTo),
    CubicBezier(CubicBezier),
    QuadraticBezier(QuadraticBezier),
    Arc(Arc),
}

impl Command {
    pub fn from_tokens(
        tokens: &mut SvgPathSyntax,
        command: char,
        state: &PathState,
    ) -> Result<Self> {
        let is_relative = command.is_lowercase();

        Ok(match command {
            'B' | 'b' => Self::Bearing(Bearing::from_tokens(tokens, state, is_relative)?),
            'R' | 'r' => Self::Repeat(Repeat::from_tokens(tokens)?),
            ']' => Self::EndRepeat,
            'M' | 'm' => Self::MoveTo(MoveTo::from_tokens(tokens, state, is_relative)?),
            'L' | 'l' => Self::LineTo(LineTo::from_tokens(tokens, state, is_relative)?),
            'H' | 'h' => {
                Self::HorizontalLineTo(HorizontalLineTo::from_tokens(tokens, state, is_relative)?)
            }
            'V' | 'v' => {
                Self::VerticalLineTo(VerticalLineTo::from_tokens(tokens, state, is_relative)?)
            }
            'Z' | 'z' => {
                let end = state.subpath_start().unwrap_or_default();
                Self::ClosePath(LineTo::from_endpoints(state.current_position(), end))
            }
            'C' | 'c' => Self::CubicBezier(CubicBezier::from_tokens(tokens, state, is_relative)?),
            'S' | 's' => {
                Self::CubicBezier(CubicBezier::from_smooth_tokens(tokens, state, is_relative)?)
            }
            'Q' | 'q' => {
                Self::QuadraticBezier(QuadraticBezier::from_tokens(tokens, state, is_relative)?)
            }
            'T' | 't' => Self::QuadraticBezier(QuadraticBezier::from_smooth_tokens(
                tokens,
                state,
                is_relative,
            )?),
            'A' | 'a' => Self::Arc(Arc::from_tokens(tokens, state, is_relative)?),
            _ => {
                return Err(Error::InvalidValue(
                    "path command".to_string(),
                    command.to_string(),
                ));
            }
        })
    }

    pub fn point_at_ratio(&self, ratio: f32) -> Result<Vec2> {
        Ok(match self {
            Self::MoveTo(seg) => seg.end(),
            Self::LineTo(seg) => seg.point_at_ratio(ratio),
            Self::HorizontalLineTo(seg) => seg.point_at_ratio(ratio),
            Self::VerticalLineTo(seg) => seg.point_at_ratio(ratio),
            Self::ClosePath(seg) => seg.point_at_ratio(ratio),
            Self::CubicBezier(seg) => seg.point_at_ratio(ratio),
            Self::QuadraticBezier(seg) => seg.point_at_ratio(ratio),
            Self::Arc(seg) => seg.point_at_ratio(ratio),

            // in practice these commands will already have been resolved
            Self::Bearing(_) | Self::Repeat(_) | Self::EndRepeat => {
                return Err(Error::InvalidAttr(
                    "path extensions must be resolved".into(),
                ));
            }
        })
    }

    pub fn render(&self, source: &str, source_command: char, state_before: &PathState) -> String {
        // Bearing and repeat commands do not contribute to output
        if matches!(self, Self::Bearing(_) | Self::Repeat(_) | Self::EndRepeat) {
            return String::new();
        }
        // preserve source if possible (bearing == 0), else render as required
        let bearing = state_before.bearing().unwrap_or(0.);
        let b_non_zero = bearing.abs() > f32::EPSILON;
        let relative_line_or_move = matches!(source_command, 'm' | 'l' | 'h' | 'v');

        if b_non_zero && relative_line_or_move {
            return match self {
                Self::MoveTo(seg) if source_command == 'm' => seg.render_relative(),
                Self::LineTo(seg) if source_command == 'l' => seg.render_relative(),
                Self::HorizontalLineTo(seg) if source_command == 'h' => {
                    seg.render_as_relative_line()
                }
                Self::VerticalLineTo(seg) if source_command == 'v' => seg.render_as_relative_line(),
                _ => source.to_string(), // unexpected: relative_line_or_move should be handled above
            };
        }
        source.to_string()
    }

    pub fn next_cubic_cp2(&self) -> Option<Vec2> {
        match self {
            Self::CubicBezier(curve) => Some(curve.control_point_2()),
            _ => None,
        }
    }

    pub fn next_quadratic_cp(&self) -> Option<Vec2> {
        match self {
            Self::QuadraticBezier(curve) => Some(curve.control_point()),
            _ => None,
        }
    }
}
