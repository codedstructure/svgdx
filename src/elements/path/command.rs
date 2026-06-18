use super::Vec2;
use super::arc::Arc;
use super::bezier::{CubicBezier, QuadraticBezier};
use super::lines::{HorizontalLineTo, LineTo, MoveTo, VerticalLineTo};
use super::syntax::SvgPathSyntax;
use crate::errors::{Error, Result};

pub(super) enum Command {
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
        start: Vec2,
        subpath_start: Option<Vec2>,
        previous_cubic_cp2: Option<Vec2>,
        previous_quadratic_cp: Option<Vec2>,
    ) -> Result<Self> {
        let is_relative = command.is_lowercase();

        Ok(match command {
            'M' | 'm' => Self::MoveTo(MoveTo::from_tokens(tokens, start, is_relative)?),
            'L' | 'l' => Self::LineTo(LineTo::from_tokens(tokens, start, is_relative)?),
            'H' | 'h' => {
                Self::HorizontalLineTo(HorizontalLineTo::from_tokens(tokens, start, is_relative)?)
            }
            'V' | 'v' => {
                Self::VerticalLineTo(VerticalLineTo::from_tokens(tokens, start, is_relative)?)
            }
            'Z' | 'z' => {
                let end = subpath_start.unwrap_or_default();
                Self::ClosePath(LineTo::from_endpoints(start, end))
            }
            'C' | 'c' => Self::CubicBezier(CubicBezier::from_tokens(tokens, start, is_relative)?),
            'S' | 's' => Self::CubicBezier(CubicBezier::from_smooth_tokens(
                tokens,
                start,
                previous_cubic_cp2,
                is_relative,
            )?),
            'Q' | 'q' => {
                Self::QuadraticBezier(QuadraticBezier::from_tokens(tokens, start, is_relative)?)
            }
            'T' | 't' => Self::QuadraticBezier(QuadraticBezier::from_smooth_tokens(
                tokens,
                start,
                previous_quadratic_cp,
                is_relative,
            )?),
            'A' | 'a' => Self::Arc(Arc::from_tokens(tokens, start, is_relative)?),
            _ => {
                return Err(Error::InvalidValue(
                    "path command".to_string(),
                    command.to_string(),
                ));
            }
        })
    }

    pub fn point_at_ratio(&self, ratio: f32) -> Vec2 {
        match self {
            Self::MoveTo(seg) => seg.end(),
            Self::LineTo(seg) => seg.point_at_ratio(ratio),
            Self::HorizontalLineTo(seg) => seg.point_at_ratio(ratio),
            Self::VerticalLineTo(seg) => seg.point_at_ratio(ratio),
            Self::ClosePath(seg) => seg.point_at_ratio(ratio),
            Self::CubicBezier(seg) => seg.point_at_ratio(ratio),
            Self::QuadraticBezier(seg) => seg.point_at_ratio(ratio),
            Self::Arc(seg) => seg.point_at_ratio(ratio),
        }
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
