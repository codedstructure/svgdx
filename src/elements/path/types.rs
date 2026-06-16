#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn distance(&self, other: Self) -> f32 {
        (other.x - self.x).hypot(other.y - self.y)
    }

    pub fn apply(&self, f: impl Fn(f32) -> f32) -> Self {
        Self {
            x: f(self.x),
            y: f(self.y),
        }
    }
}

impl From<(f32, f32)> for Vec2 {
    fn from((x, y): (f32, f32)) -> Self {
        Self { x, y }
    }
}

impl std::ops::Add<Vec2> for Vec2 {
    type Output = Self;

    fn add(self, rhs: Vec2) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl std::ops::Sub<Vec2> for Vec2 {
    type Output = Self;

    fn sub(self, rhs: Vec2) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl std::ops::Mul<Vec2> for f32 {
    type Output = Vec2;

    fn mul(self, rhs: Vec2) -> Self::Output {
        Vec2 {
            x: self * rhs.x,
            y: self * rhs.y,
        }
    }
}

impl std::ops::Mul<f32> for Vec2 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Vec2 {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl std::ops::Div<f32> for Vec2 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Vec2 {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Vec2;

    #[test]
    fn test_distance() {
        let a = Vec2::new(0., 0.);
        let b = Vec2::new(3., 4.);
        let c = Vec2::new(3., 7.);
        assert_eq!(a.distance(b), 5.);
        assert_eq!(b.distance(c), 3.);
    }

    #[test]
    fn test_addition() {
        let a = Vec2::new(1., 2.);
        let b = Vec2::new(3., 4.);
        assert_eq!(a + b, Vec2::new(4., 6.));
        assert_eq!(b + a, Vec2::new(4., 6.));
    }

    #[test]
    fn test_subtraction() {
        let a = Vec2::new(5., 7.);
        let b = Vec2::new(2., 3.);
        assert_eq!(a - b, Vec2::new(3., 4.));
        assert_eq!(b - a, Vec2::new(-3., -4.));
    }

    #[test]
    fn test_scalar_multiplication() {
        let a = Vec2::new(1., -2.);
        assert_eq!(3. * a, Vec2::new(3., -6.));
    }

    #[test]
    fn test_scalar_division() {
        let a = Vec2::new(4., 8.);
        assert_eq!(a / 2., Vec2::new(2., 4.));
    }
}
