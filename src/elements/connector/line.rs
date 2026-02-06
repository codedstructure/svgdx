use super::{loc_to_dir, Direction, Endpoint};
use crate::context::ElementMap;
use crate::elements::SvgElement;
use crate::errors::{Error, Result};
use crate::geometry::{parse_el_loc, BoundingBox, ElementLoc, LocSpec};
use crate::types::{attr_split_cycle, fstr, strp};

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

    match (point_is_right, point_is_below) {
        (true, true) => LocSpec::BottomRight,
        (true, false) => LocSpec::TopRight,
        (false, true) => LocSpec::BottomLeft,
        (false, false) => LocSpec::TopLeft,
    }
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
            x_axis: AxisOverlap::from_ranges(bb1.x1, bb1.x2, bb2.x1, bb2.x2),
            y_axis: AxisOverlap::from_ranges(bb1.y1, bb1.y2, bb2.y1, bb2.y2),
        }
    }

    fn from_point_and_bbox(point: (f32, f32), bb: &BoundingBox) -> Self {
        let point_bb = BoundingBox::new(point.0, point.1, point.0, point.1);
        Self::new(&point_bb, bb)
    }

    fn fully_overlapping(&self) -> bool {
        self.x_axis.is_overlapping() && self.y_axis.is_overlapping()
    }

    /// Returns connection points for two bboxes based on overlap analysis.
    fn resolve_bbox_to_bbox(
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

    /// Returns connection point on bbox for a fixed point.
    fn resolve_point_to_bbox(&self, point: (f32, f32), bb: &BoundingBox) -> (LocSpec, (f32, f32)) {
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

#[derive(Clone)]
pub struct LineConnector {
    source_element: SvgElement,
    start: Endpoint,
    end: Endpoint,
    overlap: bool,
}

#[allow(clippy::large_enum_variant)]
pub(crate) enum ElementParseData {
    El(SvgElement),
    ElWithLoc(SvgElement, ElementLoc, Option<Direction>),
    Point(f32, f32),
}

/// Parsed endpoint: (fixed_point_with_dir, element_with_bbox)
pub(crate) type ParsedEndpoint = (
    Option<(f32, f32, Option<Direction>)>,
    Option<(SvgElement, BoundingBox)>,
);

/// Result type for resolving two bboxes.
struct BBoxResolution {
    start_coord: (f32, f32),
    end_coord: (f32, f32),
    start_dir: Option<Direction>,
    end_dir: Option<Direction>,
    overlapping: bool,
}

impl LineConnector {
    pub(crate) fn parse_element(
        element: &mut SvgElement,
        elem_map: &impl ElementMap,
        attr_name: &str,
    ) -> Result<ElementParseData> {
        let this_ref = element
            .pop_attr(attr_name)
            .ok_or_else(|| Error::MissingAttr(attr_name.to_string()))?;

        if let Ok((elref, loc)) = parse_el_loc(&this_ref) {
            let el = elem_map
                .get_element(&elref)
                .ok_or_else(|| Error::Reference(elref))?
                .clone();

            if let Some(loc) = loc {
                let dir = if let ElementLoc::LocSpec(ls) = loc {
                    loc_to_dir(ls)
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

    /// Convert ElementParseData to either a fixed point or get bbox
    pub(crate) fn to_fixed_or_bbox(
        data: &ElementParseData,
        elem_map: &impl ElementMap,
    ) -> Result<ParsedEndpoint> {
        match data {
            ElementParseData::Point(x, y) => Ok((Some((*x, *y, None)), None)),
            ElementParseData::ElWithLoc(el, loc, dir) => {
                let coord = el.get_element_loc_coord(elem_map, *loc)?;
                Ok((
                    Some((coord.0, coord.1, *dir)),
                    Some((
                        el.clone(),
                        BoundingBox::new(coord.0, coord.1, coord.0, coord.1),
                    )),
                ))
            }
            ElementParseData::El(el) => {
                let bb = elem_map
                    .get_element_bbox(el)?
                    .ok_or_else(|| Error::MissingBBox(el.to_string()))?;
                Ok((None, Some((el.clone(), bb))))
            }
        }
    }

    /// Resolve a fixed point against a bbox using overlap-based strategy.
    fn resolve_point_to_bbox(
        point: (f32, f32),
        bb: &BoundingBox,
    ) -> ((f32, f32), Option<Direction>, bool) {
        let rel = BBoxRelation::from_point_and_bbox(point, bb);
        if rel.fully_overlapping() {
            (point, None, true)
        } else {
            let (loc, coord) = rel.resolve_point_to_bbox(point, bb);
            (coord, loc_to_dir(loc), false)
        }
    }

    /// Resolve two bboxes using overlap-based strategy.
    fn resolve_bbox_to_bbox(start_bb: &BoundingBox, end_bb: &BoundingBox) -> BBoxResolution {
        let rel = BBoxRelation::new(start_bb, end_bb);
        if rel.fully_overlapping() {
            BBoxResolution {
                start_coord: start_bb.locspec(LocSpec::Center),
                end_coord: end_bb.locspec(LocSpec::Center),
                start_dir: None,
                end_dir: None,
                overlapping: true,
            }
        } else {
            let (start_loc, start_coord, end_loc, end_coord) =
                rel.resolve_bbox_to_bbox(start_bb, end_bb);
            BBoxResolution {
                start_coord,
                end_coord,
                start_dir: loc_to_dir(start_loc),
                end_dir: loc_to_dir(end_loc),
                overlapping: false,
            }
        }
    }

    pub fn from_element(element: &SvgElement, elem_map: &impl ElementMap) -> Result<Self> {
        let mut element = element.clone();

        // Ignore deprecated attributes
        let _ = element.pop_attr("edge-type");

        let start_ret = Self::parse_element(&mut element, elem_map, "start")?;
        let end_ret = Self::parse_element(&mut element, elem_map, "end")?;

        let (start_fixed, start_data) = Self::to_fixed_or_bbox(&start_ret, elem_map)?;
        let (end_fixed, end_data) = Self::to_fixed_or_bbox(&end_ret, elem_map)?;

        let (start, end, overlap) = match (start_fixed, end_fixed) {
            // Both resolved to fixed points
            (Some((x1, y1, dir1)), Some((x2, y2, dir2))) => (
                Endpoint::new((x1, y1), dir1),
                Endpoint::new((x2, y2), dir2),
                false,
            ),

            // Start is fixed, end needs resolution
            (Some((x1, y1, dir1)), None) => {
                let (_, end_bb) = end_data.expect("end must have element if not fixed");
                let (end_coord, end_dir, overlap) = Self::resolve_point_to_bbox((x1, y1), &end_bb);
                (
                    Endpoint::new((x1, y1), dir1),
                    Endpoint::new(end_coord, end_dir),
                    overlap,
                )
            }

            // End is fixed, start needs resolution
            (None, Some((x2, y2, dir2))) => {
                let (_, start_bb) = start_data.expect("start must have element if not fixed");
                let (start_coord, start_dir, overlap) =
                    Self::resolve_point_to_bbox((x2, y2), &start_bb);
                (
                    Endpoint::new(start_coord, start_dir),
                    Endpoint::new((x2, y2), dir2),
                    overlap,
                )
            }

            // Both need resolution
            (None, None) => {
                let (_, start_bb) = start_data.expect("start must have element");
                let (_, end_bb) = end_data.expect("end must have element");
                let res = Self::resolve_bbox_to_bbox(&start_bb, &end_bb);
                (
                    Endpoint::new(res.start_coord, res.start_dir),
                    Endpoint::new(res.end_coord, res.end_dir),
                    res.overlapping,
                )
            }
        };

        Ok(Self {
            source_element: element,
            start,
            end,
            overlap,
        })
    }

    pub fn render(&self, _ctx: &impl ElementMap) -> Result<Option<SvgElement>> {
        if self.overlap {
            return Ok(None);
        }

        let (x1, y1) = self.start.origin;
        let (x2, y2) = self.end.origin;

        Ok(Some(
            SvgElement::new(
                "line",
                &[
                    ("x1".to_string(), fstr(x1)),
                    ("y1".to_string(), fstr(y1)),
                    ("x2".to_string(), fstr(x2)),
                    ("y2".to_string(), fstr(y2)),
                ],
            )
            .with_attrs_from(&self.source_element),
        ))
    }
}
