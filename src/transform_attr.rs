use std::str::FromStr;

use crate::errors::{Result, SvgdxError};
use crate::position::BoundingBox;
use crate::types::{fstr, strp};

impl BoundingBox {
    pub fn xfrm_scale(&self, sx: f32, sy: f32) -> Self {
        // scale about (0, 0) - not the center of the bbox
        Self {
            x1: self.x1 * sx,
            y1: self.y1 * sy,
            x2: self.x2 * sx,
            y2: self.y2 * sy,
        }
    }

    pub fn xfrm_translate(&self, dx: f32, dy: f32) -> Self {
        Self {
            x1: self.x1 + dx,
            y1: self.y1 + dy,
            x2: self.x2 + dx,
            y2: self.y2 + dy,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum TransformType {
    Translate(f32, f32),
    Scale(f32, f32),
    Rotate(f32, f32, f32),
    SkewX(f32),
    SkewY(f32),
    Matrix(f32, f32, f32, f32, f32, f32),
}

impl FromStr for TransformType {
    type Err = SvgdxError;

    fn from_str(value: &str) -> Result<Self> {
        let mut parts = value.splitn(2, '(');
        let name = parts
            .next()
            .ok_or_else(|| SvgdxError::ParseError("No transform name".to_owned()))?;
        let args = parts
            .next()
            .ok_or_else(|| SvgdxError::ParseError("No transform args".to_owned()))?
            .strip_suffix(')')
            .ok_or_else(|| SvgdxError::ParseError("No closing bracket".to_owned()))?
            .split(&[',', ' ', '\t', '\n', '\r'])
            .filter(|&v| !v.is_empty())
            .map(strp)
            .collect::<Result<Vec<_>>>()?;
        // See https://www.w3.org/TR/SVG11/coords.html#TransformAttribute
        Ok(match name.to_lowercase().as_str() {
            "translate" => {
                // "translate(<tx> [<ty>]), which specifies a translation by tx and ty. If <ty> is not provided, it is assumed to be zero."
                if args.len() == 1 {
                    TransformType::Translate(args[0], 0.)
                } else if args.len() == 2 {
                    TransformType::Translate(args[0], args[1])
                } else {
                    return Err(SvgdxError::ParseError(
                        "Invalid number of arguments for translate".to_string(),
                    ));
                }
            }
            "scale" => {
                // "scale(<sx> [<sy>]), which specifies a scale operation by sx and sy. If <sy> is not provided, it is assumed to be equal to <sx>."
                if args.len() == 1 {
                    TransformType::Scale(args[0], args[0])
                } else if args.len() == 2 {
                    TransformType::Scale(args[0], args[1])
                } else {
                    return Err(SvgdxError::ParseError(
                        "Invalid number of arguments for scale".to_string(),
                    ));
                }
            }
            "rotate" => {
                // "rotate(<rotate-angle> [<cx> <cy>]), which specifies a rotation by <rotate-angle> degrees about a given point."
                if args.len() == 1 {
                    TransformType::Rotate(args[0], 0., 0.)
                } else if args.len() == 3 {
                    TransformType::Rotate(args[0], args[1], args[2])
                } else {
                    return Err(SvgdxError::ParseError(
                        "Invalid number of arguments for rotate".to_string(),
                    ));
                }
            }
            "skewx" => {
                // "skewX(<skew-angle>), which specifies a skew transformation along the x-axis."
                if args.len() == 1 {
                    TransformType::SkewX(args[0])
                } else {
                    return Err(SvgdxError::ParseError(
                        "Invalid number of arguments for skewX".to_string(),
                    ));
                }
            }
            "skewy" => {
                // "skewY(<skew-angle>), which specifies a skew transformation along the y-axis."
                if args.len() == 1 {
                    TransformType::SkewY(args[0])
                } else {
                    return Err(SvgdxError::ParseError(
                        "Invalid number of arguments for skewY".to_string(),
                    ));
                }
            }
            "matrix" => {
                // "matrix(<a> <b> <c> <d> <e> <f>), which specifies a transformation in the form of a transformation matrix of six values."
                if args.len() == 6 {
                    TransformType::Matrix(args[0], args[1], args[2], args[3], args[4], args[5])
                } else {
                    return Err(SvgdxError::ParseError(
                        "Invalid number of arguments for matrix".to_string(),
                    ));
                }
            }
            _ => Err(SvgdxError::ParseError(format!(
                "Unknown transform type: '{name}'"
            )))?,
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct TransformAttr {
    transforms: Vec<TransformType>,
}

impl FromStr for TransformAttr {
    type Err = SvgdxError;

    fn from_str(value: &str) -> Result<Self> {
        let parts = value.split_inclusive(')').map(|v| v.trim());
        Ok(Self {
            transforms: parts
                .filter(|v| !v.is_empty())
                .map(|v| v.trim_start_matches([',', ' ', '\t', '\n', '\r']))
                .map(|v| v.parse())
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

impl std::fmt::Display for TransformType {
    fn fmt(&self, w: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            TransformType::Translate(tx, ty) => write!(w, "translate({}, {})", fstr(tx), fstr(ty)),
            TransformType::Scale(sx, sy) => write!(w, "scale({}, {})", fstr(sx), fstr(sy)),
            TransformType::Rotate(angle, cx, cy) => {
                write!(w, "rotate({}, {}, {})", fstr(angle), fstr(cx), fstr(cy))
            }
            TransformType::SkewX(angle) => write!(w, "skewX({})", fstr(angle)),
            TransformType::SkewY(angle) => write!(w, "skewY({})", fstr(angle)),
            TransformType::Matrix(a, b, c, d, e, f) => {
                write!(
                    w,
                    "matrix({}, {}, {}, {}, {}, {})",
                    fstr(a),
                    fstr(b),
                    fstr(c),
                    fstr(d),
                    fstr(e),
                    fstr(f)
                )
            }
        }
    }
}

impl std::fmt::Display for TransformAttr {
    fn fmt(&self, w: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (idx, transform) in self.transforms.iter().enumerate() {
            if idx != 0 {
                write!(w, " ")?;
            }
            write!(w, "{transform}")?;
        }
        Ok(())
    }
}

#[allow(dead_code)]
impl TransformAttr {
    pub fn new() -> Self {
        Self {
            transforms: Vec::new(),
        }
    }

    pub fn apply(&self, bbox: &BoundingBox) -> BoundingBox {
        let mut result = *bbox;

        for transform in self.transforms.iter().rev() {
            match *transform {
                TransformType::Translate(tx, ty) => {
                    result = result.xfrm_translate(tx, ty);
                }
                TransformType::Scale(sx, sy) => {
                    result = result.xfrm_scale(sx, sy);
                }
                TransformType::Rotate(angle, cx, cy) => {
                    let angle = (angle as f64).to_radians();
                    let (cx, cy) = (cx as f64, cy as f64);
                    let sin_a = angle.sin();
                    let cos_a = angle.cos();
                    let (x1, y1, x2, y2) = (
                        result.x1 as f64,
                        result.y1 as f64,
                        result.x2 as f64,
                        result.y2 as f64,
                    );
                    let corners = [(x1, y1), (x2, y1), (x1, y2), (x2, y2)];
                    let rot = corners
                        .iter()
                        .map(|&(x, y)| {
                            let rot_x = cx + (x - cx) * cos_a - (y - cy) * sin_a;
                            let rot_y = cy + (x - cx) * sin_a + (y - cy) * cos_a;
                            (rot_x, rot_y)
                        })
                        .collect::<Vec<_>>();
                    let min_x = rot[0].0.min(rot[1].0).min(rot[2].0).min(rot[3].0);
                    let max_x = rot[0].0.max(rot[1].0).max(rot[2].0).max(rot[3].0);
                    let min_y = rot[0].1.min(rot[1].1).min(rot[2].1).min(rot[3].1);
                    let max_y = rot[0].1.max(rot[1].1).max(rot[2].1).max(rot[3].1);
                    // deliberate down-sampling of floating point...
                    let min_x = ((16384.0 * min_x).round() / 16384.0) as f32;
                    let min_y = ((16384.0 * min_y).round() / 16384.0) as f32;
                    let max_x = ((16384.0 * max_x).round() / 16384.0) as f32;
                    let max_y = ((16384.0 * max_y).round() / 16384.0) as f32;

                    result = BoundingBox::new(min_x, min_y, max_x, max_y);
                }
                _ => (),
            }
        }

        result
    }

    pub fn translate(&mut self, tx: f32, ty: f32) {
        self.transforms.push(TransformType::Translate(tx, ty));
    }

    pub fn scale(&mut self, sx: f32, sy: f32) {
        self.transforms.push(TransformType::Scale(sx, sy));
    }

    pub fn rotate(&mut self, angle: f32) {
        self.transforms.push(TransformType::Rotate(angle, 0., 0.));
    }

    pub fn rotate_around(&mut self, angle: f32, cx: f32, cy: f32) {
        self.transforms.push(TransformType::Rotate(angle, cx, cy));
    }

    pub fn skewx(&mut self, angle: f32) {
        self.transforms.push(TransformType::SkewX(angle));
    }

    pub fn skewy(&mut self, angle: f32) {
        self.transforms.push(TransformType::SkewY(angle));
    }

    pub fn matrix(&mut self, a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) {
        self.transforms
            .push(TransformType::Matrix(a, b, c, d, e, f));
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_transform_parsing() {
        let t1: TransformAttr = "translate(10,20) scale(2) rotate(45)".parse().unwrap();
        let t2: TransformAttr = "translate( 10,   20), scale( 2, 2), rotate(  45  ,  0  ,  0  )"
            .parse()
            .unwrap();
        assert_eq!(t1, t2);

        let t: Result<TransformAttr> = "".parse();
        assert!(t.is_ok());
    }

    #[test]
    fn test_apply_translate() {
        let bbox = BoundingBox::new(0., 0., 10., 10.);

        let t: TransformAttr = "translate(10,20)".parse().unwrap();
        assert_eq!(t.apply(&bbox), BoundingBox::new(10., 20., 20., 30.));

        let t: TransformAttr = "translate(10)".parse().unwrap();
        assert_eq!(t.apply(&bbox), BoundingBox::new(10., 0., 20., 10.));
    }

    #[test]
    fn test_apply_scale() {
        let bbox = BoundingBox::new(0., 0., 10., 10.);
        let t: TransformAttr = "scale(2)".parse().unwrap();
        assert_eq!(t.apply(&bbox), BoundingBox::new(0., 0., 20., 20.));

        let t: TransformAttr = "scale(2, 3)".parse().unwrap();
        assert_eq!(t.apply(&bbox), BoundingBox::new(0., 0., 20., 30.));
    }

    #[test]
    fn test_apply_rotate() {
        let bbox = BoundingBox::new(-10., -5., 10., 5.);
        let t: TransformAttr = "rotate(90)".parse().unwrap();
        assert_eq!(t.apply(&bbox), BoundingBox::new(-5., -10., 5., 10.));

        let bbox = BoundingBox::new(0., 0., 10., 5.);
        let t: TransformAttr = "rotate(90)".parse().unwrap();
        assert_eq!(t.apply(&bbox), BoundingBox::new(-5., 0., 0., 10.));

        let bbox = BoundingBox::new(0., 0., 10., 5.);
        let t: TransformAttr = "rotate(90, 5, 2.5)".parse().unwrap();
        assert_eq!(t.apply(&bbox), BoundingBox::new(2.5, -2.5, 7.5, 7.5));

        let bbox = BoundingBox::new(-5., -5., 5., 5.);
        let t: TransformAttr = "rotate(45, 0, 0)".parse().unwrap();
        assert_eq!(
            t.apply(&bbox),
            BoundingBox::new(-7.071045, -7.071045, 7.071045, 7.071045)
        );
    }

    #[test]
    fn test_apply_multiple() {
        let t: TransformAttr = "scale(2) translate(10,20)".parse().unwrap();
        let bbox = BoundingBox::new(0., 0., 10., 10.);
        assert_eq!(t.apply(&bbox), BoundingBox::new(20., 40., 40., 60.));

        let t: TransformAttr = "rotate(90, 12, 1) translate(10,0)".parse().unwrap();
        let bbox = BoundingBox::new(0., 0., 4., 2.);
        assert_eq!(t.apply(&bbox), BoundingBox::new(11., -1., 13., 3.));

        let t: TransformAttr = "translate(10,0) rotate(90, 2, 1)".parse().unwrap();
        let bbox = BoundingBox::new(0., 0., 4., 2.);
        assert_eq!(t.apply(&bbox), BoundingBox::new(11., -1., 13., 3.));
    }

    #[test]
    fn test_transform_roundtrip() {
        let mut t = TransformAttr::new();
        t.translate(10., 20.);
        t.scale(2., 3.);
        t.rotate(45.);
        t.skewx(10.);
        t.skewy(20.);
        t.matrix(1., 0., 0., 1., 0., 0.);
        let t2 = t.to_string();
        let t3: TransformAttr = t2.parse().unwrap();
        assert_eq!(t, t3);
    }
}
