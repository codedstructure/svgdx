use super::SvgElement;
use crate::errors::{Error, Result};
use crate::geometry::BoundingBox;

use super::syntax::{PathSyntax, SvgPathSyntax};

struct PathParser {
    tokens: SvgPathSyntax,
    // current position, updated as commands are processed
    position: Option<(f32, f32)>,
    // location to return to for 'Z'/'z' commands
    subpath_start: Option<(f32, f32)>,
    // current command being processed; most commands take multiple parameter
    // sets without repeating the command character
    command: Option<char>,
    // previous second control point (if any) for evaluating 'S' and 's'
    cubic_cp2: Option<(f32, f32)>,
    // previous control point (if any) for evaluating 'T' and 't'
    quadratic_cp: Option<(f32, f32)>,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

impl PathParser {
    fn new(data: &str) -> Self {
        PathParser {
            tokens: SvgPathSyntax::new(data),
            position: None,
            subpath_start: None,
            command: None,
            cubic_cp2: None,
            quadratic_cp: None,
            min_x: 0.,
            min_y: 0.,
            max_x: 0.,
            max_y: 0.,
        }
    }

    fn update_position(&mut self, pos: (f32, f32)) {
        let old_pos = self.position;
        self.position = Some(pos);
        if old_pos.is_none() {
            self.min_x = pos.0;
            self.min_y = pos.1;
            self.max_x = pos.0;
            self.max_y = pos.1;
        } else {
            self.min_x = self.min_x.min(pos.0);
            self.min_y = self.min_y.min(pos.1);
            self.max_x = self.max_x.max(pos.0);
            self.max_y = self.max_y.max(pos.1);
        }
    }

    fn get_bbox(&self) -> Option<BoundingBox> {
        if self.position.is_some() {
            Some(BoundingBox::new(
                self.min_x, self.min_y, self.max_x, self.max_y,
            ))
        } else {
            None // we've never called update_position()
        }
    }

    fn process_instruction(&mut self) -> Result<()> {
        if self.command.is_none() || self.tokens.at_command()? {
            // "The command letter can be eliminated on subsequent commands if the same
            // command is used multiple times in a row (e.g., you can drop the second
            // "L" in "M 100 200 L 200 100 L -100 -200" and use "M 100 200 L 200 100
            // -100 -200" instead)."
            self.command = Some(self.tokens.read_command()?);
        } else {
            // this will only happen for subsequent values to an existing command
            match self.command {
                // "If a moveto is followed by multiple pairs of coordinates,
                // the subsequent pairs are treated as implicit lineto commands."
                Some('m') => {
                    self.command = Some('l');
                }
                Some('M') => {
                    self.command = Some('L');
                }
                _ => {}
            }
        }

        let mut cubic_cp2: Option<(f32, f32)> = None;
        let mut quadratic_cp: Option<(f32, f32)> = None;

        match self.command.expect("Command should be already set") {
            'M' => {
                // "(x y)+"
                let xy = self.tokens.read_coord()?;
                self.update_position(xy);
                // 'Subsequent "moveto" commands (i.e., when the "moveto" is not
                // the first command) represent the start of a new subpath'
                // (the first moveto is also the start of a subpath)
                self.subpath_start = Some(xy);
            }
            'm' => {
                // "(x y)+"
                let (dx, dy) = self.tokens.read_coord()?;
                let (px, py) = self.position.unwrap_or((0., 0.));
                let xy = (px + dx, py + dy);
                self.update_position(xy);
                // 'Subsequent "moveto" commands (i.e., when the "moveto" is not
                // the first command) represent the start of a new subpath'
                self.subpath_start = Some(xy);
            }
            'L' => {
                // "(x y)+"
                let xy = self.tokens.read_coord()?;
                self.update_position(xy);
            }
            'l' => {
                // "(x y)+"
                let (dx, dy) = self.tokens.read_coord()?;
                let (px, py) = self.position.unwrap_or((0., 0.));
                self.update_position((px + dx, py + dy));
            }
            'H' => {
                // "x+"
                let new_x = self.tokens.read_number()?;
                let (_, py) = self.position.unwrap_or((0., 0.));
                self.update_position((new_x, py));
            }
            'h' => {
                // "x+"
                let dx = self.tokens.read_number()?;
                let (px, py) = self.position.unwrap_or((0., 0.));
                self.update_position((px + dx, py));
            }
            'V' => {
                // "y+"
                let new_y = self.tokens.read_number()?;
                let (px, _) = self.position.unwrap_or((0., 0.));
                self.update_position((px, new_y));
            }
            'v' => {
                // "y+"
                let dy = self.tokens.read_number()?;
                let (px, py) = self.position.unwrap_or((0., 0.));
                self.update_position((px, py + dy));
            }
            'Z' | 'z' => {
                self.update_position(self.subpath_start.unwrap_or((0., 0.)));
                // since this doesn't consume further tokens, we must clear the command
                // to force getting a new command token, or we could loop forever
                self.command = None;
            }
            'C' => {
                // (x1 y1 x2 y2 x y)+
                let cp1 = self.tokens.read_coord()?; // control point 1
                let cp2 = self.tokens.read_coord()?; // control point 2
                let end = self.tokens.read_coord()?;
                let start = self.position.unwrap_or((0., 0.));

                cubic_cp2 = Some(cp2);
                let extrema = cubic_extrema(start, cp1, cp2, end);
                for point in extrema {
                    self.update_position(point);
                }
                self.update_position(end);
            }
            'c' => {
                // (x1 y1 x2 y2 x y)+
                let cp1 = self.tokens.read_coord()?; // control point 1
                let cp2 = self.tokens.read_coord()?; // control point 2
                let (dx, dy) = self.tokens.read_coord()?;
                let (px, py) = self.position.unwrap_or((0., 0.));

                let start = (px, py);
                let end = (px + dx, py + dy);
                let cp1 = (px + cp1.0, py + cp1.1);
                let cp2 = (px + cp2.0, py + cp2.1);

                cubic_cp2 = Some(cp2);
                let extrema = cubic_extrema(start, cp1, cp2, end);
                for point in extrema {
                    self.update_position(point);
                }
                self.update_position(end);
            }
            'S' => {
                // S: "(x2 y2 x y)+"
                let cp2 = self.tokens.read_coord()?;
                let end = self.tokens.read_coord()?;
                let start = self.position.unwrap_or((0., 0.));

                // "The first control point is assumed to be the reflection of the second
                //  control point on the previous command relative to the current point.
                //  If there is no previous command or if the previous command was not an
                //  C, c, S or s, assume the first control point is coincident with the
                //  current point."
                let cp1 = if let Some((prev_cp2x, prev_cp2y)) = self.cubic_cp2 {
                    let (px, py) = start;
                    (2. * px - prev_cp2x, 2. * py - prev_cp2y)
                } else {
                    start
                };

                cubic_cp2 = Some(cp2);
                let extrema = cubic_extrema(start, cp1, cp2, end);
                for e in extrema {
                    self.update_position(e);
                }
                self.update_position(end);
            }
            's' => {
                // s: "(x2 y2 x y)+"
                let cp2 = self.tokens.read_coord()?;
                let (dx, dy) = self.tokens.read_coord()?;
                let (px, py) = self.position.unwrap_or((0., 0.));

                let end = (px + dx, py + dy);
                let start = (px, py);

                // "The first control point is assumed to be the reflection of the second
                //  control point on the previous command relative to the current point.
                //  If there is no previous command or if the previous command was not an
                //  C, c, S or s, assume the first control point is coincident with the
                //  current point."
                let cp1 = if let Some((prev_cp2x, prev_cp2y)) = self.cubic_cp2 {
                    (2. * px - prev_cp2x, 2. * py - prev_cp2y)
                } else {
                    start
                };
                let cp2 = (px + cp2.0, py + cp2.1);

                cubic_cp2 = Some(cp2);
                let extrema = cubic_extrema(start, cp1, cp2, end);
                for e in extrema {
                    self.update_position(e);
                }
                self.update_position(end);
            }
            'Q' => {
                // Q: "(x1 y1 x y)+"
                let cp = self.tokens.read_coord()?;
                let end = self.tokens.read_coord()?;
                let start = self.position.unwrap_or((0., 0.));

                quadratic_cp = Some(cp);
                for e in quadratic_extrema(start, cp, end) {
                    self.update_position(e);
                }
                self.update_position(end);
            }
            'q' => {
                // q: "(x1 y1 x y)+"
                let cp = self.tokens.read_coord()?; // control point
                let (dx, dy) = self.tokens.read_coord()?;
                let (px, py) = self.position.unwrap_or((0., 0.));

                let start = (px, py);
                let end = (px + dx, py + dy);
                let cp = (px + cp.0, py + cp.1);

                quadratic_cp = Some(cp);
                for e in quadratic_extrema(start, cp, end) {
                    self.update_position(e);
                }
                self.update_position(end);
            }
            'T' => {
                // "(x y)+"
                let end = self.tokens.read_coord()?;
                let start = self.position.unwrap_or((0., 0.));
                // "The control point is assumed to be the reflection of the control point
                //  on the previous command relative to the current point. (If there is no
                //  previous command or if the previous command was not a Q, q, T or t,
                //  assume the control point is coincident with the current point.)"
                let cp = if let Some((prev_cpx, prev_cpy)) = self.quadratic_cp {
                    let (px, py) = start;
                    (2. * px - prev_cpx, 2. * py - prev_cpy)
                } else {
                    start
                };

                quadratic_cp = Some(cp);
                for e in quadratic_extrema(start, cp, end) {
                    self.update_position(e);
                }
                self.update_position(end);
            }
            't' => {
                // "(x y)+"
                let (dx, dy) = self.tokens.read_coord()?;
                let start = self.position.unwrap_or((0., 0.));
                let (px, py) = start;
                // "The control point is assumed to be the reflection of the control point
                //  on the previous command relative to the current point. (If there is no
                //  previous command or if the previous command was not a Q, q, T or t,
                //  assume the control point is coincident with the current point.)"
                let cp = if let Some((prev_cpx, prev_cpy)) = self.quadratic_cp {
                    (2. * px - prev_cpx, 2. * py - prev_cpy)
                } else {
                    start
                };

                let end = (px + dx, py + dy);

                quadratic_cp = Some(cp);
                for e in quadratic_extrema(start, cp, end) {
                    self.update_position(e);
                }
                self.update_position(end);
            }
            'A' => {
                // "(rx ry x-axis-rotation large-arc-flag sweep-flag x y)+"
                let rx = self.tokens.read_non_negative()?;
                let ry = self.tokens.read_non_negative()?;
                let x_axis_rotation = self.tokens.read_number()?;
                let large_arc_flag = self.tokens.read_flag()? != 0;
                let sweep_flag = self.tokens.read_flag()? != 0;
                let end = self.tokens.read_coord()?;
                let start = self.position.unwrap_or((0., 0.));

                let extrema = arc_extrema(
                    start,
                    rx,
                    ry,
                    x_axis_rotation,
                    large_arc_flag,
                    sweep_flag,
                    end,
                );
                for point in extrema {
                    self.update_position(point);
                }
                self.update_position(end);
            }
            'a' => {
                // "(rx ry x-axis-rotation large-arc-flag sweep-flag x y)+"
                let rx = self.tokens.read_non_negative()?;
                let ry = self.tokens.read_non_negative()?;
                let x_axis_rotation = self.tokens.read_number()?;
                let large_arc_flag = self.tokens.read_flag()? != 0;
                let sweep_flag = self.tokens.read_flag()? != 0;
                let (dx, dy) = self.tokens.read_coord()?;
                let start = self.position.unwrap_or((0., 0.));
                let end = (start.0 + dx, start.1 + dy);

                let extrema = arc_extrema(
                    start,
                    rx,
                    ry,
                    x_axis_rotation,
                    large_arc_flag,
                    sweep_flag,
                    end,
                );
                for point in extrema {
                    self.update_position(point);
                }
                self.update_position(end);
            }
            _ => Err(Error::InvalidValue(
                "path command".to_string(),
                self.command.unwrap_or_default().to_string(),
            ))?,
        }
        self.cubic_cp2 = cubic_cp2;
        self.quadratic_cp = quadratic_cp;
        Ok(())
    }

    fn evaluate(&mut self) -> Result<()> {
        self.tokens.skip_whitespace();
        while !self.tokens.at_end() {
            self.process_instruction()?;
        }
        Ok(())
    }
}

/// Compute the extremal points of a quadratic Bezier
/// by solving for dx/dt = 0, dy/dt = 0
fn quadratic_extrema(start: (f32, f32), cp: (f32, f32), end: (f32, f32)) -> Vec<(f32, f32)> {
    // Evaluate quadratic Bezier for a single coordinate
    fn bezier(t: f32, p0: f32, p1: f32, p2: f32) -> f32 {
        let mt = 1.0 - t;
        mt * mt * p0 + 2.0 * mt * t * p1 + t * t * p2
    }

    // Compute stationary point t for one dimension,
    // if it lies in (0,1) (the range of t for the curve)
    fn stationary_t(p0: f32, p1: f32, p2: f32) -> Option<f32> {
        // B'(t) = 2(1-t)(p1-p0) + 2t(p2-p1)
        // B'(t) == 0  when  t = (p0-p1) / (p0-2p1+p2)
        let denom = p0 - 2.0 * p1 + p2;
        if denom.abs() < 1e-6 {
            None
        } else {
            let t = (p0 - p1) / denom;
            if t > 0.0 && t < 1.0 {
                Some(t)
            } else {
                None
            }
        }
    }

    [
        stationary_t(start.0, cp.0, end.0),
        stationary_t(start.1, cp.1, end.1),
    ]
    .into_iter()
    .flatten()
    .map(|t| {
        (
            bezier(t, start.0, cp.0, end.0),
            bezier(t, start.1, cp.1, end.1),
        )
    })
    .collect()
}

fn cubic_extrema(
    start: (f32, f32),
    cp1: (f32, f32),
    cp2: (f32, f32),
    end: (f32, f32),
) -> Vec<(f32, f32)> {
    fn cubic(t: f32, p0: f32, p1: f32, p2: f32, p3: f32) -> f32 {
        let mt = 1.0 - t;
        mt * mt * mt * p0 + 3.0 * mt * mt * t * p1 + 3.0 * mt * t * t * p2 + t * t * t * p3
    }

    fn stationary_ts(p0: f32, p1: f32, p2: f32, p3: f32) -> Vec<f32> {
        // Derivative of cubic Bezier: B'(t) = 3(1-t)^2 * (p1-p0) + 6(1-t)t(p2-p1) + 3t^2 * (p3-p2)
        // Rearranging to standard form: at^2 + bt + c = 0
        let a = 3.0 * (p3 - 3.0 * p2 + 3.0 * p1 - p0);
        let b = 6.0 * (p2 - 2.0 * p1 + p0);
        let c = 3.0 * (p1 - p0);

        let mut ts = vec![];
        if a.abs() < 1e-6 {
            // Linear case: bt + c = 0
            if b.abs() >= 1e-6 {
                let t = -c / b;
                if t > 0.0 && t < 1.0 {
                    ts.push(t);
                }
            }
        } else {
            // Quadratic case
            let disc = b * b - 4.0 * a * c;
            if disc >= 0.0 {
                let sqrt_disc = disc.sqrt();
                let inv_2a = 1.0 / (2.0 * a);
                let t1 = (-b + sqrt_disc) * inv_2a;
                let t2 = (-b - sqrt_disc) * inv_2a;

                if t1 > 0.0 && t1 < 1.0 {
                    ts.push(t1);
                }
                if t2 > 0.0 && t2 < 1.0 {
                    ts.push(t2);
                }
            }
        }
        ts
    }

    let mut all_t = Vec::new();
    all_t.extend(stationary_ts(start.0, cp1.0, cp2.0, end.0));
    all_t.extend(stationary_ts(start.1, cp1.1, cp2.1, end.1));

    all_t
        .into_iter()
        .map(|t| {
            (
                cubic(t, start.0, cp1.0, cp2.0, end.0),
                cubic(t, start.1, cp1.1, cp2.1, end.1),
            )
        })
        .collect()
}

fn arc_extrema(
    start: (f32, f32),
    rx: f32,
    ry: f32,
    x_axis_rotation: f32,
    large_arc_flag: bool,
    sweep_flag: bool,
    end: (f32, f32),
) -> Vec<(f32, f32)> {
    const EPSILON: f32 = 1e-6;
    use std::f32::consts::PI;

    if rx.abs() < EPSILON
        || ry.abs() < EPSILON
        || (start.0 - end.0).hypot(start.1 - end.1) < EPSILON
    {
        return vec![];
    }

    let (rx, ry) = (rx.abs(), ry.abs());
    let phi = x_axis_rotation.to_radians();

    // Scale radii if required to reach the endpoint
    // https://www.w3.org/TR/SVG2/implnote.html#ArcCorrectionOutOfRangeRadii
    // this duplicates some code in endpoint_to_center, but the scaled values
    // are needed later in this function.
    let cos_phi = phi.cos();
    let sin_phi = phi.sin();
    let x1_prime = cos_phi * (start.0 - end.0) / 2.0 + sin_phi * (start.1 - end.1) / 2.0;
    let y1_prime = -sin_phi * (start.0 - end.0) / 2.0 + cos_phi * (start.1 - end.1) / 2.0;

    let lambda = (x1_prime * x1_prime) / (rx * rx) + (y1_prime * y1_prime) / (ry * ry);
    let (rx, ry) = if lambda > 1.0 {
        (rx * lambda.sqrt(), ry * lambda.sqrt())
    } else {
        (rx, ry)
    };

    // Convert to center form
    let (center, start_angle, sweep_angle) =
        endpoint_to_center(start, end, rx, ry, phi, large_arc_flag, sweep_flag);

    let mut angles = Vec::new();

    // Handle axis-aligned case specially for numerical stability
    if phi.abs() < EPSILON || (phi - PI).abs() < EPSILON {
        // For axis-aligned ellipses, extrema are at 0°, 90°, 180°, 270°
        angles.extend([0.0, PI / 2.0, PI, 3.0 * PI / 2.0]);
    } else if (phi - PI / 2.0).abs() < EPSILON || (phi - 3.0 * PI / 2.0).abs() < EPSILON {
        // For 90° rotated ellipses, extrema are also at cardinal directions
        angles.extend([0.0, PI / 2.0, PI, 3.0 * PI / 2.0]);
    } else {
        // General rotated case - solve derivative equations
        // dx/dt = 0: tan(t) = -ry*sin(phi) / (rx*cos(phi))
        let tan_t = -ry * phi.sin() / (rx * phi.cos());
        angles.extend([tan_t.atan(), tan_t.atan() + PI]);

        // dy/dt = 0: tan(t) = ry*cos(phi) / (rx*sin(phi))
        let tan_t = ry * phi.cos() / (rx * phi.sin());
        angles.extend([tan_t.atan(), tan_t.atan() + PI]);
    }

    fn fround(v: f32) -> f32 {
        // Round to avoid floating point precision issues
        const SCALE: f32 = 65536.0;
        (v * SCALE).round() / SCALE
    }

    // Filter to only angles within the arc sweep and compute points
    angles
        .into_iter()
        .filter_map(|angle| {
            if angle_in_sweep(angle, start_angle, sweep_angle) {
                Some(ellipse_point(center, rx, ry, phi, angle))
            } else {
                None
            }
        })
        .map(|(x, y)| (fround(x), fround(y)))
        .collect()
}

// Implements https://www.w3.org/TR/SVG2/implnote.html#ArcConversionEndpointToCenter
fn endpoint_to_center(
    start: (f32, f32),
    end: (f32, f32),
    rx: f32,
    ry: f32,
    phi: f32,
    large_arc_flag: bool,
    sweep_flag: bool,
) -> ((f32, f32), f32, f32) {
    use std::f32::consts::PI;

    let (x1, y1) = start;
    let (x2, y2) = end;
    let cos_phi = phi.cos();
    let sin_phi = phi.sin();

    // Step 1: Compute (x1', y1')
    let x1_prime = cos_phi * (x1 - x2) / 2.0 + sin_phi * (y1 - y2) / 2.0;
    let y1_prime = -sin_phi * (x1 - x2) / 2.0 + cos_phi * (y1 - y2) / 2.0;

    // Step 2: Compute (cx', cy')
    let sign = if large_arc_flag != sweep_flag {
        1.0
    } else {
        -1.0
    };
    let coeff_sq = ((rx * ry).powi(2) - (rx * y1_prime).powi(2) - (ry * x1_prime).powi(2))
        / ((rx * y1_prime).powi(2) + (ry * x1_prime).powi(2));
    let coeff = sign * coeff_sq.max(0.0).sqrt();
    let cx_prime = coeff * (rx * y1_prime) / ry;
    let cy_prime = coeff * -(ry * x1_prime) / rx;

    // Step 3: Compute (cx, cy) from (cx', cy')
    let cx = cos_phi * cx_prime - sin_phi * cy_prime + (x1 + x2) / 2.0;
    let cy = sin_phi * cx_prime + cos_phi * cy_prime + (y1 + y2) / 2.0;

    // Step 4: Compute theta1 and delta_theta angles
    fn angle_between(ux: f32, uy: f32, vx: f32, vy: f32) -> f32 {
        let dot = ux * vx + uy * vy;
        let det = ux * vy - uy * vx;
        // atan2 is more robust than arccos approach from the spec
        det.atan2(dot)
    }

    // theta1 = angle((1,0), ((x1'-cx')/rx, (y1'-cy')/ry))
    let theta1 = angle_between(
        1.0,
        0.0,
        (x1_prime - cx_prime) / rx,
        (y1_prime - cy_prime) / ry,
    );

    // delta_theta = angle(((x1'-cx')/rx, (y1'-cy')/ry), ((-x1'-cx')/rx, (-y1'-cy')/ry))
    let mut delta_theta = angle_between(
        (x1_prime - cx_prime) / rx,
        (y1_prime - cy_prime) / ry,
        (-x1_prime - cx_prime) / rx,
        (-y1_prime - cy_prime) / ry,
    );

    // Adjust delta_theta according to sweep flag
    if sweep_flag && delta_theta < 0.0 {
        delta_theta += 2.0 * PI;
    } else if !sweep_flag && delta_theta > 0.0 {
        delta_theta -= 2.0 * PI;
    }

    ((cx, cy), theta1, delta_theta)
}

fn ellipse_point(center: (f32, f32), rx: f32, ry: f32, phi: f32, t: f32) -> (f32, f32) {
    let (cos_t, sin_t) = (t.cos(), t.sin());
    let (cos_phi, sin_phi) = (phi.cos(), phi.sin());

    (
        center.0 + rx * cos_t * cos_phi - ry * sin_t * sin_phi,
        center.1 + rx * cos_t * sin_phi + ry * sin_t * cos_phi,
    )
}

fn angle_in_sweep(angle: f32, start_angle: f32, sweep_angle: f32) -> bool {
    const EPSILON: f32 = 1e-6;
    use std::f32::consts::PI;

    if sweep_angle.abs() < EPSILON {
        return false;
    }

    // Normalize to be relative to start_angle in [-PI, PI] range
    let delta = ((angle - start_angle + PI) % (2.0 * PI)) - PI;

    if sweep_angle > 0.0 {
        // Counter-clockwise sweep
        let normalized_delta = if delta < 0.0 { delta + 2.0 * PI } else { delta };
        normalized_delta <= sweep_angle
    } else {
        // Clockwise sweep
        let normalized_delta = if delta > 0.0 { delta - 2.0 * PI } else { delta };
        normalized_delta >= sweep_angle
    }
}

pub fn path_bbox(element: &SvgElement) -> Result<Option<BoundingBox>> {
    if let Some(path_data) = element.get_attr("d") {
        let mut pp = PathParser::new(path_data);
        pp.evaluate()?;
        Ok(pp.get_bbox())
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ps_number() {
        let mut ps = SvgPathSyntax::new("123 4.5  -9.25");
        ps.skip_whitespace();
        assert_eq!(ps.read_number().unwrap(), 123.);
        ps.skip_whitespace();
        assert_eq!(ps.read_number().unwrap(), 4.5);
        ps.skip_whitespace();
        assert_eq!(ps.read_number().unwrap(), -9.25);

        // should read as little as needed to allow valid parsing,
        // so numbers can be squished together providing the result
        // is unambiguous. See https://www.w3.org/TR/SVG11/paths.html#PathDataBNF
        let mut ps = SvgPathSyntax::new("123-4.5.25+5");
        assert_eq!(ps.read_number().unwrap(), 123.);
        assert_eq!(ps.read_number().unwrap(), -4.5);
        assert_eq!(ps.read_number().unwrap(), 0.25);
        assert_eq!(ps.read_number().unwrap(), 5.);

        // should support exponents
        let mut ps = SvgPathSyntax::new("1e3 -2E-2 +3.5e+2");
        assert_eq!(ps.read_number().unwrap(), 1e3);
        assert_eq!(ps.read_number().unwrap(), -2e-2);
        assert_eq!(ps.read_number().unwrap(), 3.5e+2);
        // ... and without spaces; '1e3.5' is '1e3' followed by '.5'
        let mut ps = SvgPathSyntax::new("1e3.5-2E-2+3.5e+2");
        assert_eq!(ps.read_number().unwrap(), 1e3);
        assert_eq!(ps.read_number().unwrap(), 0.5);
        assert_eq!(ps.read_number().unwrap(), -2e-2);
        assert_eq!(ps.read_number().unwrap(), 3.5e+2);
    }

    #[test]
    fn test_ps_flag() {
        let mut ps = SvgPathSyntax::new("0 1,1 0");
        assert_eq!(ps.read_flag().unwrap(), 0);
        assert_eq!(ps.read_flag().unwrap(), 1);
        assert_eq!(ps.read_flag().unwrap(), 1);
        assert_eq!(ps.read_flag().unwrap(), 0);

        // whitespace is not required around flags
        let mut ps = SvgPathSyntax::new("01");
        assert_eq!(ps.read_flag().unwrap(), 0);
        assert_eq!(ps.read_flag().unwrap(), 1);

        // only '0' and '1' are valid flags
        let mut ps = SvgPathSyntax::new("2");
        assert!(ps.read_flag().is_err());
        let mut ps = SvgPathSyntax::new("1.0");
        // will read the '1' as a flag, leaving '.0'
        assert_eq!(ps.read_flag().unwrap(), 1);
        assert!(ps.read_flag().is_err());
    }

    #[test]
    fn test_ps_coord() {
        let mut ps = SvgPathSyntax::new("123 456");
        assert_eq!(ps.read_coord().unwrap(), (123., 456.));

        let mut ps = SvgPathSyntax::new("123,456");
        assert_eq!(ps.read_coord().unwrap(), (123., 456.));

        let mut ps = SvgPathSyntax::new("123 ,   456");
        assert_eq!(ps.read_coord().unwrap(), (123., 456.));

        // Example from https://www.w3.org/TR/SVG11/paths.html#PathDataBNF
        // 'for the string "M 0.6.5" … the first coordinate will be "0.6" and
        // the second coordinate will be ".5".'
        let mut ps = SvgPathSyntax::new("0.6.5");
        assert_eq!(ps.read_coord().unwrap(), (0.6, 0.5));
    }

    #[test]
    fn test_pp_move() {
        let mut pp = PathParser::new("M10 20");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((10., 20.)));

        // if the first command is 'm' (relative moveto) it is treated
        // as an absolute moveto.
        let mut pp = PathParser::new("m10 20");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((10., 20.)));

        // There can be multiple coordinates, in which case subsequent ones
        // are implicit 'line-to' coordinates
        let mut pp = PathParser::new("M10 20 100 200");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((100., 200.)));
        assert!(pp.tokens.at_end());

        // There can be multiple coordinates, in which case subsequent ones
        // are implicit 'line-to' coordinates
        let mut pp = PathParser::new("m10 20 100 200");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((110., 220.)));
        assert!(pp.tokens.at_end());

        // Example from spec - grammar section.
        let mut pp = PathParser::new("M 0.6.5");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((0.6, 0.5)));
        assert!(pp.tokens.at_end());

        //
        // Same again as above, but with lineto (L / l) this time.
        //
        let mut pp = PathParser::new("L10 20");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((10., 20.)));

        // if the first command is 'm' (relative moveto) it is treated
        // as an absolute moveto.
        let mut pp = PathParser::new("l10 20");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((10., 20.)));

        // There can be multiple coordinates, in which case subsequent ones
        // are implicit 'line-to' coordinates
        let mut pp = PathParser::new("L10 20 100 200");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((100., 200.)));
        assert!(pp.tokens.at_end());

        // There can be multiple coordinates, in which case subsequent ones
        // are implicit 'line-to' coordinates
        let mut pp = PathParser::new("l10 20 100 200");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((110., 220.)));
        assert!(pp.tokens.at_end());

        //
        // Horizontal lines
        //
        let mut pp = PathParser::new("H 10");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((10., 0.)));
        assert!(pp.tokens.at_end());

        let mut pp = PathParser::new("H 10 80 30");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((30., 0.)));
        assert!(pp.tokens.at_end());

        let mut pp = PathParser::new("h 10");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((10., 0.)));
        assert!(pp.tokens.at_end());

        let mut pp = PathParser::new("h 10 80 30");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((120., 0.)));
        assert!(pp.tokens.at_end());

        //
        // Vertical lines
        //
        let mut pp = PathParser::new("V 10");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((0., 10.)));
        assert!(pp.tokens.at_end());

        let mut pp = PathParser::new("V 10 80 30");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((0., 30.)));
        assert!(pp.tokens.at_end());

        let mut pp = PathParser::new("v 10");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((0., 10.)));
        assert!(pp.tokens.at_end());

        let mut pp = PathParser::new("v 10 80 30");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((0., 120.)));
        assert!(pp.tokens.at_end());
    }

    #[test]
    fn test_pp_bbox() {
        let mut pp = PathParser::new("M10 20 100 200 200 150");
        pp.evaluate().unwrap();
        assert_eq!(pp.get_bbox(), Some(BoundingBox::new(10., 20., 200., 200.)));

        let mut pp = PathParser::new("M10 20 M100 200 M200 150");
        pp.evaluate().unwrap();
        assert_eq!(pp.get_bbox(), Some(BoundingBox::new(10., 20., 200., 200.)));

        let mut pp = PathParser::new("M10 20 m100 200 m-1000 150");
        pp.evaluate().unwrap();
        assert_eq!(
            pp.get_bbox(),
            Some(BoundingBox::new(-890., 20., 110., 370.))
        );
    }

    #[test]
    fn test_bezier_curve_bbox() {
        for (pd, exp) in [
            // Simple cubic curve with extrema
            ("M 0 0 c 10 20 30 20 40 0", [0., 0., 40., 15.]),
            // Multiple cubic curves - simple horizontal curves
            (
                "M 10 10 c 0 5 5 5 10 0 c 0 -5 5 -5 10 0",
                [10., 6.25, 30., 13.75],
            ),
            // Absolute cubic with clear extrema - symmetric arch
            ("M 0 0 C 0 40 40 40 40 0", [0., 0., 40., 30.]),
            // Simple S-curve
            ("M 0 0 C 20 0 20 20 40 20", [0., 0., 40., 20.]),
            // Smooth cubic with simple reflection
            ("M 0 0 C 10 0 20 20 30 20 s 20 0 30 0", [0., 0., 60., 20.]),
            // Smooth cubic without previous cubic (degenerate case)
            ("M 20 20 s 10 9 20 0", [20., 20., 40., 24.]),
            // Simple absolute smooth cubic
            (
                "M 0 0 C 0 20 20 20 20 0 S -15 -20 30 0",
                [0., -15., 30., 15.],
            ),
            // S command without previous cubic
            ("M 10 10 S 20 28 30 10", [10., 10., 30., 18.]),
            // Simple quadratic arch
            ("M 0 0 q 20 40 40 0", [0., 0., 40., 20.]),
            // Quadratic with no extrema (straight line case)
            ("M 10 10 q 10 10 20 20", [10., 10., 30., 30.]),
            // Absolute quadratic arch
            ("M 0 0 Q 20 40 40 0", [0., 0., 40., 20.]),
            // Quadratic dipping below
            ("M 0 20 Q 20 0 40 20", [0., 10., 40., 20.]),
            // Smooth quadratic following Q - symmetric waves
            ("M 0 0 Q 10 20 20 0 t 20 0", [0., -10., 40., 10.]),
            // t command without previous quadratic (degenerate)
            ("M 10 10 t 20 0", [10., 10., 30., 10.]),
            // Absolute smooth quadratic - symmetric arches
            ("M 0 0 Q 20 40 40 0 T 80 0", [0., -20., 80., 20.]),
            // T command without previous quadratic
            ("M 10 10 T 30 20", [10., 10., 30., 20.]),
            // oblique quadratic with different t values in x and y
            ("M 0 0 q 60 120 30 0", [0., 0., 40., 60.]),
        ] {
            let mut pp = PathParser::new(pd);
            pp.evaluate().unwrap();
            let exp_bbox = BoundingBox::new(exp[0], exp[1], exp[2], exp[3]);
            assert_eq!(pp.get_bbox(), Some(exp_bbox), "Failed for path: {pd}");
        }
    }

    #[test]
    fn test_arc_bbox() {
        for (pd, exp) in [
            // Simple semicircle arc - top half
            ("M 0 0 A 10 10 0 0 1 20 0", [0., -10., 20., 0.]),
            // Simple semicircle arc - bottom half
            ("M 0 0 A 10 10 0 0 0 20 0", [0., 0., 20., 10.]),
            // Quarter circle arc - first quadrant
            ("M 10 0 A 10 10 0 0 1 20 10", [10., 0., 20., 10.]),
            // Quarter circle arc - includes extrema at top
            ("M 0 10 A 10 10 0 0 1 10 0", [0., 0., 10., 10.]),
            // Small arc that doesn't include any extrema
            // ("M 5 0 A 10 10 0 0 1 15 0", [5., -1.34, 15., 0.]),
            // Elliptical arc - horizontal ellipse
            ("M 0 0 A 20 10 0 0 1 40 0", [0., -10., 40., 0.]),
            // Elliptical arc - vertical ellipse
            ("M 0 0 A 10 20 0 0 1 20 0", [0., -20., 20., 0.]),
            // Rotated ellipse - 45 degrees
            ("M 0 0 A 10 5 45 0 1 10 10", [0., 0., 10., 10.]),
            // Relative arc - simple quarter circle
            ("M 10 10 a 5 5 0 0 1 5 5", [10., 10., 15., 15.]),
            // Relative arc - semicircle
            ("M 5 5 a 10 10 0 0 1 20 0", [5., -5., 25., 5.]),
            // Large arc flag difference - same endpoints, different sweep
            ("M 0 0 A 10 10 0 1 1 20 0", [0., -10., 20., 0.]),
            // Sweep flag difference - clockwise vs counterclockwise
            ("M 0 0 A 10 10 0 0 0 20 0", [0., 0., 20., 10.]),
            // Full circle (start and end same) - should be degenerate
            ("M 10 10 A 5 5 0 0 1 10 10", [10., 10., 10., 10.]),
            // Nearly straight line arc
            // ("M 0 0 A 100 100 0 0 1 1 0", [0., 0., 1., 0.]),
            // Arc with very different radii
            ("M 0 0 A 50 2 0 0 1 100 0", [0., -2., 100., 0.]),
            // If the radii are too small they are scaled up;
            // check the bbox is too.
            ("M 0 0 A 10 5 45 1 1 100 0", [-12.5, -62.5, 100., 0.]),
        ] {
            let mut pp = PathParser::new(pd);
            pp.evaluate().unwrap();
            let exp_bbox = BoundingBox::new(exp[0], exp[1], exp[2], exp[3]);
            assert_eq!(pp.get_bbox(), Some(exp_bbox), "Failed for path: {pd}");
        }
    }

    #[test]
    fn test_multiple_subpath_bbox() {
        let mut pp = PathParser::new("m0 0 20 20h-10zm 5 -10h20v-10zm20 10l20 30");
        pp.evaluate().unwrap();
        assert_eq!(pp.get_bbox(), Some(BoundingBox::new(0., -20., 45., 30.)));
    }
}
