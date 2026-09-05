use super::corner_route::render_match_corner;
use super::gap::{GapSpec, points_with_gap};
use super::line::{ElementParseData, LineConnector, ParsedEndpoint};
use super::{Direction, Endpoint, loc_to_dir};
use crate::context::ElementMap;
use crate::elements::SvgElement;
use crate::errors::{Error, Result};
use crate::geometry::{BoundingBox, Length, LocSpec, strp_length};
use crate::types::fstr;

/// For polyline (elbow) connectors, find the shortest link using cardinal directions only.
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

/// Result type for resolving two bboxes.
struct BBoxResolution {
    start_coord: (f32, f32),
    end_coord: (f32, f32),
    start_dir: Option<Direction>,
    end_dir: Option<Direction>,
}

const BBOX_POINT_EPSILON: f32 = 1e-6;

/// Is this bbox basically a point?
fn is_degenerate_bbox(bb: &BoundingBox) -> bool {
    bb.width().abs() <= BBOX_POINT_EPSILON && bb.height().abs() <= BBOX_POINT_EPSILON
}

/// Major cardinal direction [from -> to] if points are not coincident
fn dir_towards(from: (f32, f32), to: (f32, f32)) -> Option<Direction> {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;

    if dx.abs() <= BBOX_POINT_EPSILON && dy.abs() <= BBOX_POINT_EPSILON {
        return None;
    }

    if dx.abs() >= dy.abs() {
        if dx >= 0.0 {
            Some(Direction::Right)
        } else {
            Some(Direction::Left)
        }
    } else if dy >= 0.0 {
        Some(Direction::Down)
    } else {
        Some(Direction::Up)
    }
}

#[derive(Clone)]
pub struct ElbowConnector {
    source_element: SvgElement,
    pub(super) start_el: Option<SvgElement>,
    pub(super) end_el: Option<SvgElement>,
    pub start: Endpoint,
    pub end: Endpoint,
    offset: Option<Length>,
    gap: Option<GapSpec>,
}

impl ElbowConnector {
    fn normalize_endpoint(
        parsed: &ElementParseData,
        fixed: Option<(f32, f32, Option<Direction>)>,
        data: Option<(SvgElement, BoundingBox)>,
    ) -> ParsedEndpoint {
        match (parsed, fixed, data) {
            // A bare element reference with a point-sized bbox should behave
            // like a literal fixed point for elbow routing.
            (ElementParseData::El(_), None, Some((_, bb))) if is_degenerate_bbox(&bb) => {
                (Some((bb.x1(), bb.y1(), None)), None)
            }
            (_, fixed, data) => (fixed, data),
        }
    }

    fn with_inferred_dir(
        origin: (f32, f32),
        dir: Option<Direction>,
        other: (f32, f32),
    ) -> Endpoint {
        Endpoint::new(origin, dir.or_else(|| dir_towards(origin, other)))
    }

    /// Resolve a fixed point against a bbox using cardinal direction.
    fn resolve_point_to_bbox(
        point: (f32, f32),
        bb: &BoundingBox,
    ) -> ((f32, f32), Option<Direction>) {
        let loc = closest_cardinal_loc(point, bb);
        (bb.locspec(loc), loc_to_dir(loc))
    }

    /// Resolve two bboxes using cardinal-direction shortest link.
    fn resolve_bbox_to_bbox(start_bb: &BoundingBox, end_bb: &BoundingBox) -> BBoxResolution {
        let (start_loc, end_loc) = shortest_cardinal_link(start_bb, end_bb);
        BBoxResolution {
            start_coord: start_bb.locspec(start_loc),
            end_coord: end_bb.locspec(end_loc),
            start_dir: loc_to_dir(start_loc),
            end_dir: loc_to_dir(end_loc),
        }
    }

    pub fn from_element(element: &SvgElement, elem_map: &impl ElementMap) -> Result<Self> {
        let mut element = element.clone();

        // Ignore deprecated attribute
        let _ = element.pop_attr("edge-type");

        let start_ret = LineConnector::parse_element(&mut element, elem_map, "start")?;
        let end_ret = LineConnector::parse_element(&mut element, elem_map, "end")?;
        let gap = element
            .pop_attr("gap")
            .map(|s| s.parse::<GapSpec>())
            .transpose()?;

        let offset = element
            .pop_attr("corner-offset")
            .map(|o| strp_length(&o).map_err(|_| Error::Parse("invalid corner-offset".to_owned())))
            .transpose()?;

        let (start_fixed, start_data): ParsedEndpoint =
            LineConnector::to_fixed_or_bbox(&start_ret, elem_map)?;
        let (end_fixed, end_data): ParsedEndpoint =
            LineConnector::to_fixed_or_bbox(&end_ret, elem_map)?;

        let (start_fixed, start_data) =
            Self::normalize_endpoint(&start_ret, start_fixed, start_data);
        let (end_fixed, end_data) = Self::normalize_endpoint(&end_ret, end_fixed, end_data);

        let (start, end, start_el, end_el) = match (start_fixed, end_fixed) {
            // Both resolved to fixed points
            (Some((x1, y1, dir1)), Some((x2, y2, dir2))) => (
                Self::with_inferred_dir((x1, y1), dir1, (x2, y2)),
                Self::with_inferred_dir((x2, y2), dir2, (x1, y1)),
                start_data.map(|(el, _)| el),
                end_data.map(|(el, _)| el),
            ),

            // Start is fixed, end needs resolution
            (Some((x1, y1, dir1)), None) => {
                let (end_el, end_bb) = end_data.expect("end must have element if not fixed");
                let (end_coord, end_dir) = Self::resolve_point_to_bbox((x1, y1), &end_bb);
                (
                    Self::with_inferred_dir((x1, y1), dir1, end_coord),
                    Endpoint::new(end_coord, end_dir),
                    start_data.map(|(el, _)| el),
                    Some(end_el),
                )
            }

            // End is fixed, start needs resolution
            (None, Some((x2, y2, dir2))) => {
                let (start_el, start_bb) =
                    start_data.expect("start must have element if not fixed");
                let (start_coord, start_dir) = Self::resolve_point_to_bbox((x2, y2), &start_bb);
                (
                    Endpoint::new(start_coord, start_dir),
                    Self::with_inferred_dir((x2, y2), dir2, start_coord),
                    Some(start_el),
                    end_data.map(|(el, _)| el),
                )
            }

            // Both need resolution
            (None, None) => {
                let (start_el, start_bb) = start_data.expect("start must have element");
                let (end_el, end_bb) = end_data.expect("end must have element");
                let res = Self::resolve_bbox_to_bbox(&start_bb, &end_bb);
                (
                    Endpoint::new(res.start_coord, res.start_dir),
                    Endpoint::new(res.end_coord, res.end_dir),
                    Some(start_el),
                    Some(end_el),
                )
            }
        };

        Ok(Self {
            source_element: element,
            start,
            end,
            start_el,
            end_el,
            offset,
            gap,
        })
    }

    fn render_corner(&self, ctx: &impl ElementMap) -> Result<SvgElement> {
        let (x1, y1) = self.start.origin;
        let (x2, y2) = self.end.origin;

        if self.start.origin == self.end.origin {
            return Ok(SvgElement::new(
                "polyline",
                &[("points".to_string(), format!("{} {}", fstr(x1), fstr(y1)))],
            )
            .with_attrs_from(&self.source_element));
        }

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
        if let Some(el) = &self.start_el
            && let Ok(Some(el_bb)) = ctx.get_element_bbox(el)
        {
            start_el_bb = el_bb;
        }
        if let Some(el) = &self.end_el
            && let Ok(Some(el_bb)) = ctx.get_element_bbox(el)
        {
            end_el_bb = el_bb;
        }

        let mut points = render_match_corner(
            self,
            ratio_offset,
            start_abs_offset,
            end_abs_offset,
            start_el_bb,
            end_el_bb,
            abs_offset_set,
        )?;

        if let Some(gap) = &self.gap {
            points = points_with_gap(&points, gap);
        }

        let points = super::corner_route::filter_points(points);
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

    pub fn render(&self, ctx: &impl ElementMap) -> Result<Option<SvgElement>> {
        self.render_corner(ctx).map(Some)
    }
}
