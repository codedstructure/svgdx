use super::Vec2;
use super::sample::{sample_length, sample_point_at_ratio};
use super::syntax::{PathSyntax, SvgPathSyntax};
use crate::Result;

use std::f32::consts::PI;

const EPSILON: f32 = 1e-6;
const ARC_SAMPLES: usize = 16;

pub(super) struct Arc {
    start: Vec2,
    rx: f32,
    ry: f32,
    x_axis_rotation: f32,
    large_arc_flag: bool,
    sweep_flag: bool,
    end: Vec2,
}

impl Arc {
    pub fn from_tokens(tokens: &mut SvgPathSyntax, start: Vec2, relative: bool) -> Result<Self> {
        // "(rx ry x-axis-rotation large-arc-flag sweep-flag x y)+"
        let rx = tokens.read_non_negative()?;
        let ry = tokens.read_non_negative()?;
        let x_axis_rotation = tokens.read_number()?;
        let large_arc_flag = tokens.read_flag()? != 0;
        let sweep_flag = tokens.read_flag()? != 0;
        let end = tokens.read_coord()?;

        let end = if relative { start + end } else { end };

        let mut ap = Arc {
            start,
            rx,
            ry,
            x_axis_rotation,
            large_arc_flag,
            sweep_flag,
            end,
        };

        ap.normalize_radii();
        Ok(ap)
    }

    pub fn end(&self) -> Vec2 {
        self.end
    }

    fn normalize_radii(&mut self) {
        let (rx, ry) = (self.rx.abs(), self.ry.abs());

        let phi = self.x_axis_rotation.to_radians();
        // SVG requires scaling radii up when the requested ellipse is too small to
        // connect the given endpoints; see SVG 2 implnote B.2.5.
        // https://www.w3.org/TR/SVG2/implnote.html#ArcCorrectionOutOfRangeRadii
        let (sin_phi, cos_phi) = phi.sin_cos();

        // "translation which places the origin at the midpoint"
        let inv_midpoint = (self.start - self.end) / 2.;
        // "rotation to line up the coordinate axes with the axes of the ellipse."
        let x1_prime = cos_phi * inv_midpoint.x + sin_phi * inv_midpoint.y;
        let y1_prime = -sin_phi * inv_midpoint.x + cos_phi * inv_midpoint.y;

        let lambda = (x1_prime * x1_prime) / (rx * rx) + (y1_prime * y1_prime) / (ry * ry);
        if lambda > 1.0 {
            let scale = lambda.sqrt();
            self.rx *= scale;
            self.ry *= scale;
        }
    }

    pub fn extrema(&self) -> Vec<Vec2> {
        if self.rx.abs() < EPSILON
            || self.ry.abs() < EPSILON
            || self.start.distance(self.end) < EPSILON
        {
            return vec![];
        }

        let phi = self.x_axis_rotation.to_radians();
        // let (rx, ry) = normalize_arc_radii(self.start, self.end, self.rx, self.ry, phi);

        // Convert to center form
        let (center, start_angle, sweep_angle) = self.endpoint_to_center();
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
            let tan_t = -self.ry * phi.sin() / (self.rx * phi.cos());
            angles.extend([tan_t.atan(), tan_t.atan() + PI]);

            // dy/dt = 0: tan(t) = ry*cos(phi) / (rx*sin(phi))
            let tan_t = self.ry * phi.cos() / (self.rx * phi.sin());
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
                    Some(ellipse_point(center, self.rx, self.ry, phi, angle))
                } else {
                    None
                }
            })
            .map(|v| v.apply(fround))
            .collect()
    }

    pub fn evaluate(&self, t: f32) -> Vec2 {
        if self.start.distance(self.end) < EPSILON {
            return self.start;
        }

        if self.rx.abs() < EPSILON || self.ry.abs() < EPSILON {
            return self.start + t * (self.end - self.start);
        }

        let (center, start_angle, sweep_angle) = self.endpoint_to_center();
        let phi = self.x_axis_rotation.to_radians();
        ellipse_point(center, self.rx, self.ry, phi, start_angle + sweep_angle * t)
    }

    pub fn approx_length(&self) -> f32 {
        sample_length(ARC_SAMPLES, |t| self.evaluate(t))
    }

    pub fn point_at_ratio(&self, ratio: f32) -> Vec2 {
        sample_point_at_ratio(ARC_SAMPLES, ratio, |t| self.evaluate(t))
    }

    // Implements https://www.w3.org/TR/SVG2/implnote.html#ArcConversionEndpointToCenter
    fn endpoint_to_center(&self) -> (Vec2, f32, f32) {
        let phi = self.x_axis_rotation.to_radians();

        let (x1, y1) = (self.start.x, self.start.y);
        let (x2, y2) = (self.end.x, self.end.y);
        let cos_phi = phi.cos();
        let sin_phi = phi.sin();

        // Step 1: Compute (x1', y1')
        let x1_prime = cos_phi * (x1 - x2) / 2.0 + sin_phi * (y1 - y2) / 2.0;
        let y1_prime = -sin_phi * (x1 - x2) / 2.0 + cos_phi * (y1 - y2) / 2.0;

        // Step 2: Compute (cx', cy')
        let sign = if self.large_arc_flag != self.sweep_flag {
            1.0
        } else {
            -1.0
        };
        let (rx, ry) = (self.rx, self.ry);
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
        if self.sweep_flag && delta_theta < 0.0 {
            delta_theta += 2.0 * PI;
        } else if !self.sweep_flag && delta_theta > 0.0 {
            delta_theta -= 2.0 * PI;
        }

        (Vec2::new(cx, cy), theta1, delta_theta)
    }
}

fn ellipse_point(center: Vec2, rx: f32, ry: f32, phi: f32, t: f32) -> Vec2 {
    let (cos_t, sin_t) = (t.cos(), t.sin());
    let (cos_phi, sin_phi) = (phi.cos(), phi.sin());

    Vec2::new(
        center.x + rx * cos_t * cos_phi - ry * sin_t * sin_phi,
        center.y + rx * cos_t * sin_phi + ry * sin_t * cos_phi,
    )
}

fn angle_in_sweep(angle: f32, start_angle: f32, sweep_angle: f32) -> bool {
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
