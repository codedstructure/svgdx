use crate::transform::TransformerContext;
use crate::{fstr, strp, strp_length, Length, SvgElement};
use regex::Regex;

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
    fn new(origin: (f32, f32), dir: Option<Direction>) -> Self {
        Self { origin, dir }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum ConnectionType {
    Straight,
    Corner,
}

#[derive(Clone, Debug)]
pub(crate) struct Connector {
    source_element: SvgElement,
    start: Endpoint,
    end: Endpoint,
    conn_type: ConnectionType,
    offset: Length,
}

impl Connector {
    pub fn from_element(
        element: &SvgElement,
        context: &TransformerContext,
        conn_type: ConnectionType,
    ) -> Self {
        let mut element = element.clone();
        let start_ref = element.pop_attr("start").unwrap();
        let end_ref = element.pop_attr("end").unwrap();
        let offset = element
            .pop_attr("corner-offset")
            .unwrap_or(String::from("50%"));
        let offset = strp_length(&offset).expect("Invalid corner-offset value");

        // This could probably be tidier, trying to deal with lots of combinations.
        // Needs to support explicit coordinate pairs or element references, and
        // for element references support given locations or not (in which case
        // the location is determined automatically to give the shortest distance)
        let mut start_el = None;
        let mut end_el = None;
        let mut start_loc = String::from("");
        let mut end_loc = String::from("");
        let mut start_point: Option<(f32, f32)> = None;
        let mut end_point: Option<(f32, f32)> = None;
        let mut start_dir = None;
        let mut end_dir = None;

        let loc_to_dir = |dir: String| match dir.as_str() {
            "t" => Some(Direction::Up),
            "r" => Some(Direction::Right),
            "b" => Some(Direction::Down),
            "l" => Some(Direction::Left),
            _ => None,
        };

        // Example: "#thing@tl" => top left coordinate of element id="thing"
        let re = Regex::new(r"^#(?<id>[^@]+)(@(?<loc>\S+))?$").unwrap();

        if let Some(caps) = re.captures(&start_ref) {
            let name = &caps["id"];
            start_loc = caps.name("loc").map_or("", |v| v.as_str()).to_string();
            start_dir = loc_to_dir(start_loc.clone());
            start_el = context.elem_map.get(name);
        } else {
            let mut parts = element.attr_split(&start_ref).map(|v| strp(&v).unwrap());
            start_point = Some((parts.next().unwrap(), parts.next().unwrap()));
        }
        if let Some(caps) = re.captures(&end_ref) {
            let name = &caps["id"];
            end_loc = caps.name("loc").map_or("", |v| v.as_str()).to_string();
            end_dir = loc_to_dir(end_loc.clone());
            end_el = context.elem_map.get(name);
        } else {
            let mut parts = element.attr_split(&end_ref).map(|v| strp(&v).unwrap());
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
                    end_loc = context.closest_loc(end_el, start_point);
                    end_dir = loc_to_dir(end_loc.clone());
                }
                (
                    Endpoint::new(start_point, start_dir),
                    Endpoint::new(end_el.coord(&end_loc).unwrap(), end_dir),
                )
            }
            (None, Some(end_point)) => {
                let start_el = start_el.unwrap();
                if start_loc.is_empty() {
                    start_loc = context.closest_loc(start_el, end_point);
                    start_dir = loc_to_dir(start_loc.clone());
                }
                (
                    Endpoint::new(start_el.coord(&start_loc).unwrap(), start_dir),
                    Endpoint::new(end_point, end_dir),
                )
            }
            (None, None) => {
                let (start_el, end_el) = (start_el.unwrap(), end_el.unwrap());
                if start_loc.is_empty() && end_loc.is_empty() {
                    (start_loc, end_loc) = context.shortest_link(start_el, end_el);
                    start_dir = loc_to_dir(start_loc.clone());
                    end_dir = loc_to_dir(end_loc.clone());
                } else if start_loc.is_empty() {
                    start_loc = context.closest_loc(start_el, end_el.coord(&end_loc).unwrap());
                    start_dir = loc_to_dir(start_loc.clone());
                } else if end_loc.is_empty() {
                    end_loc = context.closest_loc(end_el, start_el.coord(&start_loc).unwrap());
                    end_dir = loc_to_dir(end_loc.clone());
                }
                (
                    Endpoint::new(start_el.coord(&start_loc).unwrap(), start_dir),
                    Endpoint::new(end_el.coord(&end_loc).unwrap(), end_dir),
                )
            }
        };
        Self {
            source_element: element,
            start,
            end,
            conn_type,
            offset,
        }
    }

    pub fn render(&self) -> SvgElement {
        let (x1, y1) = self.start.origin;
        let (x2, y2) = self.end.origin;
        match self.conn_type {
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
                                .calc_offset(self.start.origin.0, self.end.origin.0);
                            vec![(x1, y1), (mid_x, y1), (mid_x, y2), (x2, y2)]
                        }
                        (Direction::Up, Direction::Down) | (Direction::Down, Direction::Up) => {
                            let mid_y = self
                                .offset
                                .calc_offset(self.start.origin.1, self.end.origin.1);
                            vec![(x1, y1), (x1, mid_y), (x2, mid_y), (x2, y2)]
                        }
                        // U-shaped connection
                        (Direction::Left, Direction::Left) => {
                            let min_x = self.start.origin.0.min(self.end.origin.0);
                            let mid_x = min_x - self.offset.absolute().unwrap();
                            vec![(x1, y1), (mid_x, y1), (mid_x, y2), (x2, y2)]
                        }
                        (Direction::Right, Direction::Right) => {
                            let max_x = self.start.origin.0.max(self.end.origin.0);
                            let mid_x = max_x + self.offset.absolute().unwrap();
                            vec![(x1, y1), (mid_x, y1), (mid_x, y2), (x2, y2)]
                        }
                        (Direction::Up, Direction::Up) => {
                            let min_y = self.start.origin.1.min(self.end.origin.1);
                            let mid_y = min_y - self.offset.absolute().unwrap();
                            vec![(x1, y1), (x1, mid_y), (x2, mid_y), (x2, y2)]
                        }
                        (Direction::Down, Direction::Down) => {
                            let max_y = self.start.origin.1.max(self.end.origin.1);
                            let mid_y = max_y + self.offset.absolute().unwrap();
                            vec![(x1, y1), (x1, mid_y), (x2, mid_y), (x2, y2)]
                        }
                    };
                } else {
                    points = vec![(x1, y1), (x2, y2)];
                }
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
        }
    }
}
