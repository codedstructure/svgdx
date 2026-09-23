use super::routing::{self, AutoConnectStrategy, ConnectMode};
use super::{Direction, Endpoint, GapSpec, loc_to_dir, points_with_gap};
use crate::context::ElementMap;
use crate::elements::SvgElement;
use crate::errors::{Error, Result};
use crate::geometry::{BoundingBox, ElementLoc, parse_elref_suffix};
use crate::types::{attr_split_cycle, extract_elref, fstr, parse_float};

#[derive(Clone)]
pub struct LineConnector {
    source_element: SvgElement,
    start: Endpoint,
    end: Endpoint,
    suppress_render: bool,
    gap: Option<GapSpec>,
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

const BBOX_POINT_EPSILON: f32 = 1e-6;

fn is_degenerate_bbox(bb: &BoundingBox) -> bool {
    bb.width().abs() <= BBOX_POINT_EPSILON && bb.height().abs() <= BBOX_POINT_EPSILON
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

        if let Ok((elref, remain)) = extract_elref(&this_ref) {
            let loc = parse_elref_suffix(remain)?;
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
            Ok(ElementParseData::Point(parse_float(&x)?, parse_float(&y)?))
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
        connect: ConnectMode,
        point: (f32, f32),
        bb: &BoundingBox,
    ) -> ((f32, f32), Option<Direction>, bool) {
        let strategy = connect.line_strategy();
        let resolution = routing::resolve_point_to_bbox(strategy, point, bb);
        (
            resolution.target.coord,
            resolution.target.loc.and_then(loc_to_dir),
            matches!(strategy, AutoConnectStrategy::OverlapOrCorner)
                && resolution.fully_overlapping,
        )
    }

    /// Resolve two bboxes using overlap-based strategy.
    fn resolve_bbox_to_bbox(
        connect: ConnectMode,
        start_bb: &BoundingBox,
        end_bb: &BoundingBox,
    ) -> BBoxResolution {
        let resolution = routing::resolve_bbox_to_bbox(connect.line_strategy(), start_bb, end_bb);
        BBoxResolution {
            start_coord: resolution.start.coord,
            end_coord: resolution.end.coord,
            start_dir: resolution.start.loc.and_then(loc_to_dir),
            end_dir: resolution.end.loc.and_then(loc_to_dir),
            overlapping: resolution.fully_overlapping,
        }
    }

    pub fn from_element(element: &SvgElement, elem_map: &impl ElementMap) -> Result<Self> {
        let mut element = element.clone();

        // Ignore deprecated attributes
        let _ = element.pop_attr("edge-type");

        let start_ret = Self::parse_element(&mut element, elem_map, "start")?;
        let end_ret = Self::parse_element(&mut element, elem_map, "end")?;
        let gap = element
            .pop_attr("gap")
            .map(|s| s.parse::<GapSpec>())
            .transpose()?;
        let connect = element
            .pop_attr("connect")
            .map(|s| s.parse::<ConnectMode>())
            .transpose()?
            .unwrap_or_default();

        let (start_fixed, start_data) = Self::to_fixed_or_bbox(&start_ret, elem_map)?;
        let (end_fixed, end_data) = Self::to_fixed_or_bbox(&end_ret, elem_map)?;

        let (start, end, suppress_render) = match (start_fixed, end_fixed) {
            // Both resolved to fixed points
            (Some((x1, y1, dir1)), Some((x2, y2, dir2))) => (
                Endpoint::new((x1, y1), dir1),
                Endpoint::new((x2, y2), dir2),
                false,
            ),

            // Start is fixed, end needs resolution
            (Some((x1, y1, dir1)), None) => {
                let (_, end_bb) = end_data.expect("end must have element if not fixed");
                let (end_coord, end_dir, _) =
                    Self::resolve_point_to_bbox(connect, (x1, y1), &end_bb);
                (
                    Endpoint::new((x1, y1), dir1),
                    Endpoint::new(end_coord, end_dir),
                    false,
                )
            }

            // End is fixed, start needs resolution
            (None, Some((x2, y2, dir2))) => {
                let (_, start_bb) = start_data.expect("start must have element if not fixed");
                let (start_coord, start_dir, _) =
                    Self::resolve_point_to_bbox(connect, (x2, y2), &start_bb);
                (
                    Endpoint::new(start_coord, start_dir),
                    Endpoint::new((x2, y2), dir2),
                    false,
                )
            }

            // Both need resolution
            (None, None) => {
                let (_, start_bb) = start_data.expect("start must have element");
                let (_, end_bb) = end_data.expect("end must have element");
                let res = Self::resolve_bbox_to_bbox(connect, &start_bb, &end_bb);
                let suppress_render = res.overlapping
                    && !(is_degenerate_bbox(&start_bb) && is_degenerate_bbox(&end_bb));
                (
                    Endpoint::new(res.start_coord, res.start_dir),
                    Endpoint::new(res.end_coord, res.end_dir),
                    suppress_render,
                )
            }
        };

        Ok(Self {
            source_element: element,
            start,
            end,
            suppress_render,
            gap,
        })
    }

    pub fn render(&self, _ctx: &impl ElementMap) -> Result<Option<SvgElement>> {
        // Keep zero-length connectors that are explicitly or point-resolved,
        // but suppress automatically routed links between overlapping bboxes.
        if self.suppress_render {
            return Ok(None);
        }

        // Apply any gap to ends of connector line
        let mut points = vec![self.start.origin, self.end.origin];
        if let Some(gap) = &self.gap {
            points = points_with_gap(&points, gap);
        }
        let (x1, y1) = points.first().unwrap_or(&self.start.origin);
        let (x2, y2) = points.last().unwrap_or(&self.end.origin);

        Ok(Some(
            SvgElement::new(
                "line",
                &[
                    ("x1".to_string(), fstr(*x1)),
                    ("y1".to_string(), fstr(*y1)),
                    ("x2".to_string(), fstr(*x2)),
                    ("y2".to_string(), fstr(*y2)),
                ],
            )
            .with_attrs_from(&self.source_element),
        ))
    }
}
