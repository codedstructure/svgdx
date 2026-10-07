use super::{SvgElement, path::get_point_along_path};
use crate::errors::{Error, Result};
use crate::geometry::{Length, TransformAttr};
use crate::types::{attr_split, parse_float};

fn get_point_along_line(el: &SvgElement, length: Length) -> Result<(f32, f32)> {
    let (is_percent, dist) = match length {
        Length::Absolute(abs) => (false, abs),
        Length::Ratio(ratio) => (true, ratio),
        Length::Rational(numer, denom) => (true, numer as f32 / denom.get() as f32),
    };

    if let (Some(x1), Some(y1), Some(x2), Some(y2)) = (
        el.get_attr("x1"),
        el.get_attr("y1"),
        el.get_attr("x2"),
        el.get_attr("y2"),
    ) {
        let x1: f32 = parse_float(x1)?;
        let y1: f32 = parse_float(y1)?;
        let x2: f32 = parse_float(x2)?;
        let y2: f32 = parse_float(y2)?;
        if x1 == x2 && y1 == y2 {
            return Ok((x1, y1));
        }

        let ratio = if is_percent {
            dist
        } else {
            let len = (x1 - x2).hypot(y1 - y2);
            dist / len
        };
        return Ok((x1 + ratio * (x2 - x1), y1 + ratio * (y2 - y1)));
    }

    Err(Error::MissingAttr(
        "line element requires x1, y1, x2 and y2".to_string(),
    ))
}

fn get_point_along_polyline(el: &SvgElement, length: Length) -> Result<(f32, f32)> {
    let (is_percent, dist) = match length {
        Length::Absolute(abs) => (false, abs),
        Length::Ratio(ratio) => (true, ratio),
        Length::Rational(numer, denom) => (true, numer as f32 / denom.get() as f32),
    };

    if let Some(points) = el.get_attr("points") {
        let mut lastx = 0.0;
        let mut lasty = 0.0;

        let mut points = attr_split(points);
        let mut cumulative_dist = 0.0;
        let mut first_point = true;
        while let (Some(x), Some(y)) = (points.next(), points.next()) {
            let x: f32 = parse_float(x)?;
            let y: f32 = parse_float(y)?;

            if !first_point {
                let len = (lastx - x).hypot(lasty - y);
                if !is_percent && cumulative_dist + len > dist {
                    let ratio = (dist - cumulative_dist) / len;
                    return Ok((
                        lastx * (1.0 - ratio) + ratio * x,
                        lasty * (1.0 - ratio) + ratio * y,
                    ));
                }
                cumulative_dist += len;
            } else if dist < 0.0 {
                // clamp to start
                return Ok((x, y));
            }
            lastx = x;
            lasty = y;

            first_point = false;
        }
        if is_percent {
            return get_point_along_polyline(el, Length::Absolute(dist * cumulative_dist));
        }
        return Ok((lastx, lasty));
    }

    Err(Error::MissingAttr(
        "polyline element requires points".to_string(),
    ))
}

pub fn get_point_along_linelike_el(el: &SvgElement, length: Length) -> Result<(f32, f32)> {
    let point = match el.name() {
        "line" => get_point_along_line(el, length),
        "polyline" => get_point_along_polyline(el, length),
        "path" => get_point_along_path(el, length),
        _ => Err(Error::InvalidElement(
            "point_along_line on a non line-like element".to_string(),
        )),
    }?;

    if let Some(transform) = el.get_attr("transform") {
        let transform: TransformAttr = transform.parse()?;
        transform.apply_to_point(point.0, point.1)
    } else {
        Ok(point)
    }
}

#[cfg(test)]
mod tests {

    fn assert_nearly_eq(a: f32, b: f32) {
        const EPSILON: f32 = 0.001;
        if (a - b).abs() > EPSILON {
            panic!("assert_nearly_eq failed: {} != {}", a, b);
        }
    }

    use crate::{
        elements::{SvgElement, line_offset::get_point_along_linelike_el},
        geometry::Length,
    };

    #[test]
    fn test_line() {
        let element = SvgElement::new(
            "line",
            &[
                ("x1".to_string(), "1".to_string()),
                ("y1".to_string(), "0.5".to_string()),
                ("x2".to_string(), "1.75".to_string()),
                ("y2".to_string(), "-0.5".to_string()),
            ],
        );

        // test absolute
        let result = get_point_along_linelike_el(&element, Length::Absolute(1.0));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_nearly_eq(x, 1.6);
        assert_nearly_eq(y, -0.3);

        // test ratio
        let result = get_point_along_linelike_el(&element, Length::Ratio(0.6));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_nearly_eq(x, 1.45);
        assert_nearly_eq(y, -0.1);

        // negative abs
        let result = get_point_along_linelike_el(&element, Length::Absolute(-0.5));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_nearly_eq(x, 0.7);
        assert_nearly_eq(y, 0.9);

        // too big ratio
        let result = get_point_along_linelike_el(&element, Length::Ratio(1.8));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_nearly_eq(x, 2.35);
        assert_nearly_eq(y, -1.3);
    }

    #[test]
    fn test_polyline() {
        let element = SvgElement::new(
            "polyline",
            &[("points".to_string(), "1 2 4 6 -8 1 -1 1".to_string())],
        );
        // length is 5 + 13 + 7 = 25

        // test absolute in first section
        let result = get_point_along_linelike_el(&element, Length::Absolute(1.0));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_nearly_eq(x, 1.6);
        assert_nearly_eq(y, 2.8);

        // test ratio in first section
        let result = get_point_along_linelike_el(&element, Length::Ratio(0.1));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_nearly_eq(x, 2.5);
        assert_nearly_eq(y, 4.0);

        // test abs
        let result = get_point_along_linelike_el(&element, Length::Absolute(11.5));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_nearly_eq(x, -2.0);
        assert_nearly_eq(y, 3.5);

        // test ratio
        let result = get_point_along_linelike_el(&element, Length::Ratio(0.85));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_nearly_eq(x, -4.75);
        assert_nearly_eq(y, 1.0);

        // test abs negative
        let result = get_point_along_linelike_el(&element, Length::Absolute(-0.85));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_nearly_eq(x, 1.0);
        assert_nearly_eq(y, 2.0);

        // test ratio large
        let result = get_point_along_linelike_el(&element, Length::Ratio(2.85));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_nearly_eq(x, -1.0);
        assert_nearly_eq(y, 1.0);
    }

    #[test]
    fn test_path() {
        // test empty d
        let element = SvgElement::new("path", &[("d".to_string(), "".to_string())]);
        let result = get_point_along_linelike_el(&element, Length::Absolute(1.5));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_nearly_eq(x, 0.0);
        assert_nearly_eq(y, 0.0);

        // test only M
        let element = SvgElement::new("path", &[("d".to_string(), "M 1 2".to_string())]);
        let result = get_point_along_linelike_el(&element, Length::Absolute(1.5));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_nearly_eq(x, 1.0);
        assert_nearly_eq(y, 2.0);

        // test h
        let element = SvgElement::new("path", &[("d".to_string(), "M 1 2 h 4".to_string())]);
        let result = get_point_along_linelike_el(&element, Length::Ratio(0.75));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_nearly_eq(x, 4.0);
        assert_nearly_eq(y, 2.0);

        // test v
        let element = SvgElement::new("path", &[("d".to_string(), "M 1 2 v 4".to_string())]);
        let result = get_point_along_linelike_el(&element, Length::Ratio(0.75));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_nearly_eq(x, 1.0);
        assert_nearly_eq(y, 5.0);

        // test H
        let element = SvgElement::new("path", &[("d".to_string(), "M 1 2 H 4".to_string())]);
        let result = get_point_along_linelike_el(&element, Length::Ratio(0.75));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_nearly_eq(x, 3.25);
        assert_nearly_eq(y, 2.0);

        // test V
        let element = SvgElement::new("path", &[("d".to_string(), "M 1 2 V 4".to_string())]);
        let result = get_point_along_linelike_el(&element, Length::Ratio(0.75));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_nearly_eq(x, 1.0);
        assert_nearly_eq(y, 3.5);

        // test l
        let element = SvgElement::new("path", &[("d".to_string(), "M 1 2 l 3 4".to_string())]);
        let result = get_point_along_linelike_el(&element, Length::Absolute(3.0));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_nearly_eq(x, 2.8);
        assert_nearly_eq(y, 4.4);

        // test L
        let element = SvgElement::new("path", &[("d".to_string(), "M 1 2 L 7 10".to_string())]);
        let result = get_point_along_linelike_el(&element, Length::Ratio(0.7));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_nearly_eq(x, 5.2);
        assert_nearly_eq(y, 7.6);

        // test a

        // over extended
        let element = SvgElement::new(
            "path",
            &[("d".to_string(), "M 3 3 a 3 3 45 0 0 -6 -6".to_string())],
        );
        let result = get_point_along_linelike_el(&element, Length::Ratio(0.5));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_nearly_eq(x, 3.0);
        assert_nearly_eq(y, -3.0);

        // over extended other direction
        let element = SvgElement::new(
            "path",
            &[("d".to_string(), "M 1 2 a 3 3 45 0 1 -6 -6".to_string())],
        );
        let result = get_point_along_linelike_el(&element, Length::Ratio(0.5));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_nearly_eq(x, -5.0);
        assert_nearly_eq(y, 2.0);

        // test A

        // test not over extended
        let element = SvgElement::new(
            "path",
            &[("d".to_string(), "M 3 0 A 3 3 45 0 1 0 3".to_string())],
        );
        let result = get_point_along_linelike_el(&element, Length::Ratio(0.5));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_nearly_eq(x, 1.5 * 2.0f32.sqrt());
        assert_nearly_eq(y, 1.5 * 2.0f32.sqrt());
    }

    #[test]
    fn test_path_transform() {
        let element = SvgElement::new(
            "path",
            &[
                ("d".to_string(), "M 0 0 h 10".to_string()),
                ("transform".to_string(), "translate(10, 0)".to_string()),
            ],
        );

        let result = get_point_along_linelike_el(&element, Length::Ratio(0.5));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_nearly_eq(x, 15.0);
        assert_nearly_eq(y, 0.0);
    }
}
