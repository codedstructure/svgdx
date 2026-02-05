use super::SvgElement;
use crate::context::ElementMap;
use crate::elements::corner_route::render_match_corner;
use crate::errors::{Error, Result};
use crate::geometry::{parse_el_loc, strp_length, BoundingBox, ElementLoc, Length, LocSpec};
use crate::types::{attr_split_cycle, fstr, strp};

pub fn is_connector(el: &SvgElement) -> bool {
    el.has_attr("start") && el.has_attr("end") && (el.name() == "line" || el.name() == "polyline")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Right,
    Down,
    Left,
}

#[derive(Clone, Copy, Debug)]
pub struct Endpoint {
    pub origin: (f32, f32),
    pub dir: Option<Direction>,
}

impl Endpoint {
    const fn new(origin: (f32, f32), dir: Option<Direction>) -> Self {
        Self { origin, dir }
    }
}

/// Returns the midpoint of two 1D ranges if they overlap, None otherwise.
/// For point-vs-range (where min1==max1), this returns Some(point) if the point is within the range.
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
/// Returns (start_loc, end_loc) based on relative positions.
fn select_corners(start_bb: &BoundingBox, end_bb: &BoundingBox) -> (LocSpec, LocSpec) {
    let start_cx = (start_bb.x1 + start_bb.x2) / 2.0;
    let start_cy = (start_bb.y1 + start_bb.y2) / 2.0;
    let end_cx = (end_bb.x1 + end_bb.x2) / 2.0;
    let end_cy = (end_bb.y1 + end_bb.y2) / 2.0;

    let end_is_right = end_cx > start_cx;
    let end_is_below = end_cy > start_cy;

    match (end_is_right, end_is_below) {
        (true, true) => (LocSpec::BottomRight, LocSpec::TopLeft),
        (true, false) => (LocSpec::TopRight, LocSpec::BottomLeft),
        (false, true) => (LocSpec::BottomLeft, LocSpec::TopRight),
        (false, false) => (LocSpec::TopLeft, LocSpec::BottomRight),
    }
}

/// Select appropriate corner LocSpec for connecting a point to a bbox with no overlap.
fn select_corner_for_point(point: (f32, f32), bb: &BoundingBox) -> LocSpec {
    let bb_cx = (bb.x1 + bb.x2) / 2.0;
    let bb_cy = (bb.y1 + bb.y2) / 2.0;

    let point_is_right = point.0 > bb_cx;
    let point_is_below = point.1 > bb_cy;

    // Return the corner of the bbox closest to the point
    match (point_is_right, point_is_below) {
        (true, true) => LocSpec::BottomRight,
        (true, false) => LocSpec::TopRight,
        (false, true) => LocSpec::BottomLeft,
        (false, false) => LocSpec::TopLeft,
    }
}

/// Represents the result of analyzing overlap between two bboxes or a point and bbox.
#[derive(Debug)]
enum ConnectionStrategy {
    /// Both dimensions overlap - bboxes intersect, no line should be drawn
    Overlap,
    /// X-ranges overlap - draw vertical line at x_midpoint, connect top/bottom edges
    Vertical { x_midpoint: f32 },
    /// Y-ranges overlap - draw horizontal line at y_midpoint, connect left/right edges
    Horizontal { y_midpoint: f32 },
    /// No overlap in either dimension - connect via corners
    Corner {
        start_loc: LocSpec,
        end_loc: LocSpec,
    },
}

impl ConnectionStrategy {
    fn new(start_bb: &BoundingBox, end_bb: &BoundingBox) -> Self {
        let x_overlap = range_overlap(start_bb.x1, start_bb.x2, end_bb.x1, end_bb.x2);
        let y_overlap = range_overlap(start_bb.y1, start_bb.y2, end_bb.y1, end_bb.y2);

        match (x_overlap, y_overlap) {
            (Some(_), Some(_)) => Self::Overlap,
            (Some(x_mid), None) => Self::Vertical { x_midpoint: x_mid },
            (None, Some(y_mid)) => Self::Horizontal { y_midpoint: y_mid },
            (None, None) => {
                let (start_loc, end_loc) = select_corners(start_bb, end_bb);
                Self::Corner { start_loc, end_loc }
            }
        }
    }
}

fn determine_point_to_bbox_strategy(point: (f32, f32), bb: &BoundingBox) -> ConnectionStrategy {
    // Treat the point as a zero-size bbox for overlap calculation
    let point_bb = BoundingBox::new(point.0, point.1, point.0, point.1);
    let x_overlap = range_overlap(point_bb.x1, point_bb.x2, bb.x1, bb.x2);
    let y_overlap = range_overlap(point_bb.y1, point_bb.y2, bb.y1, bb.y2);

    match (x_overlap, y_overlap) {
        (Some(_), Some(_)) => ConnectionStrategy::Overlap,
        (Some(_), None) => {
            // Point is within x-range of bbox - vertical line
            // Use the point's x coordinate (since it's in the overlap)
            ConnectionStrategy::Vertical {
                x_midpoint: point.0,
            }
        }
        (None, Some(_)) => {
            // Point is within y-range of bbox - horizontal line
            ConnectionStrategy::Horizontal {
                y_midpoint: point.1,
            }
        }
        (None, None) => {
            // No overlap - connect to nearest corner
            let end_loc = select_corner_for_point(point, bb);
            ConnectionStrategy::Corner {
                start_loc: LocSpec::Center, // Will use point directly
                end_loc,
            }
        }
    }
}

/// For polyline (elbow) connectors, find the shortest link using cardinal directions only.
/// Returns (start_loc, end_loc) from Top/Right/Bottom/Left.
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

/// For polyline connectors, find the closest cardinal direction from a point to a bbox.
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

#[derive(Clone)]
pub struct Connector {
    source_element: SvgElement,
    start_el: Option<SvgElement>,
    end_el: Option<SvgElement>,
    pub start: Endpoint,
    pub end: Endpoint,
    overlap: bool,
    offset: Option<Length>,
}

#[allow(clippy::large_enum_variant)]
enum ElementParseData {
    /// Element reference without explicit locspec - needs overlap-based resolution
    El(SvgElement),
    /// Element reference with explicit locspec - resolve to fixed point immediately
    ElWithLoc(SvgElement, ElementLoc, Option<Direction>),
    /// Fixed coordinate point
    Point(f32, f32),
}

impl Connector {
    fn loc_to_dir(loc: LocSpec) -> Option<Direction> {
        match loc {
            LocSpec::Top | LocSpec::TopEdge(_) => Some(Direction::Up),
            LocSpec::Right | LocSpec::RightEdge(_) => Some(Direction::Right),
            LocSpec::Bottom | LocSpec::BottomEdge(_) => Some(Direction::Down),
            LocSpec::Left | LocSpec::LeftEdge(_) => Some(Direction::Left),
            _ => None,
        }
    }

    fn parse_element(
        element: &mut SvgElement,
        elem_map: &impl ElementMap,
        attr_name: &str,
    ) -> Result<ElementParseData> {
        let this_ref = element
            .pop_attr(attr_name)
            .ok_or_else(|| Error::MissingAttr(attr_name.to_string()))?;

        // Example: "#thing@tl" => top left coordinate of element id="thing"
        if let Ok((elref, loc)) = parse_el_loc(&this_ref) {
            let el = elem_map
                .get_element(&elref)
                .ok_or_else(|| Error::Reference(elref))?
                .clone();

            if let Some(loc) = loc {
                let dir = if let ElementLoc::LocSpec(ls) = loc {
                    Self::loc_to_dir(ls)
                } else {
                    None
                };
                Ok(ElementParseData::ElWithLoc(el, loc, dir))
            } else {
                Ok(ElementParseData::El(el))
            }
        } else {
            let mut parts = attr_split_cycle(&this_ref);
            let x = parts.next().ok_or_else(|| {
                Error::InvalidValue(format!("{attr_name}.x"), this_ref.to_owned())
            })?;
            let y = parts.next().ok_or_else(|| {
                Error::InvalidValue(format!("{attr_name}.y"), this_ref.to_owned())
            })?;
            Ok(ElementParseData::Point(strp(&x)?, strp(&y)?))
        }
    }

    pub fn from_element(element: &SvgElement, elem_map: &impl ElementMap) -> Result<Self> {
        let mut element = element.clone();

        // Ignore edge-type attribute (deprecated)
        let _ = element.pop_attr("edge-type");

        let start_ret = Self::parse_element(&mut element, elem_map, "start")?;
        let end_ret = Self::parse_element(&mut element, elem_map, "end")?;

        let offset = if let Some(o_inner) = element.pop_attr("corner-offset") {
            Some(
                strp_length(&o_inner)
                    .map_err(|_| Error::Parse("invalid corner-offset".to_owned()))?,
            )
        } else {
            None
        };

        let is_polyline = element.name() == "polyline";

        // Resolve endpoints based on what was parsed.
        // If a locspec was provided, convert to fixed point immediately.
        // If just element reference, use overlap-based resolution for lines,
        // or cardinal-direction shortest-link for polylines.
        let (start, end, start_el, end_el, overlapping) = match (&start_ret, &end_ret) {
            // Both are fixed points
            (ElementParseData::Point(x1, y1), ElementParseData::Point(x2, y2)) => (
                Endpoint::new((*x1, *y1), None),
                Endpoint::new((*x2, *y2), None),
                None,
                None,
                false,
            ),

            // Both have explicit locspecs - convert to fixed points
            (
                ElementParseData::ElWithLoc(start_el, start_loc, start_dir),
                ElementParseData::ElWithLoc(end_el, end_loc, end_dir),
            ) => {
                let start_coord = start_el.get_element_loc_coord(elem_map, *start_loc)?;
                let end_coord = end_el.get_element_loc_coord(elem_map, *end_loc)?;
                (
                    Endpoint::new(start_coord, *start_dir),
                    Endpoint::new(end_coord, *end_dir),
                    Some(start_el.clone()),
                    Some(end_el.clone()),
                    false,
                )
            }

            // Start has locspec, end is point
            (
                ElementParseData::ElWithLoc(start_el, start_loc, start_dir),
                ElementParseData::Point(x2, y2),
            ) => {
                let start_coord = start_el.get_element_loc_coord(elem_map, *start_loc)?;
                (
                    Endpoint::new(start_coord, *start_dir),
                    Endpoint::new((*x2, *y2), None),
                    Some(start_el.clone()),
                    None,
                    false,
                )
            }

            // End has locspec, start is point
            (
                ElementParseData::Point(x1, y1),
                ElementParseData::ElWithLoc(end_el, end_loc, end_dir),
            ) => {
                let end_coord = end_el.get_element_loc_coord(elem_map, *end_loc)?;
                (
                    Endpoint::new((*x1, *y1), None),
                    Endpoint::new(end_coord, *end_dir),
                    None,
                    Some(end_el.clone()),
                    false,
                )
            }

            // Start has locspec, end is bare element
            (
                ElementParseData::ElWithLoc(start_el, start_loc, start_dir),
                ElementParseData::El(end_el),
            ) => {
                let start_coord = start_el.get_element_loc_coord(elem_map, *start_loc)?;
                let end_bb = elem_map
                    .get_element_bbox(end_el)?
                    .ok_or_else(|| Error::MissingBBox(end_el.to_string()))?;

                let (end_coord, end_dir, overlapping) = if is_polyline {
                    // For polylines, use cardinal direction closest to start point
                    let end_loc = closest_cardinal_loc(start_coord, &end_bb);
                    let end_pt = end_bb.locspec(end_loc);
                    (end_pt, Self::loc_to_dir(end_loc), false)
                } else {
                    // For lines, use overlap-based strategy
                    match determine_point_to_bbox_strategy(start_coord, &end_bb) {
                        ConnectionStrategy::Overlap => {
                            // Point is inside bbox
                            (start_coord, None, true)
                        }
                        ConnectionStrategy::Vertical { x_midpoint } => {
                            // Vertical line - connect to top or bottom edge
                            let end_loc = if start_coord.1 < end_bb.y1 {
                                LocSpec::Top
                            } else {
                                LocSpec::Bottom
                            };
                            let end_pt = (x_midpoint, end_bb.locspec(end_loc).1);
                            (end_pt, Self::loc_to_dir(end_loc), false)
                        }
                        ConnectionStrategy::Horizontal { y_midpoint } => {
                            // Horizontal line - connect to left or right edge
                            let end_loc = if start_coord.0 < end_bb.x1 {
                                LocSpec::Left
                            } else {
                                LocSpec::Right
                            };
                            let end_pt = (end_bb.locspec(end_loc).0, y_midpoint);
                            (end_pt, Self::loc_to_dir(end_loc), false)
                        }
                        ConnectionStrategy::Corner { end_loc, .. } => {
                            let end_pt = end_bb.locspec(end_loc);
                            (end_pt, Self::loc_to_dir(end_loc), false)
                        }
                    }
                };
                (
                    Endpoint::new(start_coord, *start_dir),
                    Endpoint::new(end_coord, end_dir),
                    Some(start_el.clone()),
                    Some(end_el.clone()),
                    overlapping,
                )
            }

            // End has locspec, start is bare element
            (
                ElementParseData::El(start_el),
                ElementParseData::ElWithLoc(end_el, end_loc, end_dir),
            ) => {
                let end_coord = end_el.get_element_loc_coord(elem_map, *end_loc)?;
                let start_bb = elem_map
                    .get_element_bbox(start_el)?
                    .ok_or_else(|| Error::MissingBBox(start_el.to_string()))?;

                let (start_coord, start_dir, overlapping) = if is_polyline {
                    // For polylines, use cardinal direction closest to end point
                    let start_loc = closest_cardinal_loc(end_coord, &start_bb);
                    let start_pt = start_bb.locspec(start_loc);
                    (start_pt, Self::loc_to_dir(start_loc), false)
                } else {
                    // For lines, use overlap-based strategy
                    match determine_point_to_bbox_strategy(end_coord, &start_bb) {
                        ConnectionStrategy::Overlap => {
                            // Point is inside bbox
                            (end_coord, None, true)
                        }
                        ConnectionStrategy::Vertical { x_midpoint } => {
                            let start_loc = if end_coord.1 < start_bb.y1 {
                                LocSpec::Top
                            } else {
                                LocSpec::Bottom
                            };
                            let start_pt = (x_midpoint, start_bb.locspec(start_loc).1);
                            (start_pt, Self::loc_to_dir(start_loc), false)
                        }
                        ConnectionStrategy::Horizontal { y_midpoint } => {
                            let start_loc = if end_coord.0 < start_bb.x1 {
                                LocSpec::Left
                            } else {
                                LocSpec::Right
                            };
                            let start_pt = (start_bb.locspec(start_loc).0, y_midpoint);
                            (start_pt, Self::loc_to_dir(start_loc), false)
                        }
                        ConnectionStrategy::Corner {
                            end_loc: start_loc, ..
                        } => {
                            let start_pt = start_bb.locspec(start_loc);
                            (start_pt, Self::loc_to_dir(start_loc), false)
                        }
                    }
                };
                (
                    Endpoint::new(start_coord, start_dir),
                    Endpoint::new(end_coord, *end_dir),
                    Some(start_el.clone()),
                    Some(end_el.clone()),
                    overlapping,
                )
            }

            // Start is point, end is bare element
            (ElementParseData::Point(x1, y1), ElementParseData::El(end_el)) => {
                let start_point = (*x1, *y1);
                let end_bb = elem_map
                    .get_element_bbox(end_el)?
                    .ok_or_else(|| Error::MissingBBox(end_el.to_string()))?;

                let (end_coord, end_dir, overlapping) = if is_polyline {
                    // For polylines, use cardinal direction closest to start point
                    let end_loc = closest_cardinal_loc(start_point, &end_bb);
                    let end_pt = end_bb.locspec(end_loc);
                    (end_pt, Self::loc_to_dir(end_loc), false)
                } else {
                    // For lines, use overlap-based strategy
                    match determine_point_to_bbox_strategy(start_point, &end_bb) {
                        ConnectionStrategy::Overlap => (start_point, None, true),
                        ConnectionStrategy::Vertical { x_midpoint } => {
                            let end_loc = if start_point.1 < end_bb.y1 {
                                LocSpec::Top
                            } else {
                                LocSpec::Bottom
                            };
                            let end_pt = (x_midpoint, end_bb.locspec(end_loc).1);
                            (end_pt, Self::loc_to_dir(end_loc), false)
                        }
                        ConnectionStrategy::Horizontal { y_midpoint } => {
                            let end_loc = if start_point.0 < end_bb.x1 {
                                LocSpec::Left
                            } else {
                                LocSpec::Right
                            };
                            let end_pt = (end_bb.locspec(end_loc).0, y_midpoint);
                            (end_pt, Self::loc_to_dir(end_loc), false)
                        }
                        ConnectionStrategy::Corner { end_loc, .. } => {
                            let end_pt = end_bb.locspec(end_loc);
                            (end_pt, Self::loc_to_dir(end_loc), false)
                        }
                    }
                };
                (
                    Endpoint::new(start_point, None),
                    Endpoint::new(end_coord, end_dir),
                    None,
                    Some(end_el.clone()),
                    overlapping,
                )
            }

            // End is point, start is bare element
            (ElementParseData::El(start_el), ElementParseData::Point(x2, y2)) => {
                let end_point = (*x2, *y2);
                let start_bb = elem_map
                    .get_element_bbox(start_el)?
                    .ok_or_else(|| Error::MissingBBox(start_el.to_string()))?;

                let (start_coord, start_dir, overlapping) = if is_polyline {
                    // For polylines, use cardinal direction closest to end point
                    let start_loc = closest_cardinal_loc(end_point, &start_bb);
                    let start_pt = start_bb.locspec(start_loc);
                    (start_pt, Self::loc_to_dir(start_loc), false)
                } else {
                    // For lines, use overlap-based strategy
                    match determine_point_to_bbox_strategy(end_point, &start_bb) {
                        ConnectionStrategy::Overlap => (end_point, None, true),
                        ConnectionStrategy::Vertical { x_midpoint } => {
                            let start_loc = if end_point.1 < start_bb.y1 {
                                LocSpec::Top
                            } else {
                                LocSpec::Bottom
                            };
                            let start_pt = (x_midpoint, start_bb.locspec(start_loc).1);
                            (start_pt, Self::loc_to_dir(start_loc), false)
                        }
                        ConnectionStrategy::Horizontal { y_midpoint } => {
                            let start_loc = if end_point.0 < start_bb.x1 {
                                LocSpec::Left
                            } else {
                                LocSpec::Right
                            };
                            let start_pt = (start_bb.locspec(start_loc).0, y_midpoint);
                            (start_pt, Self::loc_to_dir(start_loc), false)
                        }
                        ConnectionStrategy::Corner {
                            end_loc: start_loc, ..
                        } => {
                            let start_pt = start_bb.locspec(start_loc);
                            (start_pt, Self::loc_to_dir(start_loc), false)
                        }
                    }
                };
                (
                    Endpoint::new(start_coord, start_dir),
                    Endpoint::new(end_point, None),
                    Some(start_el.clone()),
                    None,
                    overlapping,
                )
            }

            // Both are bare elements
            (ElementParseData::El(start_el), ElementParseData::El(end_el)) => {
                let start_bb = elem_map
                    .get_element_bbox(start_el)?
                    .ok_or_else(|| Error::MissingBBox(start_el.to_string()))?;
                let end_bb = elem_map
                    .get_element_bbox(end_el)?
                    .ok_or_else(|| Error::MissingBBox(end_el.to_string()))?;

                let (start_coord, end_coord, start_dir, end_dir, overlapping) = if is_polyline {
                    // For polylines, use cardinal-direction shortest link
                    let (start_loc, end_loc) = shortest_cardinal_link(&start_bb, &end_bb);
                    let start_pt = start_bb.locspec(start_loc);
                    let end_pt = end_bb.locspec(end_loc);
                    (
                        start_pt,
                        end_pt,
                        Self::loc_to_dir(start_loc),
                        Self::loc_to_dir(end_loc),
                        false,
                    )
                } else {
                    // For lines, use overlap-based resolution
                    use ConnectionStrategy::*;
                    match ConnectionStrategy::new(&start_bb, &end_bb) {
                        Overlap => {
                            // Bboxes intersect - no line to draw
                            let start_c = start_bb.locspec(LocSpec::Center);
                            let end_c = end_bb.locspec(LocSpec::Center);
                            (start_c, end_c, None, None, true)
                        }
                        Vertical { x_midpoint } => {
                            // X-ranges overlap - vertical line at midpoint
                            let (start_loc, end_loc) = if start_bb.y2 < end_bb.y1 {
                                (LocSpec::Bottom, LocSpec::Top)
                            } else {
                                (LocSpec::Top, LocSpec::Bottom)
                            };
                            let start_pt = (x_midpoint, start_bb.locspec(start_loc).1);
                            let end_pt = (x_midpoint, end_bb.locspec(end_loc).1);
                            (
                                start_pt,
                                end_pt,
                                Self::loc_to_dir(start_loc),
                                Self::loc_to_dir(end_loc),
                                false,
                            )
                        }
                        Horizontal { y_midpoint } => {
                            // Y-ranges overlap - horizontal line at midpoint
                            let (start_loc, end_loc) = if start_bb.x2 < end_bb.x1 {
                                (LocSpec::Right, LocSpec::Left)
                            } else {
                                (LocSpec::Left, LocSpec::Right)
                            };
                            let start_pt = (start_bb.locspec(start_loc).0, y_midpoint);
                            let end_pt = (end_bb.locspec(end_loc).0, y_midpoint);
                            (
                                start_pt,
                                end_pt,
                                Self::loc_to_dir(start_loc),
                                Self::loc_to_dir(end_loc),
                                false,
                            )
                        }
                        Corner { start_loc, end_loc } => {
                            // No overlap - corner to corner
                            let start_pt = start_bb.locspec(start_loc);
                            let end_pt = end_bb.locspec(end_loc);
                            (
                                start_pt,
                                end_pt,
                                Self::loc_to_dir(start_loc),
                                Self::loc_to_dir(end_loc),
                                false,
                            )
                        }
                    }
                };
                (
                    Endpoint::new(start_coord, start_dir),
                    Endpoint::new(end_coord, end_dir),
                    Some(start_el.clone()),
                    Some(end_el.clone()),
                    overlapping,
                )
            }
        };

        Ok(Self {
            source_element: element,
            start,
            end,
            start_el,
            end_el,
            overlap: overlapping,
            offset,
        })
    }

    pub fn render(&self, ctx: &impl ElementMap) -> Result<Option<SvgElement>> {
        // If bboxes fully overlap, don't render any line
        if self.overlap {
            return Ok(None);
        }

        let (x1, y1) = self.start.origin;
        let (x2, y2) = self.end.origin;

        // Branch on element name: polyline uses corner routing, line uses straight
        if self.source_element.name() == "polyline" {
            self.render_corner(ctx, x1, y1, x2, y2).map(Some)
        } else {
            self.render_straight(x1, y1, x2, y2).map(Some)
        }
    }

    fn render_straight(&self, x1: f32, y1: f32, x2: f32, y2: f32) -> Result<SvgElement> {
        Ok(SvgElement::new(
            "line",
            &[
                ("x1".to_string(), fstr(x1)),
                ("y1".to_string(), fstr(y1)),
                ("x2".to_string(), fstr(x2)),
                ("y2".to_string(), fstr(y2)),
            ],
        )
        .with_attrs_from(&self.source_element))
    }

    fn render_corner(
        &self,
        ctx: &impl ElementMap,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
    ) -> Result<SvgElement> {
        // For some (e.g. u-shaped) connections we need a default *absolute* offset
        // as ratio (e.g. the overall '50%' default) don't make sense.
        let default_ratio_offset = Length::Ratio(0.5);
        let default_abs_offset = Length::Absolute(3.);

        let mut abs_offset_set = false;
        let mut start_abs_offset = default_abs_offset
            .absolute()
            .expect("set to absolute 3 above");
        let mut end_abs_offset = start_abs_offset;
        let mut ratio_offset = default_ratio_offset
            .ratio()
            .expect("set to ratio 0.5 above");
        if let Some(offset) = &self.offset {
            if let Some(o) = offset.absolute() {
                start_abs_offset = o;
                end_abs_offset = o;
                abs_offset_set = true;
            }
            if let Some(r) = offset.ratio() {
                ratio_offset = r;
                start_abs_offset = 0.0;
                end_abs_offset = 0.0;
            }
        }

        let mut start_el_bb = BoundingBox::new(x1, y1, x1, y1);
        let mut end_el_bb = BoundingBox::new(x2, y2, x2, y2);
        if let Some(el) = &self.start_el {
            if let Ok(Some(el_bb)) = ctx.get_element_bbox(el) {
                start_el_bb = el_bb;
            }
        }
        if let Some(el) = &self.end_el {
            if let Ok(Some(el_bb)) = ctx.get_element_bbox(el) {
                end_el_bb = el_bb;
            }
        }
        let points = render_match_corner(
            self,
            ratio_offset,
            start_abs_offset,
            end_abs_offset,
            start_el_bb,
            end_el_bb,
            abs_offset_set,
        )?;

        // remove repeated points
        let points = filter_points(points);
        Ok(SvgElement::new(
            "polyline",
            &[(
                "points".to_string(),
                points
                    .into_iter()
                    .map(|(px, py)| format!("{} {}", fstr(px), fstr(py)))
                    .collect::<Vec<String>>()
                    .join(", "),
            )],
        )
        .with_attrs_from(&self.source_element))
    }
}

/// Remove identical and collinear point pairs
fn filter_points(p: Vec<(f32, f32)>) -> Vec<(f32, f32)> {
    let mut ret = Vec::with_capacity(p.len());
    const EPSILON: f32 = 1e-6;

    for current in p {
        // Skip exact duplicates
        if ret.last().copied() == Some(current) {
            continue;
        }

        // Remove middle point if three consecutive points are collinear and monotonic
        if ret.len() >= 2 {
            let prev2 = ret[ret.len() - 2];
            let prev1 = ret[ret.len() - 1];

            // Vector from prev2 to prev1
            let v1 = (prev1.0 - prev2.0, prev1.1 - prev2.1);
            // Vector from prev1 to current
            let v2 = (current.0 - prev1.0, current.1 - prev1.1);

            // Check if vectors are parallel (cross product near zero)
            let cross = v1.0 * v2.1 - v1.1 * v2.0;
            let collinear = cross.abs() < EPSILON;

            if collinear {
                // Check monotonicity: dot product should be non-negative
                // (vectors point in same or opposite direction)
                let dot = v1.0 * v2.0 + v1.1 * v2.1;
                let monotonic = dot >= -EPSILON;

                if monotonic {
                    ret.pop();
                }
            }
        }
        ret.push(current);
    }
    ret
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_points() {
        for (case, point_set, expected) in [
            (
                "monotonic horizontal line",
                vec![(0., 0.), (1., 0.), (2., 0.), (3., 0.)],
                vec![(0., 0.), (3., 0.)],
            ),
            (
                "monotonic vertical line",
                vec![(0., 0.), (0., 1.), (0., 2.), (0., 3.)],
                vec![(0., 0.), (0., 3.)],
            ),
            (
                "non-monotonic horizontal line",
                vec![(0., 0.), (1., 0.), (2., 0.), (1., 0.), (5., 0.)],
                vec![(0., 0.), (2., 0.), (1., 0.), (5., 0.)],
            ),
            (
                "non-monotonic vertical line",
                vec![(0., 0.), (0., 1.), (0., 2.), (0., 1.), (0., 5.)],
                vec![(0., 0.), (0., 2.), (0., 1.), (0., 5.)],
            ),
            (
                "right angle turn",
                vec![(0., 0.), (1., 0.), (1., 1.), (1., 2.), (2., 2.)],
                vec![(0., 0.), (1., 0.), (1., 2.), (2., 2.)],
            ),
            (
                "diagonal line",
                vec![(0., 0.), (1., 1.), (1., 1.), (2., 2.)],
                vec![(0., 0.), (2., 2.)],
            ),
            (
                "gradient 3 diagonal",
                vec![(0., 0.), (1., 3.), (2., 6.), (3., 9.)],
                vec![(0., 0.), (3., 9.)],
            ),
            (
                "non-integer slope",
                vec![(0., 0.25), (1., 0.5), (2., 0.75), (3., 1.0)],
                vec![(0., 0.25), (3., 1.0)],
            ),
        ] {
            let filtered = filter_points(point_set);
            assert_eq!(filtered, expected, "{case}");
        }
    }
}
