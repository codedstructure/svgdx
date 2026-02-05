mod corner_route;
mod elbow;
mod line;

pub use elbow::ElbowConnector;
pub use line::LineConnector;

use super::SvgElement;
use crate::context::ElementMap;
use crate::errors::Result;
use crate::geometry::LocSpec;

/// Check if an element is a connector (line or polyline with start/end attrs)
pub fn is_connector(el: &SvgElement) -> bool {
    el.has_attr("start") && el.has_attr("end") && (el.name() == "line" || el.name() == "polyline")
}

/// Enum wrapping both connector types for unified handling
#[allow(clippy::large_enum_variant)]
pub enum ConnectorType {
    Line(LineConnector),
    Elbow(ElbowConnector),
}

impl ConnectorType {
    /// Factory function to create the appropriate connector type
    pub fn from_element(element: &SvgElement, elem_map: &impl ElementMap) -> Result<Self> {
        if element.name() == "polyline" {
            Ok(Self::Elbow(ElbowConnector::from_element(
                element, elem_map,
            )?))
        } else {
            Ok(Self::Line(LineConnector::from_element(element, elem_map)?))
        }
    }

    pub fn render(&self, ctx: &impl ElementMap) -> Result<Option<SvgElement>> {
        match self {
            Self::Line(c) => c.render(ctx),
            Self::Elbow(c) => c.render(ctx),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Direction {
    Up,
    Right,
    Down,
    Left,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Endpoint {
    pub origin: (f32, f32),
    pub dir: Option<Direction>,
}

impl Endpoint {
    pub(crate) const fn new(origin: (f32, f32), dir: Option<Direction>) -> Self {
        Self { origin, dir }
    }
}

/// Utility function to convert LocSpec to Direction
pub(crate) fn loc_to_dir(loc: LocSpec) -> Option<Direction> {
    match loc {
        LocSpec::Top | LocSpec::TopEdge(_) => Some(Direction::Up),
        LocSpec::Right | LocSpec::RightEdge(_) => Some(Direction::Right),
        LocSpec::Bottom | LocSpec::BottomEdge(_) => Some(Direction::Down),
        LocSpec::Left | LocSpec::LeftEdge(_) => Some(Direction::Left),
        _ => None,
    }
}
