use std::str::FromStr;

use crate::errors::{Result, SvgdxError};
use crate::position::BoundingBox;
use crate::types::strp;

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
                "Unknown transform type: {name}"
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

impl TransformAttr {
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
                _ => (),
            }
        }

        result
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
    fn test_transform_apply() {
        let bbox = BoundingBox::new(0., 0., 10., 10.);

        let t: TransformAttr = "translate(10,20)".parse().unwrap();
        assert_eq!(t.apply(&bbox), BoundingBox::new(10., 20., 20., 30.));

        let t: TransformAttr = "translate(10)".parse().unwrap();
        assert_eq!(t.apply(&bbox), BoundingBox::new(10., 0., 20., 10.));

        let t: TransformAttr = "scale(2)".parse().unwrap();
        assert_eq!(t.apply(&bbox), BoundingBox::new(0., 0., 20., 20.));

        let t: TransformAttr = "scale(2, 3)".parse().unwrap();
        assert_eq!(t.apply(&bbox), BoundingBox::new(0., 0., 20., 30.));
    }

    #[test]
    fn test_transform_apply_multiple() {
        let t: TransformAttr = "scale(2) translate(10,20)".parse().unwrap();
        let bbox = BoundingBox::new(0., 0., 10., 10.);
        assert_eq!(t.apply(&bbox), BoundingBox::new(20., 40., 40., 60.));
    }
}
