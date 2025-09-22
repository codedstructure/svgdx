use crate::elements::connector::Connector;
use crate::errors::Result;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::{elements::connector::Direction, geometry::BoundingBox};

// from the example heap docs https://doc.rust-lang.org/std/collections/binary_heap/index.html
#[derive(PartialEq, Eq)]
struct PathCost {
    cost: u32,
    idx: usize,
}

impl Ord for PathCost {
    fn cmp(&self, other: &Self) -> Ordering {
        // Notice that we flip the ordering on costs.
        // In case of a tie we compare positions - this step is necessary
        // to make implementations of `PartialEq` and `Ord` consistent.
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

// checks if there a axis aligned line segment is intersected by a bounding box
// this allows the aals to be entirely inside
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

    true
}

fn get_lines(
    connector: &Connector,
    ratio_offset: f32,
    abs_offsets: (f32, f32),
    bbs: (BoundingBox, BoundingBox),
    abs_offset_set: bool,
    dirs: (Direction, Direction),
) -> (Vec<f32>, Vec<f32>, usize, usize) {
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
        // there is a gap
        x_lines.push((start_el_bb.x1 + end_el_bb.x2) * 0.5);
        mid_x = x_lines.len() - 1;
    } else if start_el_bb.x2 < end_el_bb.x1 {
        // there is a gap
        x_lines.push((start_el_bb.x2 + end_el_bb.x1) * 0.5);
        mid_x = x_lines.len() - 1;
    }

    y_lines.push(start_el_bb.y1 - start_abs_offset);
    y_lines.push(start_el_bb.y2 + start_abs_offset);
    y_lines.push(end_el_bb.y1 - end_abs_offset);
    y_lines.push(end_el_bb.y2 + end_abs_offset);

    if start_el_bb.y1 > end_el_bb.y2 {
        // there is a gap
        y_lines.push(start_el_bb.y1 * (1.0 - ratio_offset) + end_el_bb.y2 * ratio_offset);
        mid_y = y_lines.len() - 1;
    } else if start_el_bb.y2 < end_el_bb.y1 {
        // there is a gap
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
            Direction::Down => mid_y = 1,  // positive y
            Direction::Left => mid_x = 0,  // negative x
            Direction::Right => mid_x = 1, // positive x
            Direction::Up => mid_y = 0,    // negative y
        }
    }

    (x_lines, y_lines, mid_x, mid_y)
}

fn get_edges(
    point_set: &[(f32, f32)],
    start_el_bb: BoundingBox,
    end_el_bb: BoundingBox,
) -> Vec<Vec<usize>> {
    let mut edge_set = vec![vec![]; point_set.len()];

    for i in 0..point_set.len() {
        for j in 0..point_set.len() {
            if i == j {
                continue;
            }
            let mut connected = false;

            // check if not blocked by a wall
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
                connected = true;
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
                connected = true;
            }
            if connected {
                edge_set[i].push(j);
                edge_set[j].push(i);
            }
        }
    }

    edge_set
}

fn add_start_and_end(
    connector: &Connector,
    point_set: &mut Vec<(f32, f32)>,
    edge_set: &mut Vec<Vec<usize>>,
    start_el_bb: BoundingBox,
    end_el_bb: BoundingBox,
    start_dir: Direction,
    end_dir: Direction,
) -> (usize, usize) {
    let (x1, y1) = connector.start.origin;
    let (x2, y2) = connector.end.origin;

    point_set.push((x1, y1)); // start
    point_set.push((x2, y2)); // end
    edge_set.push(vec![]);
    edge_set.push(vec![]);

    let start_ind = edge_set.len() - 2;
    let end_ind = edge_set.len() - 1;
    for i in 0..point_set.len() - 2 {
        if point_set[i].0 == x1
            && ((point_set[i].1 < y1 && start_dir == Direction::Up)
                || (point_set[i].1 > y1 && start_dir == Direction::Down))
            && !aals_blocked_by_bb(end_el_bb, point_set[i].1, y1, false, x1)
        {
            edge_set[i].push(start_ind);
            edge_set[start_ind].push(i);
        }
        if point_set[i].1 == y1
            && ((point_set[i].0 > x1 && start_dir == Direction::Right)
                || (point_set[i].0 < x1 && start_dir == Direction::Left))
            && !aals_blocked_by_bb(end_el_bb, point_set[i].0, x1, true, y1)
        {
            edge_set[i].push(start_ind);
            edge_set[start_ind].push(i);
        }

        if point_set[i].0 == x2
            && ((point_set[i].1 < y2 && end_dir == Direction::Up)
                || (point_set[i].1 > y2 && end_dir == Direction::Down))
            && !aals_blocked_by_bb(start_el_bb, point_set[i].1, y2, false, x2)
        {
            edge_set[i].push(end_ind);
            edge_set[end_ind].push(i);
        }
        if point_set[i].1 == y2
            && ((point_set[i].0 > x2 && end_dir == Direction::Right)
                || (point_set[i].0 < x2 && end_dir == Direction::Left))
            && !aals_blocked_by_bb(start_el_bb, point_set[i].0, x2, true, y2)
        {
            edge_set[i].push(end_ind);
            edge_set[end_ind].push(i);
        }
    }

    (start_ind, end_ind)
}

fn cost_function(
    point_set: &[(f32, f32)],
    edge_set: &[Vec<usize>],
    x_lines: Vec<f32>,
    y_lines: Vec<f32>,
    mid_x: usize,
    mid_y: usize,
    total_bb_size: u32,
) -> Vec<Vec<u32>> {
    // edge cost function

    // needs to be comparable to or larger than total_bb_size
    let corner_cost = 1 + total_bb_size;
    let mut edge_costs = vec![vec![]; edge_set.len()];
    for i in 0..edge_set.len() {
        for j in 0..edge_set[i].len() {
            let ind_1 = i;
            let ind_2 = edge_set[i][j];

            let mid_point_mul_x = if mid_x != usize::MAX && point_set[ind_1].0 == x_lines[mid_x] {
                0.5
            } else {
                1.0
            };
            let mid_point_mul_y = if mid_y != usize::MAX && point_set[ind_1].1 == y_lines[mid_y] {
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

    edge_costs
}

fn dijkstra_get_dists(
    point_set: &[(f32, f32)],
    edge_set: &[Vec<usize>],
    edge_costs: &[Vec<u32>],
    start_ind: usize,
    end_ind: usize,
) -> Vec<u32> {
    // just needs to be bigger than 5* (corner cost  +  total bounding box size)
    let inf = u32::MAX;
    let mut dist = vec![inf; point_set.len()];

    let mut queue: BinaryHeap<PathCost> = BinaryHeap::new();
    dist[start_ind] = 0;
    queue.push(PathCost {
        cost: 0,
        idx: start_ind,
    });

    // cant get stuck in a loop as cost for a distance either decreases or queue shrinks
    while let Some(next) = queue.pop() {
        if next.idx == end_ind {
            break;
        }

        // the node is reached by faster means so already popped
        if next.cost > dist[next.idx] {
            continue;
        }

        for i in 0..edge_set[next.idx].len() {
            let edge_cost = edge_costs[next.idx][i];
            if dist[next.idx] + edge_cost < dist[edge_set[next.idx][i]] {
                dist[edge_set[next.idx][i]] = dist[next.idx] + edge_cost;
                queue.push(PathCost {
                    cost: dist[edge_set[next.idx][i]],
                    idx: edge_set[next.idx][i],
                });
            }
        }
    }

    dist
}

fn dijkstra_get_points(
    dist: Vec<u32>,
    point_set: &[(f32, f32)],
    edge_set: &[Vec<usize>],
    edge_costs: &[Vec<u32>],
    start_ind: usize,
    end_ind: usize,
) -> Vec<(f32, f32)> {
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

    let mut points = vec![];
    for i in (0..back_points_inds.len()).rev() {
        points.push(point_set[back_points_inds[i]]);
    }
    points
}

pub fn render_match_corner(
    connector: &Connector,
    ratio_offset: f32,
    start_abs_offset: f32,
    end_abs_offset: f32,
    start_el_bb: BoundingBox,
    end_el_bb: BoundingBox,
    abs_offset_set: bool,
) -> Result<Vec<(f32, f32)>> {
    let (x1, y1) = connector.start.origin;
    let (x2, y2) = connector.end.origin;

    // method generates all points it could possibly want to go through then does dijkstras on it

    let points: Vec<(f32, f32)>;
    if let (Some(start_dir_some), Some(end_dir_some)) = (connector.start.dir, connector.end.dir) {
        // x_lines have constant x vary over y
        let (x_lines, y_lines, mid_x, mid_y) = get_lines(
            connector,
            ratio_offset,
            (start_abs_offset, end_abs_offset),
            (start_el_bb, end_el_bb),
            abs_offset_set,
            (start_dir_some, end_dir_some),
        );
        let mut point_set = vec![];

        for x in &x_lines {
            for y in &y_lines {
                point_set.push((*x, *y));
            }
        }

        let mut edge_set = get_edges(&point_set, start_el_bb, end_el_bb);
        let (start_ind, end_ind) = add_start_and_end(
            connector,
            &mut point_set,
            &mut edge_set,
            start_el_bb,
            end_el_bb,
            start_dir_some,
            end_dir_some,
        );

        let total_bb = start_el_bb.combine(&end_el_bb);
        let total_bb_size =
            (total_bb.width() + total_bb.height() + start_abs_offset + end_abs_offset) as u32;

        let edge_costs = cost_function(
            &point_set,
            &edge_set,
            x_lines,
            y_lines,
            mid_x,
            mid_y,
            total_bb_size,
        );

        let dist = dijkstra_get_dists(&point_set, &edge_set, &edge_costs, start_ind, end_ind);

        points = dijkstra_get_points(dist, &point_set, &edge_set, &edge_costs, start_ind, end_ind);
    } else {
        points = vec![(x1, y1), (x2, y2)];
    }

    Ok(points)
}
