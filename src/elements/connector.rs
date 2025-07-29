use std::cmp::Ordering;
use std::collections::BinaryHeap;

use super::SvgElement;
use crate::context::ElementMap;
use crate::errors::{Result, SvgdxError};
use crate::geometry::{parse_el_loc, strp_length, BoundingBox, Length, LocSpec, ScalarSpec};
use crate::types::{attr_split, fstr, strp};

pub fn is_connector(el: &SvgElement) -> bool {
    el.has_attr("start") && el.has_attr("end") && (el.name() == "line" || el.name() == "polyline")
}

#[derive(Clone, Copy, Debug, PartialEq)]
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
    start: Endpoint,
    end: Endpoint,
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
        .ok_or_else(|| SvgdxError::MissingBoundingBox(this.to_string()))?;

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
        .ok_or_else(|| SvgdxError::MissingBoundingBox(this.to_string()))?;
    let that_bb = context
        .get_element_bbox(that)?
        .ok_or_else(|| SvgdxError::MissingBoundingBox(that.to_string()))?;

    for this_loc in edge_locations(conn_type) {
        for that_loc in edge_locations(conn_type) {
            let this_coord = this_bb.locspec(this_loc);
            let that_coord = that_bb.locspec(that_loc);
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

#[derive(PartialEq, Eq)]
struct HeapData {
    cost: u32,
    ind: usize,
}

impl Ord for HeapData {
    // from the exaple heap docs
    fn cmp(&self, other: &Self) -> Ordering {
        // Notice that we flip the ordering on costs.
        // In case of a tie we compare positions - this step is necessary
        // to make implementations of `PartialEq` and `Ord` consistent.
        other
            .cost
            .cmp(&self.cost)
            .then_with(|| self.ind.cmp(&other.ind))
    }
}

impl PartialOrd for HeapData {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
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

    fn parse_element<'a>(
        element: &mut SvgElement,
        elem_map: &'a impl ElementMap,
        start: bool,
    ) -> Result<(
        Option<&'a SvgElement>,
        Option<LocSpec>,
        Option<(f32, f32)>,
        Option<Direction>,
    )> {
        let attrib_name = if start { "start" } else { "end" };
        let this_ref = element
            .pop_attr(attrib_name)
            .ok_or_else(|| SvgdxError::MissingAttribute(attrib_name.to_string()))?;

        let mut t_el: Option<&SvgElement> = None;
        let mut t_loc: Option<LocSpec> = None;
        let mut t_point: Option<(f32, f32)> = None;
        let mut t_dir: Option<Direction> = None;

        // Example: "#thing@tl" => top left coordinate of element id="thing"
        if let Ok((elref, loc)) = parse_el_loc(&this_ref) {
            if let Some(loc) = loc {
                t_dir = Self::loc_to_dir(loc);
                t_loc = Some(loc);
            }
            t_el = elem_map.get_element(&elref);
        } else {
            let mut parts = attr_split(&this_ref).map_while(|v| strp(&v).ok());
            t_point = Some((
                parts.next().ok_or_else(|| {
                    SvgdxError::InvalidData(
                        (attrib_name.to_owned() + "_ref x should be numeric").to_owned(),
                    )
                })?,
                parts.next().ok_or_else(|| {
                    SvgdxError::InvalidData(
                        (attrib_name.to_owned() + "_ref y should be numeric").to_owned(),
                    )
                })?,
            ));
        }

        return Ok((t_el, t_loc, t_point, t_dir));
    }

    pub fn from_element(
        element: &SvgElement,
        elem_map: &impl ElementMap,
        conn_type: ConnectionType,
    ) -> Result<Self> {
        let mut element = element.clone();

        let (start_el, mut start_loc, start_point, mut start_dir) =
            Self::parse_element(&mut element, elem_map, true)?;
        let (end_el, mut end_loc, end_point, mut end_dir) =
            Self::parse_element(&mut element, elem_map, false)?;

        let offset = if let Some(o_inner) = element.pop_attr("corner-offset") {
            Some(
                strp_length(&o_inner)
                    .map_err(|_| SvgdxError::ParseError("Invalid corner-offset".to_owned()))?,
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
                let end_el =
                    end_el.ok_or_else(|| SvgdxError::InternalLogicError("no end_el".to_owned()))?;
                if end_loc.is_none() {
                    let eloc = closest_loc(end_el, start_point, conn_type, elem_map)?;
                    end_loc = Some(eloc);
                    end_dir = Self::loc_to_dir(eloc);
                }
                let end_coord = elem_map
                    .get_element_bbox(end_el)?
                    .ok_or_else(|| SvgdxError::MissingBoundingBox(end_el.to_string()))?
                    .locspec(end_loc.expect("Set from closest_loc"));
                (
                    Endpoint::new(start_point, start_dir),
                    Endpoint::new(end_coord, end_dir),
                )
            }
            (None, Some(end_point)) => {
                let start_el = start_el
                    .ok_or_else(|| SvgdxError::InternalLogicError("no start_el".to_owned()))?;
                if start_loc.is_none() {
                    let sloc = closest_loc(start_el, end_point, conn_type, elem_map)?;
                    start_loc = Some(sloc);
                    start_dir = Self::loc_to_dir(sloc);
                }
                let start_coord = elem_map
                    .get_element_bbox(start_el)?
                    .ok_or_else(|| SvgdxError::MissingBoundingBox(start_el.to_string()))?
                    .locspec(start_loc.expect("Set from closest_loc"));
                (
                    Endpoint::new(start_coord, start_dir),
                    Endpoint::new(end_point, end_dir),
                )
            }
            (None, None) => {
                let (start_el, end_el) = (
                    start_el
                        .ok_or_else(|| SvgdxError::InternalLogicError("no start_el".to_owned()))?,
                    end_el.ok_or_else(|| SvgdxError::InternalLogicError("no end_el".to_owned()))?,
                );
                if start_loc.is_none() && end_loc.is_none() {
                    let (sloc, eloc) = shortest_link(start_el, end_el, conn_type, elem_map)?;
                    start_loc = Some(sloc);
                    end_loc = Some(eloc);
                    start_dir = Self::loc_to_dir(sloc);
                    end_dir = Self::loc_to_dir(eloc);
                } else if start_loc.is_none() {
                    let end_coord = elem_map
                        .get_element_bbox(end_el)?
                        .ok_or_else(|| SvgdxError::MissingBoundingBox(end_el.to_string()))?
                        .locspec(end_loc.expect("Not both None"));
                    let sloc = closest_loc(start_el, end_coord, conn_type, elem_map)?;
                    start_loc = Some(sloc);
                    start_dir = Self::loc_to_dir(sloc);
                } else if end_loc.is_none() {
                    let start_coord = elem_map
                        .get_element_bbox(start_el)?
                        .ok_or_else(|| SvgdxError::MissingBoundingBox(start_el.to_string()))?
                        .locspec(start_loc.expect("Not both None"));
                    let eloc = closest_loc(end_el, start_coord, conn_type, elem_map)?;
                    end_loc = Some(eloc);
                    end_dir = Self::loc_to_dir(eloc);
                }
                let start_coord = elem_map
                    .get_element_bbox(start_el)?
                    .ok_or_else(|| SvgdxError::MissingBoundingBox(start_el.to_string()))?
                    .locspec(start_loc.expect("Set above"));
                let end_coord = elem_map
                    .get_element_bbox(end_el)?
                    .ok_or_else(|| SvgdxError::MissingBoundingBox(end_el.to_string()))?
                    .locspec(end_loc.expect("Set above"));
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
            start_el: start_el.cloned(),
            end_el: end_el.cloned(),
            conn_type,
            offset,
        })
    }

    fn aals_blocked_by_bb(bb: BoundingBox, a: f32, b: f32, x_axis: bool, axis_val: f32) -> bool {
        if x_axis {
            if axis_val < bb.y1 || axis_val > bb.y2 {
                return false;
            }
            if (a < bb.x1) == (b < bb.x1) && (a > bb.x2) == (b > bb.x2) {
                return false;
            }
        } else {
            if axis_val < bb.x1 || axis_val > bb.x2 {
                return false;
            }
            if (a < bb.y1) == (b < bb.y1) && (a > bb.y2) == (b > bb.y2) {
                return false;
            }
        }

        return true;
    }

    fn render_match_conner(
        &self,
        ratio_offset: f32,
        start_abs_offset: f32,
        end_abs_offset: f32,
        sel_bb: BoundingBox,
        eel_bb: BoundingBox,
        abs_offset_set: bool,
    ) -> Result<Vec<(f32, f32)>> {
        let (x1, y1) = self.start.origin;
        let (x2, y2) = self.end.origin;

        // method generates all points it could possibly want to go through then does dijkstras on it

        let mut points: Vec<(f32, f32)>;
        if let (Some(start_dir_some), Some(end_dir_some)) = (self.start.dir, self.end.dir) {
            points = vec![];

            // x_lines have constant x vary over y
            let mut x_lines = vec![];
            let mut y_lines = vec![];
            let mut point_set = vec![];
            let mut mid_x = std::usize::MAX;
            let mut mid_y = std::usize::MAX;

            x_lines.push(sel_bb.x1 - start_abs_offset);
            x_lines.push(sel_bb.x2 + start_abs_offset);
            x_lines.push(eel_bb.x1 - end_abs_offset);
            x_lines.push(eel_bb.x2 + end_abs_offset);

            if sel_bb.x1 > eel_bb.x2 {
                // there is a gap
                x_lines.push((sel_bb.x1 + eel_bb.x2) * 0.5);
                mid_x = x_lines.len() - 1;
            } else if sel_bb.x2 < eel_bb.x1 {
                // there is a gap
                x_lines.push((sel_bb.x2 + eel_bb.x1) * 0.5);
                mid_x = x_lines.len() - 1;
            }

            y_lines.push(sel_bb.y1 - start_abs_offset);
            y_lines.push(sel_bb.y2 + start_abs_offset);
            y_lines.push(eel_bb.y1 - end_abs_offset);
            y_lines.push(eel_bb.y2 + end_abs_offset);

            if sel_bb.y1 > eel_bb.y2 {
                // there is a gap
                y_lines.push(sel_bb.y1 * (1.0 - ratio_offset) + eel_bb.y2 * ratio_offset);
                mid_y = y_lines.len() - 1;
            } else if sel_bb.y2 < eel_bb.y1 {
                // there is a gap
                y_lines.push(sel_bb.y2 * (1.0 - ratio_offset) + eel_bb.y1 * ratio_offset);
                mid_y = y_lines.len() - 1;
            }

            match start_dir_some {
                Direction::Left | Direction::Right => {
                    y_lines.push(y1);
                }
                Direction::Down | Direction::Up => {
                    x_lines.push(x1);
                }
            }

            match end_dir_some {
                Direction::Left | Direction::Right => {
                    y_lines.push(y2);
                }
                Direction::Down | Direction::Up => {
                    x_lines.push(x2);
                }
            }

            if abs_offset_set {
                match start_dir_some {
                    Direction::Down => mid_y = 1,  // positive x
                    Direction::Left => mid_x = 0,  // negative y
                    Direction::Right => mid_x = 1, // positive y
                    Direction::Up => mid_y = 0,    // positive x
                }
            }

            for i in 0..x_lines.len() {
                for j in 0..y_lines.len() {
                    point_set.push((x_lines[i], y_lines[j]));
                }
            }

            let mut edge_set = vec![vec![]; point_set.len()];

            for i in 0..point_set.len() {
                for j in 0..point_set.len() {
                    if i == j {
                        continue;
                    }
                    let mut connected = false;

                    // check if not blocked by a wall
                    if point_set[i].0 == point_set[j].0 {
                        if !Self::aals_blocked_by_bb(
                            sel_bb,
                            point_set[i].1,
                            point_set[j].1,
                            false,
                            point_set[i].0,
                        ) && !Self::aals_blocked_by_bb(
                            eel_bb,
                            point_set[i].1,
                            point_set[j].1,
                            false,
                            point_set[i].0,
                        ) {
                            connected = true;
                        }
                    } else if point_set[i].1 == point_set[j].1 {
                        if !Self::aals_blocked_by_bb(
                            sel_bb,
                            point_set[i].0,
                            point_set[j].0,
                            true,
                            point_set[i].1,
                        ) && !Self::aals_blocked_by_bb(
                            eel_bb,
                            point_set[i].0,
                            point_set[j].0,
                            true,
                            point_set[i].1,
                        ) {
                            connected = true;
                        }
                    }
                    if connected {
                        edge_set[i].push(j);
                        edge_set[j].push(i);
                    }
                }
            }

            // just needs to be bigger than 5* (corner cost  +  total bounding box size)
            let inf = 1000000;

            point_set.push((x1, y1)); // start
            point_set.push((x2, y2)); // end
            edge_set.push(vec![]);
            edge_set.push(vec![]);
            let mut queue: BinaryHeap<HeapData> = BinaryHeap::new();
            let mut dist = vec![inf; point_set.len()];

            let start_ind = edge_set.len() - 2;
            let end_ind = edge_set.len() - 1;
            for i in 0..point_set.len() - 2 {
                if (point_set[i].0 == x1 && point_set[i].1 < y1 && start_dir_some == Direction::Up)
                    || (point_set[i].0 == x1
                        && point_set[i].1 > y1
                        && start_dir_some == Direction::Down)
                {
                    if !Self::aals_blocked_by_bb(eel_bb, point_set[i].1, y1, false, x1) {
                        edge_set[i].push(start_ind);
                        edge_set[start_ind].push(i);
                    }
                }
                if (point_set[i].1 == y1
                    && point_set[i].0 > x1
                    && start_dir_some == Direction::Right)
                    || (point_set[i].1 == y1
                        && point_set[i].0 < x1
                        && start_dir_some == Direction::Left)
                {
                    if !Self::aals_blocked_by_bb(eel_bb, point_set[i].0, x1, true, y1) {
                        edge_set[i].push(start_ind);
                        edge_set[start_ind].push(i);
                    }
                }

                if (point_set[i].0 == x2 && point_set[i].1 < y2 && end_dir_some == Direction::Up)
                    || (point_set[i].0 == x2
                        && point_set[i].1 > y2
                        && end_dir_some == Direction::Down)
                {
                    if !Self::aals_blocked_by_bb(sel_bb, point_set[i].1, y2, false, x2) {
                        edge_set[i].push(end_ind);
                        edge_set[end_ind].push(i);
                    }
                }
                if (point_set[i].1 == y2 && point_set[i].0 > x2 && end_dir_some == Direction::Right)
                    || (point_set[i].1 == y2
                        && point_set[i].0 < x2
                        && end_dir_some == Direction::Left)
                {
                    if !Self::aals_blocked_by_bb(sel_bb, point_set[i].0, x2, true, y2) {
                        edge_set[i].push(end_ind);
                        edge_set[end_ind].push(i);
                    }
                }
            }

            // edge cost function
            let corner_cost = 1000;
            let mut edge_costs = vec![vec![]; edge_set.len()];
            for i in 0..edge_set.len() {
                for j in 0..edge_set[i].len() {
                    let ind_1 = i;
                    let ind_2 = edge_set[i][j];

                    let mid_point_mul_x =
                        if mid_x != std::usize::MAX && point_set[ind_1].0 == x_lines[mid_x] {
                            0.5
                        } else {
                            1.0
                        };
                    let mid_point_mul_y =
                        if mid_y != std::usize::MAX && point_set[ind_1].1 == y_lines[mid_y] {
                            0.5
                        } else {
                            1.0
                        };

                    edge_costs[i].push(
                        ((point_set[ind_1].0 - point_set[ind_2].0).abs() * mid_point_mul_y
                            + (point_set[ind_1].1 - point_set[ind_2].1).abs() * mid_point_mul_x)
                            as u32
                            + corner_cost,
                    ); // round may cause some problems
                }
            }

            dist[start_ind] = 0;
            queue.push(HeapData {
                cost: 0,
                ind: start_ind,
            });

            // cant get stuck in a loop as cost for a distance either decreases or queue shrinks
            while !queue.is_empty() {
                let next = queue.pop().expect("would not be in while loop");
                if next.ind == end_ind {
                    break;
                }

                // the node is reached by faster means so already popped
                if next.cost > dist[next.ind] {
                    continue;
                }

                for i in 0..edge_set[next.ind].len() {
                    let edge_cost = edge_costs[next.ind][i];
                    if dist[next.ind] + edge_cost < dist[edge_set[next.ind][i]] {
                        dist[edge_set[next.ind][i]] = dist[next.ind] + edge_cost;
                        queue.push(HeapData {
                            cost: dist[edge_set[next.ind][i]],
                            ind: edge_set[next.ind][i],
                        });
                    }
                }
            }

            let mut back_points_inds = vec![end_ind];
            let mut loc = end_ind;
            while loc != start_ind {
                // would get stuck in a loop if no valid solution
                let mut quit = true;
                for i in 0..edge_set[loc].len() {
                    if dist[edge_set[loc][i]] + edge_costs[loc][i] == dist[loc] {
                        loc = edge_set[loc][i];
                        back_points_inds.push(loc);
                        quit = false;
                        break;
                    }
                }
                if quit {
                    break;
                }
            }

            for i in (0..back_points_inds.len()).rev() {
                points.push(point_set[back_points_inds[i]]);
            }
        } else {
            points = vec![(x1, y1), (x2, y2)];
        }

        return Ok(points);
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
                        let start_bb = start_el
                            .bbox()?
                            .ok_or_else(|| SvgdxError::MissingBoundingBox(start_el.to_string()))?;
                        let end_bb = end_el
                            .bbox()?
                            .ok_or_else(|| SvgdxError::MissingBoundingBox(end_el.to_string()))?;
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
                            .ok_or_else(|| SvgdxError::MissingBoundingBox(start_el.to_string()))?;
                        let end_bb = ctx
                            .get_element_bbox(end_el)?
                            .ok_or_else(|| SvgdxError::MissingBoundingBox(end_el.to_string()))?;
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
                let mut start_abs_offset = default_abs_offset.absolute().ok_or("blarg 13872199")?;
                let mut end_abs_offset = start_abs_offset;
                let mut ratio_offset = default_ratio_offset.ratio().ok_or("blarg 13872198")?;
                if let Some(offset) = &self.offset {
                    if let Some(o) = offset.absolute() {
                        start_abs_offset = o;
                        end_abs_offset = o;
                        abs_offset_set = true;
                    }
                    if let Some(r) = offset.ratio() {
                        ratio_offset = r;
                    }
                }

                let mut sel_bb = BoundingBox::new(x1, y1, x1, y1);
                let mut eel_bb = BoundingBox::new(x2, y2, x2, y2);
                if let Some(el) = &self.start_el {
                    if let Ok(Some(el_bb)) = el.bbox() {
                        sel_bb = el_bb;
                    }
                }
                if let Some(el) = &self.end_el {
                    if let Ok(Some(el_bb)) = el.bbox() {
                        eel_bb = el_bb;
                    }
                }
                let points = self.render_match_conner(
                    ratio_offset,
                    start_abs_offset,
                    end_abs_offset,
                    sel_bb,
                    eel_bb,
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
