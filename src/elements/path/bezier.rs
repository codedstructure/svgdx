use super::Vec2;
use super::sample::{sample_length, sample_point_at_ratio};
use super::syntax::{PathSyntax, SvgPathSyntax};
use crate::Result;

const EPSILON: f32 = 1e-6;
const QUADRATIC_SAMPLES: usize = 8;
const CUBIC_SAMPLES: usize = 12;

pub(super) struct CubicBezier {
    start: Vec2,
    cp1: Vec2,
    cp2: Vec2,
    end: Vec2,
}

impl CubicBezier {
    pub fn from_tokens(tokens: &mut SvgPathSyntax, start: Vec2, relative: bool) -> Result<Self> {
        // "(x1 y1 x2 y2 x y)+"
        let adjust = |p: Vec2| {
            if relative { start + p } else { p }
        };
        let cp1 = adjust(tokens.read_coord()?);
        let cp2 = adjust(tokens.read_coord()?);
        let end = adjust(tokens.read_coord()?);
        Ok(Self::new(start, cp1, cp2, end))
    }

    pub fn from_smooth_tokens(
        tokens: &mut SvgPathSyntax,
        start: Vec2,
        previous_cp2: Option<Vec2>,
        relative: bool,
    ) -> Result<Self> {
        // "(x2 y2 x y)+"
        let adjust = |p: Vec2| {
            if relative { start + p } else { p }
        };
        let cp2 = adjust(tokens.read_coord()?);
        let end = adjust(tokens.read_coord()?);
        // start and previous_cp2 are always absolute.
        let cp1 = reflect_control_point(start, previous_cp2);
        Ok(Self::new(start, cp1, cp2, end))
    }

    fn new(start: Vec2, cp1: Vec2, cp2: Vec2, end: Vec2) -> Self {
        Self {
            start,
            cp1,
            cp2,
            end,
        }
    }

    pub fn control_point_2(&self) -> Vec2 {
        self.cp2
    }

    pub fn end(&self) -> Vec2 {
        self.end
    }

    pub fn extrema(&self) -> Vec<Vec2> {
        let mut all_t = Vec::new();
        all_t.extend(cubic_stationary_ts(
            self.start.x,
            self.cp1.x,
            self.cp2.x,
            self.end.x,
        ));
        all_t.extend(cubic_stationary_ts(
            self.start.y,
            self.cp1.y,
            self.cp2.y,
            self.end.y,
        ));

        all_t.into_iter().map(|t| self.evaluate(t)).collect()
    }

    pub fn evaluate(&self, t: f32) -> Vec2 {
        Vec2::new(
            evaluate_cubic(self.start.x, self.cp1.x, self.cp2.x, self.end.x, t),
            evaluate_cubic(self.start.y, self.cp1.y, self.cp2.y, self.end.y, t),
        )
    }

    pub fn approx_length(&self) -> f32 {
        sample_length(CUBIC_SAMPLES, |t| self.evaluate(t))
    }

    pub fn point_at_ratio(&self, ratio: f32) -> Vec2 {
        sample_point_at_ratio(CUBIC_SAMPLES, ratio, |t| self.evaluate(t))
    }
}

pub(super) struct QuadraticBezier {
    start: Vec2,
    cp: Vec2,
    end: Vec2,
}

impl QuadraticBezier {
    pub fn from_tokens(tokens: &mut SvgPathSyntax, start: Vec2, relative: bool) -> Result<Self> {
        // "(x1 y1 x y)+"
        let adjust = |p: Vec2| {
            if relative { start + p } else { p }
        };
        let cp = adjust(tokens.read_coord()?);
        let end = adjust(tokens.read_coord()?);
        Ok(Self { start, cp, end })
    }

    pub fn from_smooth_tokens(
        tokens: &mut SvgPathSyntax,
        start: Vec2,
        previous_cp: Option<Vec2>,
        relative: bool,
    ) -> Result<Self> {
        // "(x y)+"
        let adjust = |p: Vec2| {
            if relative { start + p } else { p }
        };
        let end = adjust(tokens.read_coord()?);
        let cp = reflect_control_point(start, previous_cp);
        Ok(Self { start, cp, end })
    }

    pub fn control_point(&self) -> Vec2 {
        self.cp
    }

    pub fn end(&self) -> Vec2 {
        self.end
    }

    pub fn extrema(&self) -> Vec<Vec2> {
        [
            quadratic_stationary_t(self.start.x, self.cp.x, self.end.x),
            quadratic_stationary_t(self.start.y, self.cp.y, self.end.y),
        ]
        .into_iter()
        .flatten()
        .map(|t| self.evaluate(t))
        .collect()
    }

    pub fn evaluate(&self, t: f32) -> Vec2 {
        Vec2::new(
            evaluate_quadratic(self.start.x, self.cp.x, self.end.x, t),
            evaluate_quadratic(self.start.y, self.cp.y, self.end.y, t),
        )
    }

    pub fn approx_length(&self) -> f32 {
        sample_length(QUADRATIC_SAMPLES, |t| self.evaluate(t))
    }

    pub fn point_at_ratio(&self, ratio: f32) -> Vec2 {
        sample_point_at_ratio(QUADRATIC_SAMPLES, ratio, |t| self.evaluate(t))
    }
}

fn reflect_control_point(start: Vec2, previous_cp: Option<Vec2>) -> Vec2 {
    if let Some(prev_cp) = previous_cp {
        2. * start - prev_cp
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
