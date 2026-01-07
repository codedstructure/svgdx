use super::SvgElement;
use crate::context::ElementMap;
use crate::elements::corner_route::render_match_corner;
use crate::errors::{Error, Result};
use crate::geometry::{
    parse_el_loc, strp_length, BoundingBox, ElementLoc, Length, LocSpec, ScalarSpec,
};
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

#[derive(Clone, Copy, Debug)]
pub enum ConnectionType {
    Horizontal,
    Vertical,
    Corner,
    Straight,
}

impl ConnectionType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "h" | "horizontal" => Self::Horizontal,
            "v" | "vertical" => Self::Vertical,
            _ => Self::Straight,
        }
    }
}

fn edge_locations(ctype: ConnectionType) -> Vec<LocSpec> {
    match ctype {
        ConnectionType::Horizontal => vec![LocSpec::Left, LocSpec::Right],
        ConnectionType::Vertical => vec![LocSpec::Top, LocSpec::Bottom],
        ConnectionType::Corner => {
            vec![LocSpec::Top, LocSpec::Right, LocSpec::Bottom, LocSpec::Left]
        }
        ConnectionType::Straight => vec![
            LocSpec::Top,
            LocSpec::Bottom,
            LocSpec::Left,
            LocSpec::Right,
            LocSpec::TopLeft,
            LocSpec::BottomLeft,
            LocSpec::TopRight,
            LocSpec::BottomRight,
        ],
    }
}

#[derive(Clone)]
pub struct Connector {
    source_element: SvgElement,
    start_el: Option<SvgElement>,
    end_el: Option<SvgElement>,
    pub start: Endpoint,
    pub end: Endpoint,
    conn_type: ConnectionType,
    offset: Option<Length>,
}

fn closest_loc(
    this: &SvgElement,
    point: (f32, f32),
    conn_type: ConnectionType,
    context: &impl ElementMap,
) -> Result<LocSpec> {
    let mut min_dist_sq = f32::MAX;
    let mut min_loc = LocSpec::Center;

    let this_bb = context
        .get_element_bbox(this)?
        .ok_or_else(|| Error::MissingBBox(this.to_string()))?;

    for loc in edge_locations(conn_type) {
        let this_coord = this_bb.locspec(loc);
        let ((x1, y1), (x2, y2)) = (this_coord, point);
        let dist_sq = (x1 - x2) * (x1 - x2) + (y1 - y2) * (y1 - y2);
        if dist_sq < min_dist_sq {
            min_dist_sq = dist_sq;
            min_loc = loc;
        }
    }
    Ok(min_loc)
}

fn shortest_link(
    this: &SvgElement,
    that: &SvgElement,
    conn_type: ConnectionType,
    context: &impl ElementMap,
) -> Result<(LocSpec, LocSpec)> {
    let mut min_dist_sq = f32::MAX;
    let mut this_min_loc = LocSpec::Center;
    let mut that_min_loc = LocSpec::Center;

    let this_bb = context
        .get_element_bbox(this)?
        .ok_or_else(|| Error::MissingBBox(this.to_string()))?;
    let that_bb = context
        .get_element_bbox(that)?
        .ok_or_else(|| Error::MissingBBox(that.to_string()))?;

    for this_loc in edge_locations(conn_type) {
        for that_loc in edge_locations(conn_type) {
            let this_coord = this_bb.locspec(this_loc);
            let that_coord = that_bb.locspec(that_loc);
            // always some as edge_locations does not include LineOffset
            let ((x1, y1), (x2, y2)) = (this_coord, that_coord);
            let dist_sq = (x1 - x2) * (x1 - x2) + (y1 - y2) * (y1 - y2);
            if dist_sq < min_dist_sq {
                min_dist_sq = dist_sq;
                this_min_loc = this_loc;
                that_min_loc = that_loc;
            }
        }
    }
    Ok((this_min_loc, that_min_loc))
}

#[allow(clippy::large_enum_variant)]
enum ElementParseData {
    El(SvgElement, Option<ElementLoc>, Option<Direction>),
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
            let mut retdir = None;
            let mut retloc = None;
            if let Some(loc) = loc {
                if let ElementLoc::LocSpec(ls) = loc {
                    retdir = Self::loc_to_dir(ls);
                }
                retloc = Some(loc);
            }
            Ok(ElementParseData::El(
                elem_map
                    .get_element(&elref)
                    .ok_or_else(|| Error::Reference(elref))?
                    .clone(),
                retloc,
                retdir,
            ))
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

    pub fn from_element(
        element: &SvgElement,
        elem_map: &impl ElementMap,
        conn_type: ConnectionType,
    ) -> Result<Self> {
        let mut element = element.clone();

        let start_ret = Self::parse_element(&mut element, elem_map, "start")?;
        let end_ret = Self::parse_element(&mut element, elem_map, "end")?;
        let (start_el, mut start_loc, mut start_dir, start_point) = match start_ret {
            ElementParseData::El(a, b, c) => (Some(a), b, c, None),
            ElementParseData::Point(x, y) => (None, None, None, Some((x, y))),
        };
        let (end_el, mut end_loc, mut end_dir, end_point) = match end_ret {
            ElementParseData::El(a, b, c) => (Some(a), b, c, None),
            ElementParseData::Point(x, y) => (None, None, None, Some((x, y))),
        };

        let offset = if let Some(o_inner) = element.pop_attr("corner-offset") {
            Some(
                strp_length(&o_inner)
                    .map_err(|_| Error::Parse("invalid corner-offset".to_owned()))?,
            )
        } else {
            None
        };

        // This could probably be tidier, trying to deal with lots of combinations.
        // Needs to support explicit coordinate pairs or element references, and
        // for element references support given locations or not (in which case
        // the location is determined automatically to give the shortest distance)

        let (start, end) = match (start_point, end_point) {
            (Some(start_point), Some(end_point)) => (
                Endpoint::new(start_point, start_dir),
                Endpoint::new(end_point, end_dir),
            ),
            (Some(start_point), None) => {
                let end_el = end_el
                    .as_ref()
                    .ok_or_else(|| Error::InternalLogic("no end_el".to_owned()))?;
                if end_loc.is_none() {
                    let eloc = closest_loc(end_el, start_point, conn_type, elem_map)?;
                    end_loc = Some(ElementLoc::LocSpec(eloc));
                    end_dir = Self::loc_to_dir(eloc);
                }
                let end_coord = end_el
                    .get_element_loc_coord(elem_map, end_loc.expect("Set from closest_loc"))?;
                (
                    Endpoint::new(start_point, start_dir),
                    Endpoint::new(end_coord, end_dir),
                )
            }
            (None, Some(end_point)) => {
                let start_el = start_el
                    .as_ref()
                    .ok_or_else(|| Error::InternalLogic("no start_el".to_owned()))?;
                if start_loc.is_none() {
                    let sloc = closest_loc(start_el, end_point, conn_type, elem_map)?;
                    start_loc = Some(ElementLoc::LocSpec(sloc));
                    start_dir = Self::loc_to_dir(sloc);
                }
                let start_coord = start_el
                    .get_element_loc_coord(elem_map, start_loc.expect("Set from closest_loc"))?;
                (
                    Endpoint::new(start_coord, start_dir),
                    Endpoint::new(end_point, end_dir),
                )
            }
            (None, None) => {
                let (start_el, end_el) = (
                    start_el
                        .as_ref()
                        .ok_or_else(|| Error::InternalLogic("no start_el".to_owned()))?,
                    end_el
                        .as_ref()
                        .ok_or_else(|| Error::InternalLogic("no end_el".to_owned()))?,
                );
                if start_loc.is_none() && end_loc.is_none() {
                    let (sloc, eloc) = shortest_link(start_el, end_el, conn_type, elem_map)?;
                    start_loc = Some(ElementLoc::LocSpec(sloc));
                    end_loc = Some(ElementLoc::LocSpec(eloc));
                    start_dir = Self::loc_to_dir(sloc);
                    end_dir = Self::loc_to_dir(eloc);
                } else if start_loc.is_none() {
                    let end_coord =
                        end_el.get_element_loc_coord(elem_map, end_loc.expect("Not both None"))?;
                    let sloc = closest_loc(start_el, end_coord, conn_type, elem_map)?;
                    start_loc = Some(ElementLoc::LocSpec(sloc));
                    start_dir = Self::loc_to_dir(sloc);
                } else if end_loc.is_none() {
                    let start_coord = start_el
                        .get_element_loc_coord(elem_map, start_loc.expect("Not both None"))?;
                    let eloc = closest_loc(end_el, start_coord, conn_type, elem_map)?;
                    end_loc = Some(ElementLoc::LocSpec(eloc));
                    end_dir = Self::loc_to_dir(eloc);
                }
                let start_coord =
                    start_el.get_element_loc_coord(elem_map, start_loc.expect("Set above"))?;
                let end_coord =
                    end_el.get_element_loc_coord(elem_map, end_loc.expect("Set above"))?;
                (
                    Endpoint::new(start_coord, start_dir),
                    Endpoint::new(end_coord, end_dir),
                )
            }
        };
        Ok(Self {
            source_element: element,
            start,
            end,
            start_el,
            end_el,
            conn_type,
            offset,
        })
    }

    pub fn render(&self, ctx: &impl ElementMap) -> Result<SvgElement> {
        let default_ratio_offset = Length::Ratio(0.5);
        let default_abs_offset = Length::Absolute(3.);

        let (x1, y1) = self.start.origin;
        let (x2, y2) = self.end.origin;
        // For some (e.g. u-shaped) connections we need a default *absolute* offset
        // as ratio (e.g. the overall '50%' default) don't make sense.
        let conn_element = match self.conn_type {
            ConnectionType::Horizontal => {
                // If we have start and end elements, use midpoint of overlapping region
                // TODO: If start_loc is specified, should probably set midpoint
                // to the y coord of that... (implies moving start_loc as an optional
                // inside Connector rather than evaluating it early)
                let midpoint =
                    if let (Some(start_el), Some(end_el)) = (&self.start_el, &self.end_el) {
                        let start_bb = ctx.get_element_bbox(start_el)?
                            .ok_or_else(|| Error::MissingBBox(start_el.to_string()))?;
                        let end_bb = ctx.get_element_bbox(end_el)?
                            .ok_or_else(|| Error::MissingBBox(end_el.to_string()))?;
                        let overlap_top = start_bb
                            .scalarspec(ScalarSpec::Miny)
                            .max(end_bb.scalarspec(ScalarSpec::Miny));
                        let overlap_bottom = start_bb
                            .scalarspec(ScalarSpec::Maxy)
                            .min(end_bb.scalarspec(ScalarSpec::Maxy));
                        (overlap_top + overlap_bottom) / 2.
                    } else {
                        y1
                    };
                SvgElement::new(
                    "line",
                    &[
                        ("x1".to_string(), fstr(x1)),
                        ("y1".to_string(), fstr(midpoint)),
                        ("x2".to_string(), fstr(x2)),
                        ("y2".to_string(), fstr(midpoint)),
                    ],
                )
                .with_attrs_from(&self.source_element)
            }
            ConnectionType::Vertical => {
                // If we have start and end elements, use midpoint of overlapping region
                let midpoint =
                    if let (Some(start_el), Some(end_el)) = (&self.start_el, &self.end_el) {
                        let start_bb = ctx
                            .get_element_bbox(start_el)?
                            .ok_or_else(|| Error::MissingBBox(start_el.to_string()))?;
                        let end_bb = ctx
                            .get_element_bbox(end_el)?
                            .ok_or_else(|| Error::MissingBBox(end_el.to_string()))?;
                        let overlap_left = start_bb
                            .scalarspec(ScalarSpec::Minx)
                            .max(end_bb.scalarspec(ScalarSpec::Minx));
                        let overlap_right = start_bb
                            .scalarspec(ScalarSpec::Maxx)
                            .min(end_bb.scalarspec(ScalarSpec::Maxx));
                        (overlap_left + overlap_right) / 2.
                    } else {
                        x1
                    };
                SvgElement::new(
                    "line",
                    &[
                        ("x1".to_string(), fstr(midpoint)),
                        ("y1".to_string(), fstr(y1)),
                        ("x2".to_string(), fstr(midpoint)),
                        ("y2".to_string(), fstr(y2)),
                    ],
                )
                .with_attrs_from(&self.source_element)
            }
            ConnectionType::Straight => SvgElement::new(
                "line",
                &[
                    ("x1".to_string(), fstr(x1)),
                    ("y1".to_string(), fstr(y1)),
                    ("x2".to_string(), fstr(x2)),
                    ("y2".to_string(), fstr(y2)),
                ],
            )
            .with_attrs_from(&self.source_element),
            ConnectionType::Corner => {
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

                // TODO: remove repeated points.
                if points.len() == 2 {
                    SvgElement::new(
                        "line",
                        &[
                            ("x1".to_string(), fstr(points[0].0)),
                            ("y1".to_string(), fstr(points[0].1)),
                            ("x2".to_string(), fstr(points[1].0)),
                            ("y2".to_string(), fstr(points[1].1)),
                        ],
                    )
                    .with_attrs_from(&self.source_element)
                } else {
                    SvgElement::new(
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
                    .with_attrs_from(&self.source_element)
                }
            }
        };
        Ok(conn_element)
    }
}
