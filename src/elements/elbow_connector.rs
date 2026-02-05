use super::connector::{Direction, Endpoint, LineConnector, ParsedEndpoint};
use super::SvgElement;
use crate::context::ElementMap;
use crate::errors::{Error, Result};
use crate::geometry::{strp_length, BoundingBox, Length, LocSpec};
use crate::types::fstr;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

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

#[derive(Clone)]
pub struct ElbowConnector {
    source_element: SvgElement,
    start_el: Option<SvgElement>,
    end_el: Option<SvgElement>,
    pub start: Endpoint,
    pub end: Endpoint,
    offset: Option<Length>,
}

impl ElbowConnector {
    /// Resolve a fixed point against a bbox using cardinal direction.
    fn resolve_point_to_bbox(
        point: (f32, f32),
        bb: &BoundingBox,
    ) -> ((f32, f32), Option<Direction>) {
        let loc = closest_cardinal_loc(point, bb);
        (bb.locspec(loc), LineConnector::loc_to_dir(loc))
    }

    /// Resolve two bboxes using cardinal-direction shortest link.
    fn resolve_bbox_to_bbox(start_bb: &BoundingBox, end_bb: &BoundingBox) -> BBoxResolution {
        let (start_loc, end_loc) = shortest_cardinal_link(start_bb, end_bb);
        BBoxResolution {
            start_coord: start_bb.locspec(start_loc),
            end_coord: end_bb.locspec(end_loc),
            start_dir: LineConnector::loc_to_dir(start_loc),
            end_dir: LineConnector::loc_to_dir(end_loc),
        }
    }

    pub fn from_element(element: &SvgElement, elem_map: &impl ElementMap) -> Result<Self> {
        let mut element = element.clone();

        // Ignore deprecated attribute
        let _ = element.pop_attr("edge-type");

        let start_ret = LineConnector::parse_element(&mut element, elem_map, "start")?;
        let end_ret = LineConnector::parse_element(&mut element, elem_map, "end")?;

        let offset = element
            .pop_attr("corner-offset")
            .map(|o| strp_length(&o).map_err(|_| Error::Parse("invalid corner-offset".to_owned())))
            .transpose()?;

        let (start_fixed, start_data): ParsedEndpoint =
            LineConnector::to_fixed_or_bbox(&start_ret, elem_map)?;
        let (end_fixed, end_data): ParsedEndpoint =
            LineConnector::to_fixed_or_bbox(&end_ret, elem_map)?;

        let (start, end, start_el, end_el) = match (start_fixed, end_fixed) {
            // Both resolved to fixed points
            (Some((x1, y1, dir1)), Some((x2, y2, dir2))) => (
                Endpoint::new((x1, y1), dir1),
                Endpoint::new((x2, y2), dir2),
                start_data.map(|(el, _)| el),
                end_data.map(|(el, _)| el),
            ),

            // Start is fixed, end needs resolution
            (Some((x1, y1, dir1)), None) => {
                let (end_el, end_bb) = end_data.expect("end must have element if not fixed");
                let (end_coord, end_dir) = Self::resolve_point_to_bbox((x1, y1), &end_bb);
                (
                    Endpoint::new((x1, y1), dir1),
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
                    Endpoint::new((x2, y2), dir2),
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
        })
    }

    fn render_corner(&self, ctx: &impl ElementMap) -> Result<SvgElement> {
        let (x1, y1) = self.start.origin;
        let (x2, y2) = self.end.origin;

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

    pub fn render(&self, ctx: &impl ElementMap) -> Result<Option<SvgElement>> {
        self.render_corner(ctx).map(Some)
    }
}

/// Remove identical and collinear point pairs
fn filter_points(p: Vec<(f32, f32)>) -> Vec<(f32, f32)> {
    let mut ret = Vec::with_capacity(p.len());
    const EPSILON: f32 = 1e-6;

    for current in p {
        if ret.last().copied() == Some(current) {
            continue;
        }

        if ret.len() >= 2 {
            let prev2 = ret[ret.len() - 2];
            let prev1 = ret[ret.len() - 1];

            let v1 = (prev1.0 - prev2.0, prev1.1 - prev2.1);
            let v2 = (current.0 - prev1.0, current.1 - prev1.1);

            let cross = v1.0 * v2.1 - v1.1 * v2.0;
            let collinear = cross.abs() < EPSILON;

            if collinear {
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

#[derive(PartialEq, Eq)]
struct PathCost {
    cost: u32,
    idx: usize,
    dir: Direction,
    from: usize,
    from_dir: Direction,
}

impl Ord for PathCost {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .cmp(&self.cost)
            .then_with(|| self.idx.cmp(&other.idx))
    }
}

impl PartialOrd for PathCost {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug)]
struct NodeState {
    ind: usize,
    dir: Direction,
}

struct LineStruct {
    x_lines: Vec<f32>,
    y_lines: Vec<f32>,
    mid_x: usize,
    mid_y: usize,
}

fn aals_blocked_by_bb(bb: BoundingBox, a: f32, b: f32, x_axis: bool, axis_val: f32) -> bool {
    if x_axis {
        if axis_val <= bb.y1 || axis_val >= bb.y2 {
            return false;
        }
        if (a <= bb.x1) == (b <= bb.x1) && (a >= bb.x2) == (b >= bb.x2) {
            return false;
        }
    } else {
        if axis_val <= bb.x1 || axis_val >= bb.x2 {
            return false;
        }
        if (a <= bb.y1) == (b <= bb.y1) && (a >= bb.y2) == (b >= bb.y2) {
            return false;
        }
    }

    true
}

fn get_lines(
    connector: &ElbowConnector,
    ratio_offset: f32,
    abs_offsets: (f32, f32),
    bbs: (BoundingBox, BoundingBox),
    abs_offset_set: bool,
    dirs: (Direction, Direction),
) -> LineStruct {
    let (x1, y1) = connector.start.origin;
    let (x2, y2) = connector.end.origin;
    let (start_el_bb, end_el_bb) = bbs;
    let (start_abs_offset, end_abs_offset) = abs_offsets;
    let (start_dir, end_dir) = dirs;

    let mut x_lines = vec![];
    let mut y_lines = vec![];
    let mut mid_x = usize::MAX;
    let mut mid_y = usize::MAX;

    x_lines.push(start_el_bb.x1 - start_abs_offset);
    x_lines.push(start_el_bb.x2 + start_abs_offset);
    x_lines.push(end_el_bb.x1 - end_abs_offset);
    x_lines.push(end_el_bb.x2 + end_abs_offset);

    if start_el_bb.x1 > end_el_bb.x2 {
        x_lines.push(start_el_bb.x1 * (1.0 - ratio_offset) + end_el_bb.x2 * ratio_offset);
        mid_x = x_lines.len() - 1;
    } else if start_el_bb.x2 < end_el_bb.x1 {
        x_lines.push(start_el_bb.x2 * (1.0 - ratio_offset) + end_el_bb.x1 * ratio_offset);
        mid_x = x_lines.len() - 1;
    }

    y_lines.push(start_el_bb.y1 - start_abs_offset);
    y_lines.push(start_el_bb.y2 + start_abs_offset);
    y_lines.push(end_el_bb.y1 - end_abs_offset);
    y_lines.push(end_el_bb.y2 + end_abs_offset);

    if start_el_bb.y1 > end_el_bb.y2 {
        y_lines.push(start_el_bb.y1 * (1.0 - ratio_offset) + end_el_bb.y2 * ratio_offset);
        mid_y = y_lines.len() - 1;
    } else if start_el_bb.y2 < end_el_bb.y1 {
        y_lines.push(start_el_bb.y2 * (1.0 - ratio_offset) + end_el_bb.y1 * ratio_offset);
        mid_y = y_lines.len() - 1;
    }

    match start_dir {
        Direction::Left | Direction::Right => {
            y_lines.push(y1);
        }
        Direction::Down | Direction::Up => {
            x_lines.push(x1);
        }
    }

    match end_dir {
        Direction::Left | Direction::Right => {
            y_lines.push(y2);
        }
        Direction::Down | Direction::Up => {
            x_lines.push(x2);
        }
    }

    if abs_offset_set {
        match start_dir {
            Direction::Down => mid_y = 1,
            Direction::Left => mid_x = 0,
            Direction::Right => mid_x = 1,
            Direction::Up => mid_y = 0,
        }
    }

    LineStruct {
        x_lines,
        y_lines,
        mid_x,
        mid_y,
    }
}

fn get_edges(
    point_set: &[(f32, f32)],
    start_el_bb: BoundingBox,
    end_el_bb: BoundingBox,
) -> Vec<(Vec<usize>, Vec<usize>)> {
    let mut edge_set = vec![(vec![], vec![]); point_set.len()];

    for i in 0..point_set.len() {
        for j in 0..point_set.len() {
            if i == j {
                continue;
            }

            if point_set[i].0 == point_set[j].0
                && !aals_blocked_by_bb(
                    start_el_bb,
                    point_set[i].1,
                    point_set[j].1,
                    false,
                    point_set[i].0,
                )
                && !aals_blocked_by_bb(
                    end_el_bb,
                    point_set[i].1,
                    point_set[j].1,
                    false,
                    point_set[i].0,
                )
            {
                edge_set[i].1.push(j);
                edge_set[j].1.push(i);
            }

            if point_set[i].1 == point_set[j].1
                && !aals_blocked_by_bb(
                    start_el_bb,
                    point_set[i].0,
                    point_set[j].0,
                    true,
                    point_set[i].1,
                )
                && !aals_blocked_by_bb(
                    end_el_bb,
                    point_set[i].0,
                    point_set[j].0,
                    true,
                    point_set[i].1,
                )
            {
                edge_set[i].0.push(j);
                edge_set[j].0.push(i);
            }
        }
    }

    edge_set
}

struct Graph {
    point_set: Vec<(f32, f32)>,
    edge_set: Vec<(Vec<usize>, Vec<usize>)>,
    edge_costs: Vec<(Vec<u32>, Vec<u32>)>,
}

fn add_start_and_end(
    connector: &ElbowConnector,
    graph: &mut Graph,
    abs_offsets: (f32, f32),
    bbs: (BoundingBox, BoundingBox),
    dirs: (Direction, Direction),
    line_struct: &LineStruct,
    corner_cost: u32,
) -> (usize, usize) {
    let (x1, y1) = connector.start.origin;
    let (x2, y2) = connector.end.origin;
    let (start_el_bb, end_el_bb) = bbs;
    let (start_abs_offset, end_abs_offset) = abs_offsets;
    let (start_dir, end_dir) = dirs;

    let offset = |dir: Direction, abs_offset: f32| match dir {
        Direction::Up => -abs_offset,
        Direction::Down => abs_offset,
        Direction::Left => -abs_offset,
        Direction::Right => abs_offset,
    };

    graph.point_set.push((x1, y1));
    graph.point_set.push((x2, y2));
    graph.edge_set.push((vec![], vec![]));
    graph.edge_set.push((vec![], vec![]));
    graph.edge_costs.push((vec![], vec![]));
    graph.edge_costs.push((vec![], vec![]));

    let start_ind = graph.edge_set.len() - 2;
    let end_ind = graph.edge_set.len() - 1;
    for i in 0..graph.point_set.len() - 2 {
        let y1_prime = y1 + offset(start_dir, start_abs_offset);
        let x1_prime = x1 + offset(start_dir, start_abs_offset);
        if graph.point_set[i].0 == x1
            && ((graph.point_set[i].1 <= y1 && start_dir == Direction::Up)
                || (graph.point_set[i].1 >= y1 && start_dir == Direction::Down))
            && !aals_blocked_by_bb(end_el_bb, graph.point_set[i].1, y1, false, x1)
        {
            graph.edge_set[i].1.push(start_ind);
            graph.edge_set[start_ind].1.push(i);
            let mut cost = cost_function(&graph.point_set, line_struct, corner_cost, i, start_ind);
            if !((graph.point_set[i].1 <= y1_prime && start_dir == Direction::Up)
                || (graph.point_set[i].1 >= y1_prime && start_dir == Direction::Down))
            {
                cost += start_abs_offset as u32 * 2;
            }
            graph.edge_costs[i].1.push(cost);
            graph.edge_costs[start_ind].1.push(cost);
        }
        if graph.point_set[i].1 == y1
            && ((graph.point_set[i].0 >= x1 && start_dir == Direction::Right)
                || (graph.point_set[i].0 <= x1 && start_dir == Direction::Left))
            && !aals_blocked_by_bb(end_el_bb, graph.point_set[i].0, x1, true, y1)
        {
            graph.edge_set[i].0.push(start_ind);
            graph.edge_set[start_ind].0.push(i);
            let mut cost = cost_function(&graph.point_set, line_struct, corner_cost, i, start_ind);
            if !((graph.point_set[i].0 >= x1_prime && start_dir == Direction::Right)
                || (graph.point_set[i].0 <= x1_prime && start_dir == Direction::Left))
            {
                cost += start_abs_offset as u32 * 2;
            }
            graph.edge_costs[i].0.push(cost);
            graph.edge_costs[start_ind].0.push(cost);
        }

        let y2_prime = y2 + offset(end_dir, end_abs_offset);
        let x2_prime = x2 + offset(end_dir, end_abs_offset);
        if graph.point_set[i].0 == x2
            && ((graph.point_set[i].1 <= y2 && end_dir == Direction::Up)
                || (graph.point_set[i].1 >= y2 && end_dir == Direction::Down))
            && !aals_blocked_by_bb(start_el_bb, graph.point_set[i].1, y2, false, x2)
        {
            graph.edge_set[i].1.push(end_ind);
            graph.edge_set[end_ind].1.push(i);
            let mut cost = cost_function(&graph.point_set, line_struct, corner_cost, i, end_ind);
            if !((graph.point_set[i].1 <= y2_prime && end_dir == Direction::Up)
                || (graph.point_set[i].1 >= y2_prime && end_dir == Direction::Down))
            {
                cost += end_abs_offset as u32 * 2;
            }
            graph.edge_costs[i].1.push(cost);
            graph.edge_costs[end_ind].1.push(cost);
        }
        if graph.point_set[i].1 == y2
            && ((graph.point_set[i].0 >= x2 && end_dir == Direction::Right)
                || (graph.point_set[i].0 <= x2 && end_dir == Direction::Left))
            && !aals_blocked_by_bb(start_el_bb, graph.point_set[i].0, x2, true, y2)
        {
            graph.edge_set[i].0.push(end_ind);
            graph.edge_set[end_ind].0.push(i);
            let mut cost = cost_function(&graph.point_set, line_struct, corner_cost, i, end_ind);
            if !((graph.point_set[i].0 >= x2_prime && end_dir == Direction::Right)
                || (graph.point_set[i].0 <= x2_prime && end_dir == Direction::Left))
            {
                cost += end_abs_offset as u32 * 2;
            }
            graph.edge_costs[i].0.push(cost);
            graph.edge_costs[end_ind].0.push(cost);
        }
    }

    (start_ind, end_ind)
}

fn compute_costs(
    point_set: &[(f32, f32)],
    edge_set: &[(Vec<usize>, Vec<usize>)],
    line_struct: &LineStruct,
    corner_cost: u32,
) -> Vec<(Vec<u32>, Vec<u32>)> {
    let mut edge_costs = vec![(vec![], vec![]); edge_set.len()];
    for i in 0..edge_set.len() {
        for j in edge_set[i].0.iter() {
            let ind_1 = i;
            let ind_2 = *j;
            edge_costs[i].0.push(cost_function(
                point_set,
                line_struct,
                corner_cost,
                ind_1,
                ind_2,
            ));
        }

        for j in edge_set[i].1.iter() {
            let ind_1 = i;
            let ind_2 = *j;
            edge_costs[i].1.push(cost_function(
                point_set,
                line_struct,
                corner_cost,
                ind_1,
                ind_2,
            ));
        }
    }

    edge_costs
}

fn cost_function(
    point_set: &[(f32, f32)],
    line_struct: &LineStruct,
    corner_cost: u32,
    ind_1: usize,
    ind_2: usize,
) -> u32 {
    let mid_point_mul_x = if line_struct.mid_x != usize::MAX
        && point_set[ind_1].0 == line_struct.x_lines[line_struct.mid_x]
    {
        0.5
    } else {
        1.0
    };
    let mid_point_mul_y = if line_struct.mid_y != usize::MAX
        && point_set[ind_1].1 == line_struct.y_lines[line_struct.mid_y]
    {
        0.5
    } else {
        1.0
    };

    ((point_set[ind_1].0 - point_set[ind_2].0).abs() * mid_point_mul_y
        + (point_set[ind_1].1 - point_set[ind_2].1).abs() * mid_point_mul_x) as u32
        + corner_cost
}

type PrevPoint = Vec<(NodeState, NodeState)>;

fn dijkstra_get_dists(
    graph: &Graph,
    start_ind: usize,
    end_ind: usize,
    dirs: (Direction, Direction),
    corner_cost: u32,
) -> (Vec<(u32, u32)>, PrevPoint) {
    let (start_dir, end_dir) = dirs;
    let inf = u32::MAX;
    let mut dists = vec![(inf, inf); graph.point_set.len()];
    let mut prev_point = vec![
        (
            NodeState {
                ind: graph.point_set.len() - 1,
                dir: Direction::Left
            },
            NodeState {
                ind: graph.point_set.len() - 1,
                dir: Direction::Left
            }
        );
        graph.point_set.len()
    ];

    let mut queue: BinaryHeap<PathCost> = BinaryHeap::new();
    if start_dir == Direction::Left || start_dir == Direction::Right {
        dists[start_ind].0 = 0;
    } else {
        dists[start_ind].1 = 0;
    }
    queue.push(PathCost {
        cost: 0,
        idx: start_ind,
        dir: start_dir,
        from: start_ind,
        from_dir: start_dir,
    });

    let simplified_end_dir = match end_dir {
        Direction::Left | Direction::Right => Direction::Left,
        Direction::Down | Direction::Up => Direction::Down,
    };

    while let Some(next) = queue.pop() {
        if next.idx == end_ind && simplified_end_dir == next.dir {
            match next.dir {
                Direction::Left | Direction::Right => {
                    prev_point[next.idx].0 = NodeState {
                        ind: next.from,
                        dir: next.from_dir,
                    }
                }
                Direction::Down | Direction::Up => {
                    prev_point[next.idx].1 = NodeState {
                        ind: next.from,
                        dir: next.from_dir,
                    }
                }
            };
            break;
        }
        let dist = match next.dir {
            Direction::Left | Direction::Right => dists[next.idx].0,
            Direction::Down | Direction::Up => dists[next.idx].1,
        };

        if next.cost > dist {
            continue;
        }

        match next.dir {
            Direction::Left | Direction::Right => {
                prev_point[next.idx].0 = NodeState {
                    ind: next.from,
                    dir: next.from_dir,
                }
            }
            Direction::Down | Direction::Up => {
                prev_point[next.idx].1 = NodeState {
                    ind: next.from,
                    dir: next.from_dir,
                }
            }
        };

        for i in 0..graph.edge_set[next.idx].0.len() {
            let mut edge_cost = graph.edge_costs[next.idx].0[i];
            if next.dir == Direction::Left || next.dir == Direction::Right {
                edge_cost += corner_cost * 2;
            }
            let other_ind = graph.edge_set[next.idx].0[i];
            if dist + edge_cost < dists[other_ind].0 {
                dists[other_ind].0 = dist + edge_cost;
                queue.push(PathCost {
                    cost: dists[other_ind].0,
                    idx: other_ind,
                    dir: Direction::Left,
                    from: next.idx,
                    from_dir: next.dir,
                });
            }
        }

        for i in 0..graph.edge_set[next.idx].1.len() {
            let mut edge_cost = graph.edge_costs[next.idx].1[i];
            if next.dir == Direction::Down || next.dir == Direction::Up {
                edge_cost += corner_cost * 2;
            }
            let other_ind = graph.edge_set[next.idx].1[i];
            if dist + edge_cost < dists[graph.edge_set[next.idx].1[i]].1 {
                dists[other_ind].1 = dist + edge_cost;
                queue.push(PathCost {
                    cost: dists[other_ind].1,
                    idx: graph.edge_set[next.idx].1[i],
                    dir: Direction::Down,
                    from: next.idx,
                    from_dir: next.dir,
                });
            }
        }
    }

    (dists, prev_point)
}

fn dijkstra_get_points(
    point_set: &[(f32, f32)],
    prev_point: &[(NodeState, NodeState)],
    start_ind: usize,
    end_ind: usize,
    end_dir: Direction,
) -> Vec<(f32, f32)> {
    let mut back_points_inds = vec![end_ind];
    let mut loc = end_ind;
    let mut dir = end_dir;
    for _ in 0..10 {
        if loc == start_ind {
            break;
        }
        let new_loc = match dir {
            Direction::Left | Direction::Right => {
                dir = prev_point[loc].0.dir;
                prev_point[loc].0.ind
            }
            Direction::Down | Direction::Up => {
                dir = prev_point[loc].1.dir;
                prev_point[loc].1.ind
            }
        };
        if new_loc == loc {
            break;
        }
        loc = new_loc;
        back_points_inds.push(loc);
    }
    if back_points_inds.len() == 1 {
        back_points_inds.push(start_ind);
    }

    let mut points = vec![];
    for i in (0..back_points_inds.len()).rev() {
        points.push(point_set[back_points_inds[i]]);
    }
    points
}

pub fn render_match_corner(
    connector: &ElbowConnector,
    mut ratio_offset: f32,
    start_abs_offset: f32,
    end_abs_offset: f32,
    start_el_bb: BoundingBox,
    end_el_bb: BoundingBox,
    abs_offset_set: bool,
) -> Result<Vec<(f32, f32)>> {
    let (x1, y1) = connector.start.origin;
    let (x2, y2) = connector.end.origin;

    let points: Vec<(f32, f32)>;
    if let (Some(start_dir_some), Some(end_dir_some)) = (connector.start.dir, connector.end.dir) {
        ratio_offset = ratio_offset.clamp(0.0, 1.0);

        let line_struct = get_lines(
            connector,
            ratio_offset,
            (start_abs_offset, end_abs_offset),
            (start_el_bb, end_el_bb),
            abs_offset_set,
            (start_dir_some, end_dir_some),
        );
        let mut point_set = vec![];

        for x in &line_struct.x_lines {
            for y in &line_struct.y_lines {
                point_set.push((*x, *y));
            }
        }

        let edge_set = get_edges(&point_set, start_el_bb, end_el_bb);

        let total_bb = start_el_bb.combine(&end_el_bb);
        let total_bb_size =
            (total_bb.width() + total_bb.height() + start_abs_offset + end_abs_offset) as u32;
        let corner_cost = 1 + total_bb_size;

        let edge_costs = compute_costs(&point_set, &edge_set, &line_struct, corner_cost);

        let mut graph = Graph {
            point_set,
            edge_set,
            edge_costs,
        };

        let (start_ind, end_ind) = add_start_and_end(
            connector,
            &mut graph,
            (start_abs_offset, end_abs_offset),
            (start_el_bb, end_el_bb),
            (start_dir_some, end_dir_some),
            &line_struct,
            corner_cost,
        );

        let (_, prev_point) = dijkstra_get_dists(
            &graph,
            start_ind,
            end_ind,
            (start_dir_some, end_dir_some),
            corner_cost,
        );

        points = dijkstra_get_points(
            &graph.point_set,
            &prev_point,
            start_ind,
            end_ind,
            end_dir_some,
        );
    } else {
        points = vec![(x1, y1), (x2, y2)];
    }

    Ok(points)
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
