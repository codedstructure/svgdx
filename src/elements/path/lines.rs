use super::Vec2;
use super::state::PathState;
use super::syntax::{PathSyntax, SvgPathSyntax};
use crate::Result;
use crate::context::ContextView;
use crate::types::fstr;

// bearing commands only affect lines / moves, hence in this module.
pub(super) struct Bearing {
    bearing: f32,
}

impl Bearing {
    pub fn from_tokens(
        tokens: &mut SvgPathSyntax,
        ctx: &impl ContextView,
        state: &PathState,
        relative: bool,
    ) -> Result<Self> {
        let value = tokens.read_number(ctx)?;
        let bearing = if relative {
            state.bearing().unwrap_or(0.) + value
        } else {
            value
        };
        Ok(Self { bearing })
    }

    pub fn bearing(&self) -> f32 {
        self.bearing
    }
}

pub(super) struct MoveTo {
    start: Vec2,
    end: Vec2,
}

impl MoveTo {
    pub fn from_tokens(
        tokens: &mut SvgPathSyntax,
        ctx: &impl ContextView,
        state: &PathState,
        relative: bool,
    ) -> Result<Self> {
        // "(x y)+"
        let end = tokens.read_coord(ctx)?;
        let end = if relative {
            state.current_position() + state.apply_bearing(end)
        } else {
            end
        };
        Ok(Self {
            start: state.current_position(),
            end,
        })
    }

    pub fn end(&self) -> Vec2 {
        self.end
    }

    pub fn render_relative(&self) -> String {
        // Keep formatting simple; this can be optimized later if needed.
        let delta = self.end - self.start;
        format!("m{} {}", fstr(delta.x), fstr(delta.y))
    }
}

pub(super) struct LineTo {
    start: Vec2,
    end: Vec2,
}

impl LineTo {
    pub fn from_tokens(
        tokens: &mut SvgPathSyntax,
        ctx: &impl ContextView,
        state: &PathState,
        relative: bool,
    ) -> Result<Self> {
        // "(x y)+"
        let end = tokens.read_coord(ctx)?;
        let start = state.current_position();
        let end = if relative {
            start + state.apply_bearing(end)
        } else {
            end
        };
        Ok(Self { start, end })
    }

    pub fn from_endpoints(start: Vec2, end: Vec2) -> Self {
        Self { start, end }
    }

    pub fn end(&self) -> Vec2 {
        self.end
    }

    pub fn point_at_ratio(&self, ratio: f32) -> Vec2 {
        self.start + ratio * (self.end - self.start)
    }

    pub fn render_relative(&self) -> String {
        let delta = self.end - self.start;
        format!("l{} {}", fstr(delta.x), fstr(delta.y))
    }
}

pub(super) struct HorizontalLineTo {
    start: Vec2,
    end: Vec2,
}

impl HorizontalLineTo {
    pub fn from_tokens(
        tokens: &mut SvgPathSyntax,
        ctx: &impl ContextView,
        state: &PathState,
        relative: bool,
    ) -> Result<Self> {
        // "x+"
        let x = tokens.read_number(ctx)?;
        let start = state.current_position();
        let end = if relative {
            // "When a relative h command is used, the end point of the line is
            // (cpx + x cos cb, cpy + x sin cb)."
            start + state.apply_bearing(Vec2::new(x, 0.))
        } else {
            Vec2::new(x, start.y)
        };
        Ok(Self { start, end })
    }

    pub fn end(&self) -> Vec2 {
        self.end
    }

    pub fn point_at_ratio(&self, ratio: f32) -> Vec2 {
        self.start + ratio * (self.end - self.start)
    }

    pub fn render_as_relative_line(&self) -> String {
        let delta = self.end - self.start;
        format!("l{} {}", fstr(delta.x), fstr(delta.y))
    }
}

pub(super) struct VerticalLineTo {
    start: Vec2,
    end: Vec2,
}

impl VerticalLineTo {
    pub fn from_tokens(
        tokens: &mut SvgPathSyntax,
        ctx: &impl ContextView,
        state: &PathState,
        relative: bool,
    ) -> Result<Self> {
        // "y+"
        let y = tokens.read_number(ctx)?;
        let start = state.current_position();
        let end = if relative {
            // "When a relative v command is used, the end point of the line is
            // (cpx + y sin cb, cpy + y cos cb)."
            start + state.apply_bearing(Vec2::new(0., y))
        } else {
            Vec2::new(start.x, y)
        };
        Ok(Self { start, end })
    }

    pub fn end(&self) -> Vec2 {
        self.end
    }

    pub fn point_at_ratio(&self, ratio: f32) -> Vec2 {
        self.start + ratio * (self.end - self.start)
    }

    pub fn render_as_relative_line(&self) -> String {
        let delta = self.end - self.start;
        format!("l{} {}", fstr(delta.x), fstr(delta.y))
    }
}
