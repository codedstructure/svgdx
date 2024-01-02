use crate::element::SvgElement;
use crate::transform::TransformerContext;
use crate::types::{attr_split, fstr, strp, strp_length, Length};
use regex::Regex;

use anyhow::{Context, Result};

#[derive(Clone, Copy, Debug)]
enum Direction {
    Up,
    Right,
    Down,
    Left,
}

#[derive(Clone, Copy, Debug)]
struct Endpoint {
    origin: (f32, f32),
    dir: Option<Direction>,
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

fn edge_locations(el: &SvgElement, ctype: ConnectionType) -> Vec<&str> {
    let mut result = match ctype {
        ConnectionType::Horizontal => vec!["l", "r"],
        ConnectionType::Vertical => vec!["t", "b"],
        ConnectionType::Corner => vec!["t", "r", "b", "l"],
        ConnectionType::Straight => vec!["t", "b", "l", "r", "tl", "bl", "tr", "br"],
    };
    if el.name == "line" {
        result.extend(&["xy1", "start", "xy2", "end"]);
    }
    result
}

#[derive(Clone, Debug)]
pub struct Connector {
    source_element: SvgElement,
    start_el: Option<SvgElement>,
    end_el: Option<SvgElement>,
    start: Endpoint,
    end: Endpoint,
    conn_type: ConnectionType,
    offset: Option<Length>,
}

fn closest_loc(this: &SvgElement, point: (f32, f32), conn_type: ConnectionType) -> Result<String> {
    let mut min_dist_sq = f32::MAX;
    let mut min_loc = "c";

    for loc in edge_locations(this, conn_type) {
        let this_coord = this.coord(loc)?;
        if let (Some((x1, y1)), (x2, y2)) = (this_coord, point) {
            let dist_sq = (x1 - x2) * (x1 - x2) + (y1 - y2) * (y1 - y2);
            if dist_sq < min_dist_sq {
                min_dist_sq = dist_sq;
                min_loc = loc;
            }
        }
    }
    Ok(min_loc.to_string())
}

fn shortest_link(
    this: &SvgElement,
    that: &SvgElement,
    conn_type: ConnectionType,
) -> Result<(String, String)> {
    let mut min_dist_sq = f32::MAX;
    let mut this_min_loc = "c";
    let mut that_min_loc = "c";
    for this_loc in edge_locations(this, conn_type) {
        for that_loc in edge_locations(that, conn_type) {
            let this_coord = this.coord(this_loc)?;
            let that_coord = that.coord(that_loc)?;
            if let (Some((x1, y1)), Some((x2, y2))) = (this_coord, that_coord) {
                let dist_sq = (x1 - x2) * (x1 - x2) + (y1 - y2) * (y1 - y2);
                if dist_sq < min_dist_sq {
                    min_dist_sq = dist_sq;
                    this_min_loc = this_loc;
                    that_min_loc = that_loc;
                }
            }
        }
    }
    Ok((this_min_loc.to_owned(), that_min_loc.to_owned()))
}

impl Connector {
    // TODO: This should take a LocSpec
    fn loc_to_dir(loc: &str) -> Option<Direction> {
        // loc may have a 'length' part following a colon, ignore that
        match loc.split(':').next().expect("always at least once") {
            "t" => Some(Direction::Up),
            "r" => Some(Direction::Right),
            "b" => Some(Direction::Down),
            "l" => Some(Direction::Left),
            _ => None,
        }
    }

    pub fn from_element(
        element: &SvgElement,
        context: &TransformerContext,
        conn_type: ConnectionType,
    ) -> Result<Self> {
        let mut element = element.clone();
        let start_ref = element.pop_attr("start").context("No 'start' element")?;
        let end_ref = element.pop_attr("end").context("No 'end' element")?;
        let offset = if let Some(o_inner) = element.pop_attr("corner-offset") {
            Some(strp_length(&o_inner).context("Invalid corner-offset")?)
        } else {
            None
        };

        // This could probably be tidier, trying to deal with lots of combinations.
        // Needs to support explicit coordinate pairs or element references, and
        // for element references support given locations or not (in which case
        // the location is determined automatically to give the shortest distance)
        let mut start_el = None;
        let mut end_el = None;
        let mut start_loc = String::new();
        let mut end_loc = String::new();
        let mut start_point: Option<(f32, f32)> = None;
        let mut end_point: Option<(f32, f32)> = None;
        let mut start_dir = None;
        let mut end_dir = None;

        // Example: "#thing@tl" => top left coordinate of element id="thing"
        let re = Regex::new(r"^#(?<id>[^@]+)(@(?<loc>\S+))?$").expect("Bad RegEx");

        if let Some(caps) = re.captures(&start_ref) {
            let name = &caps["id"];
            start_loc = caps.name("loc").map_or("", |v| v.as_str()).to_string();
            start_dir = Self::loc_to_dir(&start_loc);
            start_el = context.elem_map.get(name);
        } else {
            let mut parts = attr_split(&start_ref).map(|v| strp(&v).unwrap());
            start_point = Some((parts.next().unwrap(), parts.next().unwrap()));
        }
        if let Some(caps) = re.captures(&end_ref) {
            let name = &caps["id"];
            end_loc = caps.name("loc").map_or("", |v| v.as_str()).to_string();
            end_dir = Self::loc_to_dir(&end_loc);
            end_el = context.elem_map.get(name);
        } else {
            let mut parts = attr_split(&end_ref).map(|v| strp(&v).unwrap());
            end_point = Some((parts.next().unwrap(), parts.next().unwrap()));
        }

        let (start, end) = match (start_point, end_point) {
            (Some(start_point), Some(end_point)) => (
                Endpoint::new(start_point, start_dir),
                Endpoint::new(end_point, end_dir),
            ),
            (Some(start_point), None) => {
                let end_el = end_el.unwrap();
                if end_loc.is_empty() {
                    end_loc = closest_loc(end_el, start_point, conn_type)?;
                    end_dir = Self::loc_to_dir(&end_loc);
                }
                (
                    Endpoint::new(start_point, start_dir),
                    Endpoint::new(end_el.coord(&end_loc)?.unwrap(), end_dir),
                )
            }
            (None, Some(end_point)) => {
                let start_el = start_el.unwrap();
                if start_loc.is_empty() {
                    start_loc = closest_loc(start_el, end_point, conn_type)?;
                    start_dir = Self::loc_to_dir(&start_loc);
                }
                (
                    Endpoint::new(
                        start_el
                            .coord(&start_loc)?
                            .context("no coord for start_loc")?,
                        start_dir,
                    ),
                    Endpoint::new(end_point, end_dir),
                )
            }
            (None, None) => {
                let (start_el, end_el) = (start_el.unwrap(), end_el.unwrap());
                if start_loc.is_empty() && end_loc.is_empty() {
                    (start_loc, end_loc) = shortest_link(start_el, end_el, conn_type)?;
                    start_dir = Self::loc_to_dir(&start_loc);
                    end_dir = Self::loc_to_dir(&end_loc);
                } else if start_loc.is_empty() {
                    start_loc = closest_loc(start_el, end_el.coord(&end_loc)?.unwrap(), conn_type)?;
                    start_dir = Self::loc_to_dir(&start_loc);
                } else if end_loc.is_empty() {
                    end_loc = closest_loc(
                        end_el,
                        start_el
                            .coord(&start_loc)?
                            .context("no coord for start_loc")?,
                        conn_type,
                    )?;
                    end_dir = Self::loc_to_dir(&end_loc);
                }
                (
                    Endpoint::new(
                        start_el
                            .coord(&start_loc)?
                            .context("no coord for start_loc")?,
                        start_dir,
                    ),
                    Endpoint::new(end_el.coord(&end_loc)?.unwrap(), end_dir),
                )
            }
        };
        Ok(Self {
            source_element: element,
            start,
            end,
            start_el: start_el.cloned(),
            end_el: end_el.cloned(),
            conn_type,
            offset,
        })
    }

    pub fn render(&self) -> Result<SvgElement> {
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
                        let start_bb = start_el.bbox()?.context("start element bbox")?;
                        let end_bb = end_el.bbox()?.context("end element bbox")?;
                        let overlap_top = start_bb
                            .scalarspec(crate::types::ScalarSpec::Miny)
                            .max(end_bb.scalarspec(crate::types::ScalarSpec::Miny));
                        let overlap_bottom = start_bb
                            .scalarspec(crate::types::ScalarSpec::Maxy)
                            .min(end_bb.scalarspec(crate::types::ScalarSpec::Maxy));
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
                        let start_bb = start_el.bbox()?.context("start element bbox")?;
                        let end_bb = end_el.bbox()?.context("end element bbox")?;
                        let overlap_left = start_bb
                            .scalarspec(crate::types::ScalarSpec::Minx)
                            .max(end_bb.scalarspec(crate::types::ScalarSpec::Minx));
                        let overlap_right = start_bb
                            .scalarspec(crate::types::ScalarSpec::Maxx)
                            .min(end_bb.scalarspec(crate::types::ScalarSpec::Maxx));
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
                let points;
                if let (Some(start_dir_some), Some(end_dir_some)) = (self.start.dir, self.end.dir) {
                    points = match (start_dir_some, end_dir_some) {
                        // L-shaped connection
                        (Direction::Up | Direction::Down, Direction::Left | Direction::Right) => {
                            vec![(x1, y1), (self.start.origin.0, self.end.origin.1), (x2, y2)]
                        }
                        (Direction::Left | Direction::Right, Direction::Up | Direction::Down) => {
                            vec![(x1, y1), (self.end.origin.0, self.start.origin.1), (x2, y2)]
                        }
                        // Z-shaped connection
                        (Direction::Left, Direction::Right)
                        | (Direction::Right, Direction::Left) => {
                            let mid_x = self
                                .offset
                                .unwrap_or(default_ratio_offset)
                                .calc_offset(self.start.origin.0, self.end.origin.0);
                            vec![(x1, y1), (mid_x, y1), (mid_x, y2), (x2, y2)]
                        }
                        (Direction::Up, Direction::Down) | (Direction::Down, Direction::Up) => {
                            let mid_y = self
                                .offset
                                .unwrap_or(default_ratio_offset)
                                .calc_offset(self.start.origin.1, self.end.origin.1);
                            vec![(x1, y1), (x1, mid_y), (x2, mid_y), (x2, y2)]
                        }
                        // U-shaped connection
                        (Direction::Left, Direction::Left) => {
                            let min_x = self.start.origin.0.min(self.end.origin.0);
                            let mid_x = min_x
                                - self
                                    .offset
                                    .unwrap_or(default_abs_offset)
                                    .absolute()
                                    .context("Corner type requires absolute offset")?;
                            vec![(x1, y1), (mid_x, y1), (mid_x, y2), (x2, y2)]
                        }
                        (Direction::Right, Direction::Right) => {
                            let max_x = self.start.origin.0.max(self.end.origin.0);
                            let mid_x = max_x
                                + self
                                    .offset
                                    .unwrap_or(default_abs_offset)
                                    .absolute()
                                    .context("Corner type requires absolute offset")?;

                            vec![(x1, y1), (mid_x, y1), (mid_x, y2), (x2, y2)]
                        }
                        (Direction::Up, Direction::Up) => {
                            let min_y = self.start.origin.1.min(self.end.origin.1);
                            let mid_y = min_y
                                - self
                                    .offset
                                    .unwrap_or(default_abs_offset)
                                    .absolute()
                                    .context("Corner type requires absolute offset")?;

                            vec![(x1, y1), (x1, mid_y), (x2, mid_y), (x2, y2)]
                        }
                        (Direction::Down, Direction::Down) => {
                            let max_y = self.start.origin.1.max(self.end.origin.1);
                            let mid_y = max_y
                                + self
                                    .offset
                                    .unwrap_or(default_abs_offset)
                                    .absolute()
                                    .context("Corner type requires absolute offset")?;

                            vec![(x1, y1), (x1, mid_y), (x2, mid_y), (x2, y2)]
                        }
                    };
                } else {
                    points = vec![(x1, y1), (x2, y2)];
                }
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
