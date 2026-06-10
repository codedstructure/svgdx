use super::SvgElement;
use super::syntax::PATH_COMMANDS;
use crate::errors::{Error, Result};
use crate::geometry::Length;
use crate::types::{attr_split, strp};

pub fn get_point_along_path(el: &SvgElement, length: Length) -> Result<(f32, f32)> {
    let (is_percent, dist) = match length {
        Length::Absolute(abs) => (false, abs),
        Length::Ratio(ratio) => (true, ratio),
        Length::Rational(numer, denom) => (true, numer as f32 / denom.get() as f32),
    };

    if let Some(d) = el.get_attr("d") {
        let mut pos = (0.0, 0.0);

        let mut cumulative_dist = 0.0;
        let mut last_stable_pos = pos;
        let mut r = 0.0;
        let mut large_arc_flag = false;
        let mut sweeping_flag = false;

        let mut op = ' ';
        let mut arg_num = 0;
        for item in attr_split(d) {
            if item.starts_with(PATH_COMMANDS) {
                if let Some(c) = item.chars().next() {
                    op = c;
                    arg_num = 0;
                    if dist < 0.0 && !['m', 'M'].contains(&c) {
                        // clamping the start
                        return Ok(last_stable_pos);
                    }
                }
            } else {
                let num_args = match op {
                    'm' | 'M' | 'l' | 'L' => 2,
                    'h' | 'H' | 'v' | 'V' => 1,
                    'a' | 'A' => 7,
                    _ => {
                        return Err(Error::InvalidValue(
                            "point_along_path() unhandled command".into(),
                            op.to_string(),
                        ));
                    }
                };
                if arg_num == num_args {
                    return Err(Error::Parse("path has too many vars".to_string()));
                }

                match op {
                    'm' | 'M' => {
                        let val = strp(&item)?;
                        let pos_ref = if arg_num == 0 { &mut pos.0 } else { &mut pos.1 };
                        if op == 'm' {
                            *pos_ref += val;
                        } else {
                            *pos_ref = val;
                        }
                    }
                    'h' | 'H' | 'v' | 'V' => {
                        let val = strp(&item)?;
                        let pos_ref = if op == 'h' || op == 'H' {
                            &mut pos.0
                        } else {
                            &mut pos.1
                        };
                        if op == 'h' || op == 'v' {
                            *pos_ref += val;
                        } else {
                            *pos_ref = val;
                        }
                        let d =
                            (pos.0 - last_stable_pos.0).abs() + (pos.1 - last_stable_pos.1).abs();
                        if !is_percent && cumulative_dist + d > dist {
                            let r = (dist - cumulative_dist) / d;
                            return Ok((
                                last_stable_pos.0 * (1.0 - r) + pos.0 * r,
                                last_stable_pos.1 * (1.0 - r) + pos.1 * r,
                            ));
                        }

                        cumulative_dist += d;
                    }
                    'l' | 'L' => {
                        let val = strp(&item)?;
                        if arg_num == 0 {
                            if op == 'l' {
                                pos.0 += val;
                            } else {
                                pos.0 = val;
                            }
                        } else {
                            if op == 'l' {
                                pos.1 += val;
                            } else {
                                pos.1 = val;
                            }
                            let d = (last_stable_pos.0 - pos.0).hypot(last_stable_pos.1 - pos.1);
                            if !is_percent && cumulative_dist + d > dist {
                                let r = (dist - cumulative_dist) / d;
                                return Ok((
                                    last_stable_pos.0 * (1.0 - r) + pos.0 * r,
                                    last_stable_pos.1 * (1.0 - r) + pos.1 * r,
                                ));
                            }

                            cumulative_dist += d;
                        }
                    }
                    'a' | 'A' => {
                        if arg_num == 0 {
                            let val = strp(&item)?;
                            r = val;
                        } else if arg_num == 1 {
                            let val = strp(&item)?;
                            if r != val {
                                return Err(Error::Parse(
                                    "path length not supported for non circle ellipse".to_string(),
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
                            let val = strp(&item)?;
                            if op == 'a' {
                                pos.0 += val;
                            } else {
                                pos.0 = val;
                            }
                        } else {
                            let val = strp(&item)?;
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
                        }
                    }
                    _ => {
                        return Err(Error::InvalidValue(
                            "point_along_path() unhandled command".into(),
                            op.to_string(),
                        ));
                    }
                }

                arg_num += 1;

                if arg_num == num_args {
                    last_stable_pos = pos;
                }
            }
        }
        if is_percent {
            return get_point_along_path(el, Length::Absolute(dist * cumulative_dist));
        }
        return Ok(pos);
    }

    Err(Error::MissingAttr("path element requires d".to_string()))
}
