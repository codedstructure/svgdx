use std::fmt::Display;
use std::str::FromStr;

use crate::errors::{Error, Result};
use crate::geometry::{BoundingBox, LocSpec};

/// Type of connector `connect` attribute value, used for determining
/// connection points to use when not fully specified.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum ConnectMode {
    #[default]
    Default,
    Cardinal,
}

impl ConnectMode {
    pub const fn line_strategy(self) -> AutoConnectStrategy {
        match self {
            Self::Default => AutoConnectStrategy::OverlapOrCorner,
            Self::Cardinal => AutoConnectStrategy::Cardinal,
        }
    }

    pub const fn polyline_strategy(self) -> AutoConnectStrategy {
        match self {
            Self::Default | Self::Cardinal => AutoConnectStrategy::Cardinal,
        }
    }
}

impl FromStr for ConnectMode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            s if s == Self::Default.to_string() => Ok(Self::Default),
            s if s == Self::Cardinal.to_string() => Ok(Self::Cardinal),
            _ => Err(Error::InvalidValue("connect".into(), s.into())),
        }
    }
}

impl Display for ConnectMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Default => "default",
            Self::Cardinal => "cardinal",
        };
        f.write_str(s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AutoConnectStrategy {
    OverlapOrCorner,
    Cardinal,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct PointRouting {
    pub target: ResolvedTarget,
    pub fully_overlapping: bool,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct ResolvedTarget {
    pub loc: Option<LocSpec>,
    pub coord: (f32, f32),
}

#[derive(Clone, Copy, Debug)]
pub(super) struct BBoxRouting {
    pub start: ResolvedTarget,
    pub end: ResolvedTarget,
    pub fully_overlapping: bool,
}

/// Returns the midpoint of two 1D ranges if they overlap, None otherwise.
fn range_overlap(min1: f32, max1: f32, min2: f32, max2: f32) -> Option<f32> {
    let overlap_min = min1.max(min2);
    let overlap_max = max1.min(max2);
    if overlap_min <= overlap_max {
        Some((overlap_min + overlap_max) / 2.0)
    } else {
        None
    }
}

/// Select appropriate corner LocSpecs for connecting two bboxes with no overlap.
fn select_corners(start_bb: &BoundingBox, end_bb: &BoundingBox) -> (LocSpec, LocSpec) {
    let end_is_right = end_bb.cx() > start_bb.cx();
    let end_is_below = end_bb.cy() > start_bb.cy();

    match (end_is_right, end_is_below) {
        (true, true) => (LocSpec::BottomRight, LocSpec::TopLeft),
        (true, false) => (LocSpec::TopRight, LocSpec::BottomLeft),
        (false, true) => (LocSpec::BottomLeft, LocSpec::TopRight),
        (false, false) => (LocSpec::TopLeft, LocSpec::BottomRight),
    }
}

/// Select appropriate corner LocSpec for connecting a point to a bbox with no overlap.
fn select_corner_for_point(point: (f32, f32), bb: &BoundingBox) -> LocSpec {
    let point_is_right = point.0 > bb.cx();
    let point_is_below = point.1 > bb.cy();

    match (point_is_right, point_is_below) {
        (true, true) => LocSpec::BottomRight,
        (true, false) => LocSpec::TopRight,
        (false, true) => LocSpec::BottomLeft,
        (false, false) => LocSpec::TopLeft,
    }
}

/// For cardinal routing, find the shortest link using cardinal directions only.
fn shortest_cardinal_link(start_bb: &BoundingBox, end_bb: &BoundingBox) -> (LocSpec, LocSpec) {
    const CARDINAL_LOCS: [LocSpec; 4] =
        [LocSpec::Top, LocSpec::Right, LocSpec::Bottom, LocSpec::Left];

    let mut min_dist_sq = f32::MAX;
    let mut start_min_loc = LocSpec::Right;
    let mut end_min_loc = LocSpec::Left;

    for start_loc in &CARDINAL_LOCS {
        for end_loc in &CARDINAL_LOCS {
            let start_coord = start_bb.locspec(*start_loc);
            let end_coord = end_bb.locspec(*end_loc);
            let (x1, y1) = start_coord;
            let (x2, y2) = end_coord;
            let dist_sq = (x1 - x2) * (x1 - x2) + (y1 - y2) * (y1 - y2);
            if dist_sq < min_dist_sq {
                min_dist_sq = dist_sq;
                start_min_loc = *start_loc;
                end_min_loc = *end_loc;
            }
        }
    }
    (start_min_loc, end_min_loc)
}

/// Find the closest cardinal direction from a point to a bbox.
fn closest_cardinal_loc(point: (f32, f32), bb: &BoundingBox) -> LocSpec {
    const CARDINAL_LOCS: [LocSpec; 4] =
        [LocSpec::Top, LocSpec::Right, LocSpec::Bottom, LocSpec::Left];

    let mut min_dist_sq = f32::MAX;
    let mut min_loc = LocSpec::Right;

    for loc in &CARDINAL_LOCS {
        let coord = bb.locspec(*loc);
        let dist_sq = (coord.0 - point.0).powi(2) + (coord.1 - point.1).powi(2);
        if dist_sq < min_dist_sq {
            min_dist_sq = dist_sq;
            min_loc = *loc;
        }
    }
    min_loc
}

/// Axis-agnostic overlap analysis result for one dimension.
#[derive(Debug, Clone, Copy)]
enum AxisOverlap {
    Overlapping(f32),
    Before,
    After,
}

impl AxisOverlap {
    fn from_ranges(min1: f32, max1: f32, min2: f32, max2: f32) -> Self {
        if let Some(mid) = range_overlap(min1, max1, min2, max2) {
            Self::Overlapping(mid)
        } else if max1 < min2 {
            Self::Before
        } else {
            Self::After
        }
    }

    fn is_overlapping(&self) -> bool {
        matches!(self, Self::Overlapping(_))
    }
}

/// Analyzed relationship between two bounding boxes.
#[derive(Debug)]
struct BBoxRelation {
    x_axis: AxisOverlap,
    y_axis: AxisOverlap,
}

impl BBoxRelation {
    fn new(bb1: &BoundingBox, bb2: &BoundingBox) -> Self {
        Self {
            x_axis: AxisOverlap::from_ranges(bb1.x1(), bb1.x2(), bb2.x1(), bb2.x2()),
            y_axis: AxisOverlap::from_ranges(bb1.y1(), bb1.y2(), bb2.y1(), bb2.y2()),
        }
    }

    fn from_point_and_bbox(point: (f32, f32), bb: &BoundingBox) -> Self {
        let point_bb = BoundingBox::new(point.0, point.1, point.0, point.1);
        Self::new(&point_bb, bb)
    }

    fn fully_overlapping(&self) -> bool {
        self.x_axis.is_overlapping() && self.y_axis.is_overlapping()
    }

    fn resolve_overlap_or_corner_bbox_to_bbox(
        &self,
        start_bb: &BoundingBox,
        end_bb: &BoundingBox,
    ) -> (LocSpec, (f32, f32), LocSpec, (f32, f32)) {
        match (&self.x_axis, &self.y_axis) {
            (AxisOverlap::Overlapping(x_mid), AxisOverlap::Before) => {
                let start_loc = LocSpec::Bottom;
                let end_loc = LocSpec::Top;
                (
                    start_loc,
                    (*x_mid, start_bb.locspec(start_loc).1),
                    end_loc,
                    (*x_mid, end_bb.locspec(end_loc).1),
                )
            }
            (AxisOverlap::Overlapping(x_mid), AxisOverlap::After) => {
                let start_loc = LocSpec::Top;
                let end_loc = LocSpec::Bottom;
                (
                    start_loc,
                    (*x_mid, start_bb.locspec(start_loc).1),
                    end_loc,
                    (*x_mid, end_bb.locspec(end_loc).1),
                )
            }
            (AxisOverlap::Before, AxisOverlap::Overlapping(y_mid)) => {
                let start_loc = LocSpec::Right;
                let end_loc = LocSpec::Left;
                (
                    start_loc,
                    (start_bb.locspec(start_loc).0, *y_mid),
                    end_loc,
                    (end_bb.locspec(end_loc).0, *y_mid),
                )
            }
            (AxisOverlap::After, AxisOverlap::Overlapping(y_mid)) => {
                let start_loc = LocSpec::Left;
                let end_loc = LocSpec::Right;
                (
                    start_loc,
                    (start_bb.locspec(start_loc).0, *y_mid),
                    end_loc,
                    (end_bb.locspec(end_loc).0, *y_mid),
                )
            }
            _ => {
                let (start_loc, end_loc) = select_corners(start_bb, end_bb);
                (
                    start_loc,
                    start_bb.locspec(start_loc),
                    end_loc,
                    end_bb.locspec(end_loc),
                )
            }
        }
    }

    fn resolve_overlap_or_corner_point_to_bbox(
        &self,
        point: (f32, f32),
        bb: &BoundingBox,
    ) -> (LocSpec, (f32, f32)) {
        match (&self.x_axis, &self.y_axis) {
            (AxisOverlap::Overlapping(_), AxisOverlap::Before) => {
                (LocSpec::Top, (point.0, bb.locspec(LocSpec::Top).1))
            }
            (AxisOverlap::Overlapping(_), AxisOverlap::After) => {
                (LocSpec::Bottom, (point.0, bb.locspec(LocSpec::Bottom).1))
            }
            (AxisOverlap::Before, AxisOverlap::Overlapping(_)) => {
                (LocSpec::Left, (bb.locspec(LocSpec::Left).0, point.1))
            }
            (AxisOverlap::After, AxisOverlap::Overlapping(_)) => {
                (LocSpec::Right, (bb.locspec(LocSpec::Right).0, point.1))
            }
            _ => {
                let loc = select_corner_for_point(point, bb);
                (loc, bb.locspec(loc))
            }
        }
    }
}

pub fn resolve_bbox_to_bbox(
    strategy: AutoConnectStrategy,
    start_bb: &BoundingBox,
    end_bb: &BoundingBox,
) -> BBoxRouting {
    let relation = BBoxRelation::new(start_bb, end_bb);
    let fully_overlapping = relation.fully_overlapping();

    match strategy {
        AutoConnectStrategy::OverlapOrCorner if fully_overlapping => BBoxRouting {
            start: ResolvedTarget {
                loc: None,
                coord: start_bb.locspec(LocSpec::Center),
            },
            end: ResolvedTarget {
                loc: None,
                coord: end_bb.locspec(LocSpec::Center),
            },
            fully_overlapping,
        },
        AutoConnectStrategy::OverlapOrCorner => {
            let (start_loc, start_coord, end_loc, end_coord) =
                relation.resolve_overlap_or_corner_bbox_to_bbox(start_bb, end_bb);
            BBoxRouting {
                start: ResolvedTarget {
                    loc: Some(start_loc),
                    coord: start_coord,
                },
                end: ResolvedTarget {
                    loc: Some(end_loc),
                    coord: end_coord,
                },
                fully_overlapping,
            }
        }
        AutoConnectStrategy::Cardinal => {
            let (start_loc, end_loc) = shortest_cardinal_link(start_bb, end_bb);
            BBoxRouting {
                start: ResolvedTarget {
                    loc: Some(start_loc),
                    coord: start_bb.locspec(start_loc),
                },
                end: ResolvedTarget {
                    loc: Some(end_loc),
                    coord: end_bb.locspec(end_loc),
                },
                fully_overlapping,
            }
        }
    }
}

pub fn resolve_point_to_bbox(
    strategy: AutoConnectStrategy,
    point: (f32, f32),
    bb: &BoundingBox,
) -> PointRouting {
    let relation = BBoxRelation::from_point_and_bbox(point, bb);
    let fully_overlapping = relation.fully_overlapping();

    match strategy {
        AutoConnectStrategy::OverlapOrCorner if fully_overlapping => PointRouting {
            target: ResolvedTarget {
                loc: None,
                coord: point,
            },
            fully_overlapping,
        },
        AutoConnectStrategy::OverlapOrCorner => {
            let (loc, coord) = relation.resolve_overlap_or_corner_point_to_bbox(point, bb);
            PointRouting {
                target: ResolvedTarget {
                    loc: Some(loc),
                    coord,
                },
                fully_overlapping,
            }
        }
        AutoConnectStrategy::Cardinal => {
            let loc = closest_cardinal_loc(point, bb);
            PointRouting {
                target: ResolvedTarget {
                    loc: Some(loc),
                    coord: bb.locspec(loc),
                },
                fully_overlapping,
            }
        }
    }
}
