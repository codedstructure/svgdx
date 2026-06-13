use super::{SvgElement, path::get_point_along_path};
use crate::errors::{Error, Result};
use crate::geometry::Length;
use crate::types::{attr_split, strp};

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
        let x1: f32 = strp(x1)?;
        let y1: f32 = strp(y1)?;
        let x2: f32 = strp(x2)?;
        let y2: f32 = strp(y2)?;
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
            let x: f32 = strp(&x)?;
            let y: f32 = strp(&y)?;

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

pub fn get_point_along_linelike_type_el(el: &SvgElement, length: Length) -> Result<(f32, f32)> {
    match el.name() {
        "line" => get_point_along_line(el, length),
        "polyline" => get_point_along_polyline(el, length),
        "path" => get_point_along_path(el, length),
        _ => Err(Error::InternalLogic(
            "point_along_line on a non line-like element".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {

    use assertables::assert_abs_diff_le_x;

    use crate::{
        elements::{SvgElement, line_offset::get_point_along_linelike_type_el},
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
        let result = get_point_along_linelike_type_el(&element, Length::Absolute(1.0));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_abs_diff_le_x!(x, 1.6, 0.001);
        assert_abs_diff_le_x!(y, -0.3, 0.001);

        // test ratio
        let result = get_point_along_linelike_type_el(&element, Length::Ratio(0.6));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_abs_diff_le_x!(x, 1.45, 0.001);
        assert_abs_diff_le_x!(y, -0.1, 0.001);

        // negative abs
        let result = get_point_along_linelike_type_el(&element, Length::Absolute(-0.5));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_abs_diff_le_x!(x, 0.7, 0.001);
        assert_abs_diff_le_x!(y, 0.9, 0.001);

        // too big ratio
        let result = get_point_along_linelike_type_el(&element, Length::Ratio(1.8));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_abs_diff_le_x!(x, 2.35, 0.001);
        assert_abs_diff_le_x!(y, -1.3, 0.001);
    }

    #[test]
    fn test_polyline() {
        let element = SvgElement::new(
            "polyline",
            &[("points".to_string(), "1 2 4 6 -8 1 -1 1".to_string())],
        );
        // length is 5 + 13 + 7 = 25

        // test absolute in first section
        let result = get_point_along_linelike_type_el(&element, Length::Absolute(1.0));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_abs_diff_le_x!(x, 1.6, 0.001);
        assert_abs_diff_le_x!(y, 2.8, 0.001);

        // test ratio in first section
        let result = get_point_along_linelike_type_el(&element, Length::Ratio(0.1));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_abs_diff_le_x!(x, 2.5, 0.001);
        assert_abs_diff_le_x!(y, 4.0, 0.001);

        // test abs
        let result = get_point_along_linelike_type_el(&element, Length::Absolute(11.5));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_abs_diff_le_x!(x, -2.0, 0.001);
        assert_abs_diff_le_x!(y, 3.5, 0.001);

        // test ratio
        let result = get_point_along_linelike_type_el(&element, Length::Ratio(0.85));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_abs_diff_le_x!(x, -4.75, 0.001);
        assert_abs_diff_le_x!(y, 1.0, 0.001);

        // test abs negative
        let result = get_point_along_linelike_type_el(&element, Length::Absolute(-0.85));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_abs_diff_le_x!(x, 1.0, 0.001);
        assert_abs_diff_le_x!(y, 2.0, 0.001);

        // test ratio large
        let result = get_point_along_linelike_type_el(&element, Length::Ratio(2.85));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_abs_diff_le_x!(x, -1.0, 0.001);
        assert_abs_diff_le_x!(y, 1.0, 0.001);
    }

    #[test]
    fn test_path() {
        // test empty d
        let element = SvgElement::new("path", &[("d".to_string(), "".to_string())]);
        let result = get_point_along_linelike_type_el(&element, Length::Absolute(1.5));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_abs_diff_le_x!(x, 0.0, 0.001);
        assert_abs_diff_le_x!(y, 0.0, 0.001);

        // test only M
        let element = SvgElement::new("path", &[("d".to_string(), "M 1 2".to_string())]);
        let result = get_point_along_linelike_type_el(&element, Length::Absolute(1.5));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_abs_diff_le_x!(x, 1.0, 0.001);
        assert_abs_diff_le_x!(y, 2.0, 0.001);

        // test h
        let element = SvgElement::new("path", &[("d".to_string(), "M 1 2 h 4".to_string())]);
        let result = get_point_along_linelike_type_el(&element, Length::Ratio(0.75));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_abs_diff_le_x!(x, 4.0, 0.001);
        assert_abs_diff_le_x!(y, 2.0, 0.001);

        // test v
        let element = SvgElement::new("path", &[("d".to_string(), "M 1 2 v 4".to_string())]);
        let result = get_point_along_linelike_type_el(&element, Length::Ratio(0.75));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_abs_diff_le_x!(x, 1.0, 0.001);
        assert_abs_diff_le_x!(y, 5.0, 0.001);

        // test H
        let element = SvgElement::new("path", &[("d".to_string(), "M 1 2 H 4".to_string())]);
        let result = get_point_along_linelike_type_el(&element, Length::Ratio(0.75));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_abs_diff_le_x!(x, 3.25, 0.001);
        assert_abs_diff_le_x!(y, 2.0, 0.001);

        // test V
        let element = SvgElement::new("path", &[("d".to_string(), "M 1 2 V 4".to_string())]);
        let result = get_point_along_linelike_type_el(&element, Length::Ratio(0.75));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_abs_diff_le_x!(x, 1.0, 0.001);
        assert_abs_diff_le_x!(y, 3.5, 0.001);

        // test l
        let element = SvgElement::new("path", &[("d".to_string(), "M 1 2 l 3 4".to_string())]);
        let result = get_point_along_linelike_type_el(&element, Length::Absolute(3.0));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_abs_diff_le_x!(x, 2.8, 0.001);
        assert_abs_diff_le_x!(y, 4.4, 0.001);

        // test L
        let element = SvgElement::new("path", &[("d".to_string(), "M 1 2 L 7 10".to_string())]);
        let result = get_point_along_linelike_type_el(&element, Length::Ratio(0.7));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_abs_diff_le_x!(x, 5.2, 0.001);
        assert_abs_diff_le_x!(y, 7.6, 0.001);

        // test a

        // over extended
        let element = SvgElement::new(
            "path",
            &[("d".to_string(), "M 3 3 a 3 3 45 0 0 -6 -6".to_string())],
        );
        let result = get_point_along_linelike_type_el(&element, Length::Ratio(0.5));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_abs_diff_le_x!(x, 3.0, 0.001);
        assert_abs_diff_le_x!(y, -3.0, 0.001);

        // over extended other direction
        let element = SvgElement::new(
            "path",
            &[("d".to_string(), "M 1 2 a 3 3 45 0 1 -6 -6".to_string())],
        );
        let result = get_point_along_linelike_type_el(&element, Length::Ratio(0.5));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_abs_diff_le_x!(x, -5.0, 0.001);
        assert_abs_diff_le_x!(y, 2.0, 0.001);

        // test A

        // test not over extended
        let element = SvgElement::new(
            "path",
            &[("d".to_string(), "M 3 0 A 3 3 45 0 1 0 3".to_string())],
        );
        let result = get_point_along_linelike_type_el(&element, Length::Ratio(0.5));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_abs_diff_le_x!(x, 1.5 * 2.0f32.sqrt(), 0.001);
        assert_abs_diff_le_x!(y, 1.5 * 2.0f32.sqrt(), 0.001);
    }
}
