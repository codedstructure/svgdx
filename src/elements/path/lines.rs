use super::Vec2;
use super::syntax::{PathSyntax, SvgPathSyntax};
use crate::Result;

pub(super) struct MoveTo {
    end: Vec2,
}

impl MoveTo {
    pub fn from_tokens(tokens: &mut SvgPathSyntax, start: Vec2, relative: bool) -> Result<Self> {
        // "(x y)+"
        let end = tokens.read_coord()?;
        let end = if relative { start + end } else { end };
        Ok(Self { end })
    }

    pub fn end(&self) -> Vec2 {
        self.end
    }
}

pub(super) struct LineTo {
    start: Vec2,
    end: Vec2,
}

impl LineTo {
    pub fn from_tokens(tokens: &mut SvgPathSyntax, start: Vec2, relative: bool) -> Result<Self> {
        // "(x y)+"
        let end = tokens.read_coord()?;
        let end = if relative { start + end } else { end };
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
}

pub(super) struct HorizontalLineTo {
    start: Vec2,
    end: Vec2,
}

impl HorizontalLineTo {
    pub fn from_tokens(tokens: &mut SvgPathSyntax, start: Vec2, relative: bool) -> Result<Self> {
        // "x+"
        let x = tokens.read_number()?;
        let end_x = if relative { start.x + x } else { x };
        let end = Vec2::new(end_x, start.y);
        Ok(Self { start, end })
    }

    pub fn end(&self) -> Vec2 {
        self.end
    }

    pub fn point_at_ratio(&self, ratio: f32) -> Vec2 {
        self.start + ratio * (self.end - self.start)
    }
}

pub(super) struct VerticalLineTo {
    start: Vec2,
    end: Vec2,
}

impl VerticalLineTo {
    pub fn from_tokens(tokens: &mut SvgPathSyntax, start: Vec2, relative: bool) -> Result<Self> {
        // "y+"
        let y = tokens.read_number()?;
        let end_y = if relative { start.y + y } else { y };
        let end = Vec2::new(start.x, end_y);
        Ok(Self { start, end })
    }

    pub fn end(&self) -> Vec2 {
        self.end
    }

    pub fn point_at_ratio(&self, ratio: f32) -> Vec2 {
        self.start + ratio * (self.end - self.start)
    }
}
