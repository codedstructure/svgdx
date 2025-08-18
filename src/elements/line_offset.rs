use super::SvgElement;
use crate::errors::{Result, SvgdxError};
use crate::geometry::Length;

fn get_point_along_line(el: &SvgElement, length: Length) -> Result<(f32, f32)> {
    let (is_percent, dist) = match length {
        Length::Absolute(abs) => (false, abs),
        Length::Ratio(ratio) => (true, ratio),
    };

    if let (Some(x1), Some(y1), Some(x2), Some(y2)) = (
        el.get_attr("x1"),
        el.get_attr("y1"),
        el.get_attr("x2"),
        el.get_attr("y2"),
    ) {
        let x1: f32 = x1.parse()?;
        let y1: f32 = y1.parse()?;
        let x2: f32 = x2.parse()?;
        let y2: f32 = y2.parse()?;
        if x1 == x2 && y1 == y2 {
            return Ok((x1, y1));
        }

        let ratio = if is_percent {
            dist
        } else {
            let len = ((x1 - x2) * (x1 - x2) + (y1 - y2) * (y1 - y2)).sqrt();
            dist / len
        };
        return Ok((x1 + ratio * (x2 - x1), y1 + ratio * (y2 - y1)));
    }

    Err(SvgdxError::MissingAttribute(
        "in line either x1, y1, x2 or y2".to_string(),
    ))
}

fn get_point_along_polyline(el: &SvgElement, length: Length) -> Result<(f32, f32)> {
    let (mut is_percent, mut dist) = match length {
        Length::Absolute(abs) => (false, abs),
        Length::Ratio(ratio) => (true, ratio),
    };

    if let Some(points) = el.get_attr("points") {
        let replaced_commas = points.replace([','], " ");
        let mut lastx;
        let mut lasty;

        // loop to allow repeat to find total length if a percentage
        loop {
            let mut points = replaced_commas.split_whitespace();
            let mut cumulative_dist = 0.0;
            lastx = 0.0;
            lasty = 0.0;
            let mut first_point = true;
            while let (Some(x), Some(y)) = (points.next(), points.next()) {
                let x: f32 = x.parse()?;
                let y: f32 = y.parse()?;

                if !first_point {
                    let len = ((lastx - x) * (lastx - x) + (lasty - y) * (lasty - y)).sqrt();
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
            if !is_percent {
                break;
            } else {
                is_percent = false;
                dist *= cumulative_dist;
            }
        }
        return Ok((lastx, lasty));
    }

    Err(SvgdxError::MissingAttribute(
        "points in polyline".to_string(),
    ))
}

fn get_point_along_path(el: &SvgElement, length: Length) -> Result<(f32, f32)> {
    let (mut is_percent, mut dist) = match length {
        Length::Absolute(abs) => (false, abs),
        Length::Ratio(ratio) => (true, ratio),
    };

    if let Some(d) = el.get_attr("d") {
        let replaced_commas = d.replace([','], " ");
        let items = replaced_commas.split_whitespace();

        let mut pos;
        // loop to allow repeat to find total length if a percentage
        loop {
            let mut cumulative_dist = 0.0;
            pos = (0.0, 0.0);
            let mut last_stable_pos = pos;
            let mut r = 0.0;
            let mut large_arc_flag = false;
            let mut sweeping_flag = false;

            let mut op = ' ';
            let mut arg_num = 0;
            for item in items.clone() {
                if item.starts_with([
                    'a', 'A', 'b', 'B', 'c', 'C', 'h', 'H', 'l', 'L', 'm', 'M', 'q', 'Q', 's', 'S',
                    't', 'T', 'v', 'V', 'z', 'Z',
                ]) {
                    if let Some(c) = item.chars().next() {
                        op = c;
                        arg_num = 0;
                        if dist < 0.0 && !['m', 'M'].contains(&c) {
                            // clamping the start
                            return Ok(last_stable_pos);
                        }
                    }
                } else {
                    if ['b', 'B', 'c', 'C', 'q', 'Q', 's', 'S', 't', 'T', 'z', 'Z'].contains(&op) {
                        return Err(SvgdxError::InvalidData(format!(
                            "not yet impl path parsing line offset for {op}"
                        )));
                    } else if op == 'm' || op == 'M' {
                        if arg_num == 0 {
                            let val = item.parse::<f32>()?;
                            if op == 'm' {
                                pos.0 += val;
                            } else {
                                pos.0 = val;
                            }
                        } else if arg_num == 1 {
                            let val = item.parse::<f32>()?;
                            if op == 'm' {
                                pos.1 += val;
                            } else {
                                pos.1 = val;
                            }
                            last_stable_pos = pos;
                        } else {
                            return Err(SvgdxError::ParseError(
                                "path has too many vars".to_string(),
                            ));
                        }
                    } else if op == 'h' || op == 'H' {
                        if arg_num == 0 {
                            let val = item.parse::<f32>()?;
                            if op == 'h' {
                                pos.0 += val;
                            } else {
                                pos.0 = val;
                            }
                            let d = (pos.0 - last_stable_pos.0).abs();
                            if !is_percent && cumulative_dist + d > dist {
                                let r = (dist - cumulative_dist) / d;
                                return Ok((last_stable_pos.0 * (1.0 - r) + pos.0 * r, pos.1));
                            }

                            cumulative_dist += d;
                            last_stable_pos = pos;
                        } else {
                            return Err(SvgdxError::ParseError(
                                "path has too many vars".to_string(),
                            ));
                        }
                    } else if op == 'v' || op == 'V' {
                        if arg_num == 0 {
                            let val = item.parse::<f32>()?;
                            if op == 'v' {
                                pos.1 += val;
                            } else {
                                pos.1 = val;
                            }
                            let d = (pos.1 - last_stable_pos.1).abs();
                            if !is_percent && cumulative_dist + d > dist {
                                let r = (dist - cumulative_dist) / d;
                                return Ok((pos.0, last_stable_pos.1 * (1.0 - r) + pos.1 * r));
                            }

                            cumulative_dist += d;
                            last_stable_pos = pos;
                        } else {
                            return Err(SvgdxError::ParseError(
                                "path has too many vars".to_string(),
                            ));
                        }
                    } else if op == 'l' || op == 'L' {
                        if arg_num == 0 {
                            let val = item.parse::<f32>()?;
                            if op == 'l' {
                                pos.0 += val;
                            } else {
                                pos.0 = val;
                            }
                        } else if arg_num == 1 {
                            let val = item.parse::<f32>()?;
                            if op == 'l' {
                                pos.1 += val;
                            } else {
                                pos.1 = val;
                            }
                            let d = ((last_stable_pos.0 - pos.0) * (last_stable_pos.0 - pos.0)
                                + (last_stable_pos.1 - pos.1) * (last_stable_pos.1 - pos.1))
                                .sqrt();
                            if !is_percent && cumulative_dist + d > dist {
                                let r = (dist - cumulative_dist) / d;
                                return Ok((
                                    last_stable_pos.0 * (1.0 - r) + pos.0 * r,
                                    last_stable_pos.1 * (1.0 - r) + pos.1 * r,
                                ));
                            }

                            cumulative_dist += d;
                            last_stable_pos = pos;
                        } else {
                            return Err(SvgdxError::ParseError(
                                "path has too many vars".to_string(),
                            ));
                        }
                    } else if op == 'a' || op == 'A' {
                        if arg_num == 0 {
                            let val = item.parse::<f32>()?;
                            r = val;
                        } else if arg_num == 1 {
                            let val = item.parse::<f32>()?;
                            if r != val {
                                return Err(SvgdxError::ParseError(
                                    "path length not supported for non circle elipse".to_string(),
                                ));
                            }
                        } else if arg_num == 2 {
                            // unused as not mean anything for circle
                        } else if arg_num == 3 {
                            let val = item.parse::<u32>()?;
                            large_arc_flag = val != 0;
                        } else if arg_num == 4 {
                            let val = item.parse::<u32>()?;
                            sweeping_flag = val != 0;
                        } else if arg_num == 5 {
                            let val = item.parse::<f32>()?;
                            if op == 'a' {
                                pos.0 += val;
                            } else {
                                pos.0 = val;
                            }
                        } else if arg_num == 6 {
                            let val = item.parse::<f32>()?;
                            if op == 'a' {
                                pos.1 += val;
                            } else {
                                pos.1 = val;
                            }

                            let d2 = (last_stable_pos.0 - pos.0) * (last_stable_pos.0 - pos.0)
                                + (last_stable_pos.1 - pos.1) * (last_stable_pos.1 - pos.1);
                            let d = d2.sqrt();

                            let desc = r * r - d2 / 4.0;
                            let mid_point = (
                                (last_stable_pos.0 + pos.0) * 0.5,
                                (last_stable_pos.1 + pos.1) * 0.5,
                            );
                            let centre = if desc <= 0.0 {
                                r = d / 2.0;
                                mid_point
                            } else {
                                let inv_d = 1.0 / d;
                                let perp = (
                                    (last_stable_pos.1 - pos.1) * inv_d,
                                    (pos.0 - last_stable_pos.0) * inv_d,
                                );
                                let sign = large_arc_flag ^ sweeping_flag; // which circle to use
                                let len = if sign { desc.sqrt() } else { -desc.sqrt() };

                                (mid_point.0 + perp.0 * len, mid_point.1 + perp.1 * len)
                            };
                            let ang_1 =
                                (last_stable_pos.1 - centre.1).atan2(last_stable_pos.0 - centre.0);
                            let ang_2 = (pos.1 - centre.1).atan2(pos.0 - centre.0);

                            let mut shortest_arc_angle = ang_2 - ang_1;
                            if shortest_arc_angle < -std::f32::consts::PI {
                                shortest_arc_angle += std::f32::consts::PI * 2.0;
                            } else if shortest_arc_angle > std::f32::consts::PI {
                                shortest_arc_angle -= std::f32::consts::PI * 2.0;
                            }

                            if (shortest_arc_angle.abs() - std::f32::consts::PI).abs() < 0.0001
                                && (shortest_arc_angle > 0.0) ^ !sweeping_flag
                            {
                                shortest_arc_angle = -shortest_arc_angle;
                            }

                            let arc_angle = if large_arc_flag {
                                (std::f32::consts::PI * 2.0 - shortest_arc_angle.abs())
                                    * shortest_arc_angle.signum()
                            } else {
                                shortest_arc_angle
                            };
                            let arc_length = arc_angle.abs() * r;

                            if !is_percent && cumulative_dist + arc_length > dist {
                                let ratio = (dist - cumulative_dist) / arc_length;
                                let final_angle = ang_1 + arc_angle * ratio;

                                return Ok((
                                    centre.0 + r * (final_angle).cos(),
                                    centre.1 + r * (final_angle).sin(),
                                ));
                            }

                            cumulative_dist += arc_length;
                            last_stable_pos = pos;
                        } else {
                            return Err(SvgdxError::ParseError(
                                "path has too many vars".to_string(),
                            ));
                        }
                    }

                    arg_num += 1;
                }
            }
            if !is_percent {
                break;
            } else {
                is_percent = false;
                dist *= cumulative_dist;
            }
        }
        return Ok(pos);
    }

    Err(SvgdxError::MissingAttribute("d in path".to_string()))
}

pub fn get_point_along_linelike_type_el(el: &SvgElement, length: Length) -> Result<(f32, f32)> {
    let name = el.name();

    if name == "line" {
        return get_point_along_line(el, length);
    }
    if name == "polyline" {
        return get_point_along_polyline(el, length);
    }
    if name == "path" {
        return get_point_along_path(el, length);
    }

    Err(SvgdxError::MissingAttribute(
        "looking for point on line in a non line element".to_string(),
    ))
}

#[cfg(test)]
mod tests {

    use assertables::assert_abs_diff_le_x;

    use crate::{
        elements::{line_offset::get_point_along_linelike_type_el, SvgElement},
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
        assert_abs_diff_le_x!(x, -3.0, 0.001);
        assert_abs_diff_le_x!(y, 3.0, 0.001);

        // over extended other direction
        let element = SvgElement::new(
            "path",
            &[("d".to_string(), "M 1 2 a 3 3 45 0 1 -6 -6".to_string())],
        );
        let result = get_point_along_linelike_type_el(&element, Length::Ratio(0.5));
        assert!(result.is_ok());
        let (x, y) = result.unwrap();
        assert_abs_diff_le_x!(x, 1.0, 0.001);
        assert_abs_diff_le_x!(y, -4.0, 0.001);

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
