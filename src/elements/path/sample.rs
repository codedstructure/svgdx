//! Functions for sampling the length of curved path segments
//! and find point at a ratio along the segment.
const EPSILON: f32 = 1e-6;

fn point_distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    (b.0 - a.0).hypot(b.1 - a.1)
}

pub(super) fn sample_length(samples: usize, mut evaluate: impl FnMut(f32) -> (f32, f32)) -> f32 {
    let samples = samples.max(1);
    let mut prev = evaluate(0.0);
    let mut total = 0.0;

    for index in 1..=samples {
        let t = index as f32 / samples as f32;
        let curr = evaluate(t);
        total += point_distance(prev, curr);
        prev = curr;
    }

    total
}

pub(super) fn sample_point_at_ratio(
    samples: usize,
    ratio: f32,
    mut evaluate: impl FnMut(f32) -> (f32, f32),
) -> (f32, f32) {
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
        let length = point_distance(pair[0].1, pair[1].1);
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
            return (
                start.0 + (end.0 - start.0) * local_ratio,
                start.1 + (end.1 - start.1) * local_ratio,
            );
        }
        elapsed += length;
    }

    points.last().expect("points should not be empty").1
}
