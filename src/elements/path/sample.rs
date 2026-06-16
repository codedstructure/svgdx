//! Functions for sampling the length of curved path segments
//! and find point at a ratio along the segment.

use super::Vec2;

const EPSILON: f32 = 1e-6;

pub(super) fn sample_length(samples: usize, mut evaluate: impl FnMut(f32) -> Vec2) -> f32 {
    let samples = samples.max(1);
    let mut prev = evaluate(0.0);
    let mut total = 0.0;

    for index in 1..=samples {
        let t = index as f32 / samples as f32;
        let curr = evaluate(t);
        total += curr.distance(prev);
        prev = curr;
    }

    total
}

pub(super) fn sample_point_at_ratio(
    samples: usize,
    ratio: f32,
    mut evaluate: impl FnMut(f32) -> Vec2,
) -> Vec2 {
    let samples = samples.max(1);
    let ratio = ratio.clamp(0.0, 1.0);
    let mut points = Vec::with_capacity(samples + 1);

    for index in 0..=samples {
        let t = index as f32 / samples as f32;
        points.push((t, evaluate(t)));
    }

    let mut total = 0.0;
    let mut segments = Vec::with_capacity(samples);
    for pair in points.windows(2) {
        let length = pair[0].1.distance(pair[1].1);
        segments.push(length);
        total += length;
    }

    if total <= EPSILON {
        return points[0].1;
    }

    let target = total * ratio;
    let mut elapsed = 0.0;

    for (index, length) in segments.into_iter().enumerate() {
        if elapsed + length >= target {
            if length <= EPSILON {
                return points[index].1;
            }
            let local_ratio = (target - elapsed) / length;
            let start = points[index].1;
            let end = points[index + 1].1;
            return start + local_ratio * (end - start);
        }
        elapsed += length;
    }

    points.last().expect("points should not be empty").1
}
