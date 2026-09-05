use crate::errors::{Error, Result};
use crate::geometry::{Length, strp_length};
use crate::types::attr_split_cycle;

const GAP_EPSILON: f32 = 1e-6;

type PointList = [(f32, f32)];

#[derive(Clone, Debug)]
pub(super) struct GapSpec {
    pub start: Length,
    pub end: Length,
}

impl std::str::FromStr for GapSpec {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut parts = attr_split_cycle(s);
        let start = parts
            .next()
            .as_deref()
            .map(strp_length)
            .transpose()?
            .ok_or_else(|| Error::InvalidValue("start gap".to_string(), s.to_string()))?;
        let end = parts
            .next()
            .as_deref()
            .map(strp_length)
            .transpose()?
            .ok_or_else(|| Error::InvalidValue("end gap".to_string(), s.to_string()))?;
        Ok(GapSpec { start, end })
    }
}

impl GapSpec {
    pub fn to_absolute(&self, total_len: f32) -> (f32, f32) {
        if total_len <= GAP_EPSILON {
            return (0.0, 0.0);
        }

        let start_dist = self.start.evaluate(total_len).clamp(0.0, total_len);
        let end_dist = (total_len - self.end.evaluate(total_len)).clamp(0.0, total_len);
        if start_dist <= end_dist {
            (start_dist, end_dist)
        } else {
            let mid = 0.5 * (start_dist + end_dist);
            (mid, mid)
        }
    }
}

fn polyline_length(points: &PointList) -> f32 {
    points
        .windows(2)
        .map(|pair| (pair[1].0 - pair[0].0).hypot(pair[1].1 - pair[0].1))
        .sum()
}

/// saturating point at distance along polyline
///
/// if distance < 0, returns the first point;
/// if distance > total length, returns the last point
// TODO: would be nice to unify with get_point_along_(poly)line, but that has
// different handling for negative distances: extrapolates the first segment
// prior to the start of the polyline. Should consider whether that is what we
// want, and make consistent. Also consider EdgeSpec / Length::calc_offset().
fn point_at_distance(points: &PointList, distance: f32) -> (f32, f32) {
    if points.is_empty() {
        return (0., 0.);
    }
    if points.len() == 1 {
        return points[0];
    }
    if distance <= 0. {
        return points[0];
    }

    let mut cumulative = 0.0;
    for pair in points.windows(2) {
        let (sx, sy) = pair[0];
        let (ex, ey) = pair[1];
        let seg_len = (ex - sx).hypot(ey - sy);
        if seg_len <= GAP_EPSILON {
            continue;
        }

        let next = cumulative + seg_len;
        if next >= distance {
            let ratio = (distance - cumulative) / seg_len;
            return (
                sx * (1.0 - ratio) + ex * ratio,
                sy * (1.0 - ratio) + ey * ratio,
            );
        }
        cumulative = next;
    }

    *points.last().expect("points checked non-empty")
}

/// push point to vec iff it is distinct(ish) from previous point
fn push_distinct(points: &mut Vec<(f32, f32)>, point: (f32, f32)) {
    if let Some((last_x, last_y)) = points.last().copied()
        && (last_x - point.0).abs() <= GAP_EPSILON
        && (last_y - point.1).abs() <= GAP_EPSILON
    {
        return;
    }
    points.push(point);
}

/// Generate a list of points along a polyline, excluding start and end of
/// given gap lengths
// TODO: consider negative Lengths analogous to Length::calc_offset or
// behaviour of line_offset::get_point_along_(poly)line
pub(super) fn points_with_gap(points: &PointList, gap: &GapSpec) -> Vec<(f32, f32)> {
    let total_len = polyline_length(points);
    let (start_dist, end_dist) = gap.to_absolute(total_len);

    if points.is_empty() {
        return vec![];
    }
    if points.len() == 1 {
        return vec![points[0]];
    }

    if (start_dist - end_dist).abs() <= GAP_EPSILON {
        return vec![point_at_distance(points, start_dist)];
    }

    let start_point = point_at_distance(points, start_dist);
    let end_point = point_at_distance(points, end_dist);

    let mut trimmed = Vec::new();
    push_distinct(&mut trimmed, start_point);

    let mut cumulative = 0.0;
    for pair in points.windows(2) {
        let segment_len = (pair[1].0 - pair[0].0).hypot(pair[1].1 - pair[0].1);
        cumulative += segment_len;
        if cumulative > start_dist + GAP_EPSILON && cumulative < end_dist - GAP_EPSILON {
            push_distinct(&mut trimmed, pair[1]);
        }
    }

    push_distinct(&mut trimmed, end_point);
    trimmed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_gap_line() {
        // simple point-to-point line
        let points = vec![(10.0, 5.0), (20.0, 5.0)];
        let res = points_with_gap(
            &points,
            &GapSpec {
                start: Length::Absolute(2.0),
                end: Length::Absolute(1.0),
            },
        );
        assert_eq!(res, vec![(12.0, 5.0), (19.0, 5.0)]);

        let res = points_with_gap(
            &points,
            &GapSpec {
                start: Length::Ratio(0.4),
                end: Length::Ratio(0.3),
            },
        );
        assert_eq!(res, vec![(14.0, 5.0), (17.0, 5.0)]);
    }

    #[test]
    fn test_apply_gap_polyline() {
        // more complex polyline
        let points = vec![(0.0, 0.0), (0.0, 10.0), (10.0, 10.0), (10.0, 20.0)];

        let res = points_with_gap(
            &points,
            &GapSpec {
                start: Length::Absolute(17.0),
                end: Length::Absolute(4.0),
            },
        );
        assert_eq!(res, vec![(7.0, 10.0), (10.0, 10.0), (10.0, 16.0)]);

        let res = points_with_gap(
            &points,
            &GapSpec {
                start: Length::Ratio(0.8),
                end: Length::Ratio(0.1),
            },
        );
        assert_eq!(res, vec![(10.0, 14.0), (10.0, 17.0)]);
    }
}
