use super::syntax::{PathSyntax, SvgPathSyntax};
use crate::Result;

const EPSILON: f32 = 1e-6;

pub(super) struct CubicBezierParams {
    start: (f32, f32),
    cp1: (f32, f32),
    cp2: (f32, f32),
    end: (f32, f32),
}

impl CubicBezierParams {
    pub fn from_tokens(
        tokens: &mut SvgPathSyntax,
        start: (f32, f32),
        relative: bool,
    ) -> Result<Self> {
        let cp1 = tokens.read_coord()?;
        let cp2 = tokens.read_coord()?;
        let end = tokens.read_coord()?;
        Ok(Self::new(start, cp1, cp2, end, relative))
    }

    pub fn from_smooth_tokens(
        tokens: &mut SvgPathSyntax,
        start: (f32, f32),
        previous_cp2: Option<(f32, f32)>,
        relative: bool,
    ) -> Result<Self> {
        let cp2 = tokens.read_coord()?;
        let end = tokens.read_coord()?;
        let cp1 = reflect_control_point(start, previous_cp2);
        let (cp2, end) = if relative {
            (offset_point(start, cp2), offset_point(start, end))
        } else {
            (cp2, end)
        };
        Ok(Self {
            start,
            cp1,
            cp2,
            end,
        })
    }

    fn new(
        start: (f32, f32),
        cp1: (f32, f32),
        cp2: (f32, f32),
        end: (f32, f32),
        relative: bool,
    ) -> Self {
        let (cp1, cp2, end) = if relative {
            (
                offset_point(start, cp1),
                offset_point(start, cp2),
                offset_point(start, end),
            )
        } else {
            (cp1, cp2, end)
        };

        Self {
            start,
            cp1,
            cp2,
            end,
        }
    }

    pub fn control_point_2(&self) -> (f32, f32) {
        self.cp2
    }

    pub fn end(&self) -> (f32, f32) {
        self.end
    }

    pub fn extrema(&self) -> Vec<(f32, f32)> {
        let mut all_t = Vec::new();
        all_t.extend(cubic_stationary_ts(
            self.start.0,
            self.cp1.0,
            self.cp2.0,
            self.end.0,
        ));
        all_t.extend(cubic_stationary_ts(
            self.start.1,
            self.cp1.1,
            self.cp2.1,
            self.end.1,
        ));

        all_t.into_iter().map(|t| self.evaluate(t)).collect()
    }

    pub fn evaluate(&self, t: f32) -> (f32, f32) {
        (
            evaluate_cubic(self.start.0, self.cp1.0, self.cp2.0, self.end.0, t),
            evaluate_cubic(self.start.1, self.cp1.1, self.cp2.1, self.end.1, t),
        )
    }
}

pub(super) struct QuadraticBezierParams {
    start: (f32, f32),
    cp: (f32, f32),
    end: (f32, f32),
}

impl QuadraticBezierParams {
    pub fn from_tokens(
        tokens: &mut SvgPathSyntax,
        start: (f32, f32),
        relative: bool,
    ) -> Result<Self> {
        let cp = tokens.read_coord()?;
        let end = tokens.read_coord()?;
        Ok(Self::new(start, cp, end, relative))
    }

    pub fn from_smooth_tokens(
        tokens: &mut SvgPathSyntax,
        start: (f32, f32),
        previous_cp: Option<(f32, f32)>,
        relative: bool,
    ) -> Result<Self> {
        let end = tokens.read_coord()?;
        let cp = reflect_control_point(start, previous_cp);
        let end = if relative {
            offset_point(start, end)
        } else {
            end
        };
        Ok(Self { start, cp, end })
    }

    fn new(start: (f32, f32), cp: (f32, f32), end: (f32, f32), relative: bool) -> Self {
        let (cp, end) = if relative {
            (offset_point(start, cp), offset_point(start, end))
        } else {
            (cp, end)
        };

        Self { start, cp, end }
    }

    pub fn control_point(&self) -> (f32, f32) {
        self.cp
    }

    pub fn end(&self) -> (f32, f32) {
        self.end
    }

    pub fn extrema(&self) -> Vec<(f32, f32)> {
        [
            quadratic_stationary_t(self.start.0, self.cp.0, self.end.0),
            quadratic_stationary_t(self.start.1, self.cp.1, self.end.1),
        ]
        .into_iter()
        .flatten()
        .map(|t| self.evaluate(t))
        .collect()
    }

    pub fn evaluate(&self, t: f32) -> (f32, f32) {
        (
            evaluate_quadratic(self.start.0, self.cp.0, self.end.0, t),
            evaluate_quadratic(self.start.1, self.cp.1, self.end.1, t),
        )
    }
}

fn offset_point(origin: (f32, f32), delta: (f32, f32)) -> (f32, f32) {
    (origin.0 + delta.0, origin.1 + delta.1)
}

fn reflect_control_point(start: (f32, f32), previous_cp: Option<(f32, f32)>) -> (f32, f32) {
    if let Some((prev_cpx, prev_cpy)) = previous_cp {
        (2. * start.0 - prev_cpx, 2. * start.1 - prev_cpy)
    } else {
        start
    }
}

// Compute stationary point t for one dimension,
// if it lies in (0, 1) (the range of t for the curve)
fn quadratic_stationary_t(p0: f32, p1: f32, p2: f32) -> Option<f32> {
    // B'(t) = 2(1-t)(p1-p0) + 2t(p2-p1)
    // B'(t) == 0 when t = (p0-p1) / (p0-2p1+p2)
    let denom = p0 - 2.0 * p1 + p2;
    if denom.abs() < EPSILON {
        None
    } else {
        let t = (p0 - p1) / denom;
        if t > 0.0 && t < 1.0 { Some(t) } else { None }
    }
}

fn cubic_stationary_ts(p0: f32, p1: f32, p2: f32, p3: f32) -> Vec<f32> {
    // Derivative of cubic Bezier: B'(t) = 3(1-t)^2 * (p1-p0) + 6(1-t)t(p2-p1) + 3t^2 * (p3-p2)
    // Rearranging to standard form: at^2 + bt + c = 0
    let a = 3.0 * (p3 - 3.0 * p2 + 3.0 * p1 - p0);
    let b = 6.0 * (p2 - 2.0 * p1 + p0);
    let c = 3.0 * (p1 - p0);

    let mut ts = vec![];
    if a.abs() < EPSILON {
        // Linear case: bt + c = 0
        if b.abs() >= EPSILON {
            let t = -c / b;
            if t > 0.0 && t < 1.0 {
                ts.push(t);
            }
        }
    } else {
        // Quadratic case: at^2 + bt + c = 0
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

fn evaluate_cubic(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let mt = 1.0 - t;
    mt * mt * mt * p0 + 3.0 * mt * mt * t * p1 + 3.0 * mt * t * t * p2 + t * t * t * p3
}

fn evaluate_quadratic(p0: f32, p1: f32, p2: f32, t: f32) -> f32 {
    let mt = 1.0 - t;
    mt * mt * p0 + 2.0 * mt * t * p1 + t * t * p2
}
