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
    dir: Direction,
    from: usize,
    from_dir: Direction,
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
    println!("abs offsets {:?} {:?}",start_abs_offset, end_abs_offset);

    x_lines.push(start_el_bb.x1 - start_abs_offset);
    x_lines.push(start_el_bb.x2 + start_abs_offset);
    x_lines.push(end_el_bb.x1 - end_abs_offset);
    x_lines.push(end_el_bb.x2 + end_abs_offset);

    if start_el_bb.x1 > end_el_bb.x2 {
        // there is a gap
        x_lines.push(start_el_bb.x1 * (1.0 - ratio_offset) + end_el_bb.x2 * ratio_offset);
        mid_x = x_lines.len() - 1;
    } else if start_el_bb.x2 < end_el_bb.x1 {
        // there is a gap
        x_lines.push(start_el_bb.x2 * (1.0 - ratio_offset) + end_el_bb.x1 * ratio_offset);
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
) -> Vec<(Vec<usize>, Vec<usize>)> {
    let mut edge_set = vec![(vec![], vec![]); point_set.len()];

    for i in 0..point_set.len() {
        for j in 0..point_set.len() {
            if i == j {
                continue;
            }

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

fn add_start_and_end(
    connector: &Connector,
    point_set: &mut Vec<(f32, f32)>,
    edge_set: &mut Vec<(Vec<usize>, Vec<usize>)>,
    edge_costs: &mut Vec<(Vec<u32>, Vec<u32>)>,
    abs_offsets: (f32, f32),
    start_el_bb: BoundingBox,
    end_el_bb: BoundingBox,
    start_dir: Direction,
    end_dir: Direction,
    x_lines: &Vec<f32>,
    y_lines: &Vec<f32>,
    mid_x: usize,
    mid_y: usize,
    corner_cost: u32,
) -> (usize, usize) {
    let (x1, y1) = connector.start.origin;
    let (x2, y2) = connector.end.origin;
    let (start_abs_offset, end_abs_offset) = abs_offsets;

    let offset = |dir: Direction, abs_offset: f32| match dir {
        Direction::Up => -abs_offset,
        Direction::Down => abs_offset,
        Direction::Left => -abs_offset,
        Direction::Right => abs_offset,
    };

    point_set.push((x1, y1)); // start
    point_set.push((x2, y2)); // end
    edge_set.push((vec![], vec![]));
    edge_set.push((vec![], vec![]));
    edge_costs.push((vec![], vec![]));
    edge_costs.push((vec![], vec![]));

    let start_ind = edge_set.len() - 2;
    let end_ind = edge_set.len() - 1;
    for i in 0..point_set.len() - 2 {
        let y1_prime = y1 + offset(start_dir, start_abs_offset);
        let x1_prime = x1 + offset(start_dir, start_abs_offset);
        if point_set[i].0 == x1
            && ((point_set[i].1 <= y1 && start_dir == Direction::Up)
                || (point_set[i].1 >= y1 && start_dir == Direction::Down))
            && !aals_blocked_by_bb(end_el_bb, point_set[i].1, y1, false, x1)
        {
            edge_set[i].1.push(start_ind);
            edge_set[start_ind].1.push(i);
            let mut cost = cost_function(point_set, x_lines, y_lines, mid_x, mid_y, corner_cost, i, start_ind);
            if !((point_set[i].1 <= y1_prime && start_dir == Direction::Up)
                || (point_set[i].1 >= y1_prime && start_dir == Direction::Down))
            {
                cost += start_abs_offset as u32 *2;
            }
            edge_costs[i].1.push(cost);
            edge_costs[start_ind].1.push(cost);
        }
        if point_set[i].1 == y1
            && ((point_set[i].0 >= x1 && start_dir == Direction::Right)
                || (point_set[i].0 <= x1 && start_dir == Direction::Left))
            && !aals_blocked_by_bb(end_el_bb, point_set[i].0, x1, true, y1)
        {
            edge_set[i].0.push(start_ind);
            edge_set[start_ind].0.push(i);
            let mut cost = cost_function(point_set, x_lines, y_lines, mid_x, mid_y, corner_cost, i, start_ind);
            if !((point_set[i].0 >= x1_prime && start_dir == Direction::Right)
                || (point_set[i].0 <= x1_prime && start_dir == Direction::Left))
            {
                cost += start_abs_offset as u32 *2;
            }
            edge_costs[i].0.push(cost);
            edge_costs[start_ind].0.push(cost);
        }

        let y2_prime = y2 + offset(end_dir, end_abs_offset);
        let x2_prime = x2 + offset(end_dir, end_abs_offset);
        if point_set[i].0 == x2
            && ((point_set[i].1 <= y2 && end_dir == Direction::Up)
                || (point_set[i].1 >= y2 && end_dir == Direction::Down))
            && !aals_blocked_by_bb(start_el_bb, point_set[i].1, y2, false, x2)
        {
            edge_set[i].1.push(end_ind);
            edge_set[end_ind].1.push(i);
            let mut cost = cost_function(point_set, x_lines, y_lines, mid_x, mid_y, corner_cost, i, end_ind);
            if !((point_set[i].1 <= y2_prime && end_dir == Direction::Up)
                || (point_set[i].1 >= y2_prime && end_dir == Direction::Down))
            {
                cost += end_abs_offset as u32 *2;
            }
            edge_costs[i].1.push(cost);
            edge_costs[end_ind].1.push(cost);
        }
        if point_set[i].1 == y2
            && ((point_set[i].0 >= x2 && end_dir == Direction::Right)
                || (point_set[i].0 <= x2 && end_dir == Direction::Left))
            && !aals_blocked_by_bb(start_el_bb, point_set[i].0, x2, true, y2)
        {
            edge_set[i].0.push(end_ind);
            edge_set[end_ind].0.push(i);
            let mut cost = cost_function(point_set, x_lines, y_lines, mid_x, mid_y, corner_cost, i, end_ind);
            if !((point_set[i].0 >= x2_prime && end_dir == Direction::Right)
                || (point_set[i].0 <= x2_prime && end_dir == Direction::Left))
            {
                cost += end_abs_offset as u32 *2;
            }
            edge_costs[i].0.push(cost);
            edge_costs[end_ind].0.push(cost);
        }
    }

    println!("{:?} {:?}",edge_costs[start_ind], edge_costs[end_ind]);
    println!("{:?} {:?}",edge_set[start_ind], edge_set[end_ind]);
    for i in edge_set[start_ind].0.iter() {
        println!("{} {:?}",*i, point_set[*i]);
    }
    for i in edge_set[end_ind].0.iter() {
        println!("{} {:?}",*i, point_set[*i]);
    }

    println!("{:?} {:?}",x_lines, y_lines);
    println!("{:?} {:?}", point_set[start_ind], point_set[end_ind]);

    (start_ind, end_ind)
}

fn compute_costs(
    point_set: &[(f32, f32)],
    edge_set: &[(Vec<usize>, Vec<usize>)],
    x_lines: &Vec<f32>,
    y_lines: &Vec<f32>,
    mid_x: usize,
    mid_y: usize,
    corner_cost: u32,
) -> Vec<(Vec<u32>, Vec<u32>)> {
    // edge cost function

    println!("corner cost {}", corner_cost);
    let mut edge_costs = vec![(vec![], vec![]); edge_set.len()];
    for i in 0..edge_set.len() {
        // x moving edge
        for j in edge_set[i].0.iter() {
            let ind_1 = i;
            let ind_2 = *j;

            edge_costs[i].0.push(cost_function(point_set, &x_lines, &y_lines, mid_x, mid_y, corner_cost, ind_1, ind_2));
        }

        // y moving edge
        for j in edge_set[i].1.iter() {
            let ind_1 = i;
            let ind_2 = *j;

            edge_costs[i].1.push(cost_function(point_set, &x_lines, &y_lines, mid_x, mid_y, corner_cost, ind_1, ind_2));
        }
    }

    edge_costs
}

fn cost_function(
    point_set: &[(f32, f32)],
    x_lines: &Vec<f32>,
    y_lines: &Vec<f32>,
    mid_x: usize,
    mid_y: usize,
    corner_cost: u32,
    ind_1: usize,
    ind_2: usize,
) -> u32 {
    // swapping order of ind_1 and ind_2 does nothing

    // ind_1 uses in mid point calcs could use ind_2 in same place no diff
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

    return ((point_set[ind_1].0 - point_set[ind_2].0).abs() * mid_point_mul_y
    + (point_set[ind_1].1 - point_set[ind_2].1).abs() * mid_point_mul_x)
    as u32 + corner_cost;// round may cause some problems
}

fn dijkstra_get_dists(
    point_set: &[(f32, f32)],
    edge_set: &[(Vec<usize>, Vec<usize>)],
    edge_costs: &[(Vec<u32>, Vec<u32>)],
    start_ind: usize,
    end_ind: usize,
    start_dir: Direction,
    end_dir: Direction,
    corner_cost: u32,
) -> (
    Vec<(u32, u32)>,
    Vec<((usize, Direction), (usize, Direction))>,
) {
    // just needs to be bigger than 5* (corner cost  +  total bounding box size)
    let inf = u32::MAX;
    let mut dists = vec![(inf, inf); point_set.len()];
    let mut prev_point = vec![
        (
            (point_set.len() - 1, Direction::Left),
            (point_set.len() - 1, Direction::Left)
        );
        point_set.len()
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

    // cant get stuck in a loop as cost for a distance either decreases or queue shrinks
    while let Some(next) = queue.pop() {
        if next.idx == end_ind && simplified_end_dir == next.dir {
            match next.dir {
                Direction::Left | Direction::Right => {
                    prev_point[next.idx].0 = (next.from, next.from_dir)
                }
                Direction::Down | Direction::Up => {
                    prev_point[next.idx].1 = (next.from, next.from_dir)
                }
            };
            break;
        }
        let dist = match next.dir {
            Direction::Left | Direction::Right => dists[next.idx].0,
            Direction::Down | Direction::Up => dists[next.idx].1,
        };

        // the node is reached by faster means so already popped
        if next.cost > dist {
            continue;
        }

        match next.dir {
            Direction::Left | Direction::Right => {
                prev_point[next.idx].0 = (next.from, next.from_dir)
            }
            Direction::Down | Direction::Up => prev_point[next.idx].1 = (next.from, next.from_dir),
        };

        // x moving edge
        for i in 0..edge_set[next.idx].0.len() {
            let mut edge_cost = edge_costs[next.idx].0[i];
            // if same direction then add 2 corners
            if next.dir == Direction::Left || next.dir == Direction::Right {
                edge_cost += corner_cost*2;
            }
            let other_ind = edge_set[next.idx].0[i];
            if dist + edge_cost < dists[other_ind].0 {
                println!("improvement {}, old: {}, new {}", other_ind, dists[other_ind].0, dist + edge_cost);
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

        // y moving edge
        for i in 0..edge_set[next.idx].1.len() {
            let mut edge_cost = edge_costs[next.idx].1[i];
            if next.dir == Direction::Down || next.dir == Direction::Up {
                edge_cost += corner_cost*2;
            }
            let other_ind = edge_set[next.idx].1[i];
            if dist + edge_cost < dists[edge_set[next.idx].1[i]].1 {
                println!("improvement {}, old: {}, new {}", other_ind, dists[other_ind].1, dist + edge_cost);
                dists[other_ind].1 = dist + edge_cost;
                queue.push(PathCost {
                    cost: dists[other_ind].1,
                    idx: edge_set[next.idx].1[i],
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
    prev_point: &[((usize, Direction), (usize, Direction))],
    start_ind: usize,
    end_ind: usize,
    end_dir: Direction,
) -> Vec<(f32, f32)> {
    let mut back_points_inds = vec![end_ind];
    let mut loc = end_ind;
    let mut dir = end_dir;
    // wont use more than 10 corners garentee no loops
    for _ in 0..10 {
        if loc == start_ind {
            break;
        }
        let new_loc;
        match dir {
            Direction::Left | Direction::Right => (new_loc, dir) = prev_point[loc].0,
            Direction::Down | Direction::Up => (new_loc, dir) = prev_point[loc].1,
        };
        if new_loc == loc {
            break;
        }
        loc = new_loc;
        back_points_inds.push(loc);
    }
    // if no path just do line
    if back_points_inds.len() == 1{
        back_points_inds.push(start_ind);
    }

    let mut points = vec![];
    for i in (0..back_points_inds.len()).rev() {
        points.push(point_set[back_points_inds[i]]);
    }
    points
}

pub fn render_match_corner(
    connector: &Connector,
    mut ratio_offset: f32,
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
        // clamp ratio between 0.0 and 1.0
        ratio_offset = ratio_offset.clamp(0.0, 1.0);

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


        let total_bb = start_el_bb.combine(&end_el_bb);
        let total_bb_size =
            (total_bb.width() + total_bb.height() + start_abs_offset + end_abs_offset) as u32;
        // needs to be comparable to or larger than total_bb_size
        let corner_cost = 1 + total_bb_size;

        let mut edge_costs = compute_costs(
            &point_set,
            &edge_set,
            &x_lines,
            &y_lines,
            mid_x,
            mid_y,
            corner_cost,
        );

        let (start_ind, end_ind) = add_start_and_end(
            connector,
            &mut point_set,
            &mut edge_set,
            &mut edge_costs,
            (start_abs_offset, end_abs_offset),
            start_el_bb,
            end_el_bb,
            start_dir_some,
            end_dir_some,
            &x_lines,
            &y_lines,
            mid_x,
            mid_y,
            corner_cost,
        );

        let (_, prev_point) = dijkstra_get_dists(
            &point_set,
            &edge_set,
            &edge_costs,
            start_ind,
            end_ind,
            start_dir_some,
            end_dir_some,
            corner_cost,
        );

        points = dijkstra_get_points(&point_set, &prev_point, start_ind, end_ind, end_dir_some);
    } else {
        points = vec![(x1, y1), (x2, y2)];
    }

    Ok(points)
}
