use std::num::NonZeroU32;
use std::str::FromStr;

use crate::constants::{EDGESPEC_SEP, LOCSPEC_SEP};
use crate::errors::{Error, Result};
use crate::types::{attr_split, extract_elref, strp, ElRef};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub fn as_wh(&self) -> (f32, f32) {
        (self.width, self.height)
    }

    pub fn scale(&self, factor: f32) -> Self {
        Self::new(self.width * factor, self.height * factor)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum Length {
    Absolute(f32),
    Ratio(f32),
    Rational(i32, NonZeroU32), // numerator, denominator
}

impl Default for Length {
    fn default() -> Self {
        Self::Absolute(0.)
    }
}

impl Length {
    #[allow(dead_code)]
    pub const fn ratio(&self) -> Option<f32> {
        match self {
            Self::Ratio(result) => Some(*result),
            Self::Rational(numer, denom) => Some(*numer as f32 / denom.get() as f32),
            _ => None,
        }
    }

    pub const fn absolute(&self) -> Option<f32> {
        if let Self::Absolute(result) = self {
            Some(*result)
        } else {
            None
        }
    }

    /// Convert a `Length` to a value, taking a base value as input
    /// in case a ratio length is used.
    pub fn evaluate(&self, base: f32) -> f32 {
        match self {
            Self::Absolute(abs) => *abs,
            Self::Ratio(ratio) => base * ratio,
            Self::Rational(numer, denom) => base * (*numer as f32 / denom.get() as f32),
        }
    }

    /// Given a single value, update it (scale or addition) from
    /// the current Length value
    pub fn adjust(&self, value: f32) -> f32 {
        match self {
            Self::Absolute(abs) => value + abs,
            Self::Ratio(ratio) => value * ratio,
            Self::Rational(numer, denom) => value * (*numer as f32 / denom.get() as f32),
        }
    }

    /// Given a range, return a value (typically) in the range
    /// where a positive Absolute is 'from start', a negative Absolute
    /// is 'backwards from end' and Ratios scale as 0%=start, 100%=end
    /// but ratio values are not limited to 0..100 at either end.
    pub fn calc_offset(&self, start: f32, end: f32) -> f32 {
        match self {
            Self::Absolute(abs) => {
                let mult = if end < start { -1. } else { 1. };
                if abs < &0. {
                    // '+' here since abs is negative and
                    // we're going 'back' from the end.
                    end + abs * mult
                } else {
                    start + abs * mult
                }
            }
            Self::Ratio(ratio) => start + (end - start) * ratio,
            Self::Rational(numer, denom) => {
                start + (end - start) * (*numer as f32 / denom.get() as f32)
            }
        }
    }
}

pub fn strp_length(s: &str) -> Result<Length> {
    s.parse::<Length>()
}

impl FromStr for Length {
    type Err = Error;

    /// Parse a ratio (float or %age) to an f32
    /// Note this deliberately does not clamp to 0..1
    fn from_str(value: &str) -> Result<Self> {
        let value = value.trim();
        if let Some(pc) = value.strip_suffix('%') {
            Ok(Length::Ratio(strp(pc)? * 0.01))
        } else if let Some((numer, denom)) = value.split_once('/') {
            let numer = numer
                .parse()
                .map_err(|_| Error::Parse(format!("expected an integer numerator: '{value}'")))?;
            let denom = denom.parse().map_err(|_| {
                Error::Parse(format!("expected an integer denominator >= 1: '{value}'"))
            })?;
            Ok(Length::Rational(numer, denom))
        } else {
            Ok(Length::Absolute(strp(value)?))
        }
    }
}

impl DirSpec {
    pub fn to_locspec(self) -> LocSpec {
        match self {
            Self::InFront => LocSpec::Right,
            Self::Behind => LocSpec::Left,
            Self::Below => LocSpec::Bottom,
            Self::Above => LocSpec::Top,
        }
    }
}

/// `LocSpec` defines a location on a `BoundingBox`
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LocSpec {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
    Center,
    TopEdge(Length),
    RightEdge(Length),
    BottomEdge(Length),
    LeftEdge(Length),
}

/// `LocSpec` defines a location on a `SvgElement`
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ElementLoc {
    LocSpec(LocSpec),
    LineOffset(Length),
}

impl LocSpec {
    pub fn is_top(&self) -> bool {
        matches!(
            self,
            Self::Top | Self::TopLeft | Self::TopRight | Self::TopEdge(_)
        )
    }

    pub fn is_right(&self) -> bool {
        matches!(
            self,
            Self::Right | Self::TopRight | Self::BottomRight | Self::RightEdge(_)
        )
    }

    pub fn is_bottom(&self) -> bool {
        matches!(
            self,
            Self::Bottom | Self::BottomLeft | Self::BottomRight | Self::BottomEdge(_)
        )
    }

    pub fn is_left(&self) -> bool {
        matches!(
            self,
            Self::Left | Self::TopLeft | Self::BottomLeft | Self::LeftEdge(_)
        )
    }
}

impl FromStr for ElementLoc {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        if let Ok(ls) = LocSpec::from_str(value) {
            Ok(ElementLoc::LocSpec(ls))
        } else if let Some((edge, len)) = value.split_once(EDGESPEC_SEP) {
            let len = len.parse::<Length>()?;
            match edge {
                "" => Ok(ElementLoc::LineOffset(len)),
                _ => Err(Error::Parse(format!("invalid LocSpec format {value}"))),
            }
        } else {
            Err(Error::Parse(format!("invalid LocSpec format {value}")))
        }
    }
}

impl FromStr for LocSpec {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "tl" => Ok(Self::TopLeft),
            "t" => Ok(Self::Top),
            "tr" => Ok(Self::TopRight),
            "r" => Ok(Self::Right),
            "br" => Ok(Self::BottomRight),
            "b" => Ok(Self::Bottom),
            "bl" => Ok(Self::BottomLeft),
            "l" => Ok(Self::Left),
            "c" => Ok(Self::Center),
            s => {
                if let Some((edge, len)) = s.split_once(EDGESPEC_SEP) {
                    let len = len.parse::<Length>()?;
                    match edge {
                        "t" => Ok(Self::TopEdge(len)),
                        "r" => Ok(Self::RightEdge(len)),
                        "b" => Ok(Self::BottomEdge(len)),
                        "l" => Ok(Self::LeftEdge(len)),
                        _ => Err(Error::Parse(format!("invalid LocSpec format {value}"))),
                    }
                } else {
                    Err(Error::Parse(format!("invalid LocSpec format {value}")))
                }
            }
        }
    }
}

impl From<ScalarSpec> for LocSpec {
    fn from(value: ScalarSpec) -> Self {
        match value {
            ScalarSpec::Minx => Self::Left,
            ScalarSpec::Maxx => Self::Right,
            ScalarSpec::Cx => Self::Center,
            ScalarSpec::Miny => Self::Top,
            ScalarSpec::Maxy => Self::Bottom,
            ScalarSpec::Cy => Self::Center,
            ScalarSpec::Width => Self::Right,
            ScalarSpec::Radius | ScalarSpec::Rx => Self::Right,
            ScalarSpec::Height => Self::Bottom,
            ScalarSpec::Ry => Self::Bottom,
        }
    }
}

/// `ScalarSpec` defines a single value from a `BoundingBox`
///
/// These are the min and max x & y values, together with width and height.
#[derive(Clone, Copy)]
pub enum ScalarSpec {
    Minx,
    Maxx,
    Cx,
    Miny,
    Maxy,
    Cy,
    Radius,
    Width,
    Rx,
    Height,
    Ry,
}

impl ScalarSpec {
    pub fn is_size_scalar(&self) -> bool {
        matches!(
            self,
            Self::Width | Self::Height | Self::Rx | Self::Ry | Self::Radius
        )
    }
}

impl FromStr for ScalarSpec {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        // TODO: consider x1/x2/y1/y2: note that for e.g. a line it is
        // *not* required that the *attributes* x1 < x2 or y1 < y2.
        // Perhaps a separate 'attribute spec' concept is needed...
        match value {
            "x" | "x1" => Ok(Self::Minx),
            "y" | "y1" => Ok(Self::Miny),
            "cx" => Ok(Self::Cx),
            "x2" => Ok(Self::Maxx),
            "y2" => Ok(Self::Maxy),
            "cy" => Ok(Self::Cy),
            "r" => Ok(Self::Radius),
            "w" | "width" => Ok(Self::Width),
            "rx" => Ok(Self::Rx),
            "h" | "height" => Ok(Self::Height),
            "ry" => Ok(Self::Ry),
            _ => Err(Error::Parse(format!("Invalid ScalarSpec format {value}"))),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TrblLength {
    pub top: Length,
    pub right: Length,
    pub bottom: Length,
    pub left: Length,
}

impl TrblLength {
    pub fn new(top: Length, right: Length, bottom: Length, left: Length) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }
}

impl FromStr for TrblLength {
    type Err = Error;
    fn from_str(value: &str) -> Result<Self> {
        // convert parts to Length, fail if any conversion fails.
        let parts: Result<Vec<_>> = attr_split(value).map(|v| strp_length(&v)).collect();
        let parts = parts?;

        Ok(match parts.len() {
            1 => TrblLength::new(parts[0], parts[0], parts[0], parts[0]),
            2 => TrblLength::new(parts[0], parts[1], parts[0], parts[1]),
            3 => TrblLength::new(parts[0], parts[1], parts[2], parts[1]),
            4 => TrblLength::new(parts[0], parts[1], parts[2], parts[3]),
            _ => Err(Error::Arity("TrblLength requires 1-4 values".to_owned()))?,
        })
    }
}

/// `DirSpec` defines a location relative to an element's `BoundingBox`
#[derive(Clone, Copy, Debug)]
pub enum DirSpec {
    InFront,
    Behind,
    Below,
    Above,
}

impl FromStr for DirSpec {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "h" => Ok(Self::InFront),
            "H" => Ok(Self::Behind),
            "v" => Ok(Self::Below),
            "V" => Ok(Self::Above),
            _ => Err(Error::Parse(format!("invalid DirSpec format {value}"))),
        }
    }
}

/// Parse a elref + optional ElementLoc, e.g. `#id@tl:10%` or `#id`
pub fn parse_el_loc(s: &str) -> Result<(ElRef, Option<ElementLoc>)> {
    let (elref, remain) = extract_elref(s)?;
    if remain.is_empty() {
        return Ok((elref, None));
    }
    let remain = remain
        .strip_prefix(LOCSPEC_SEP)
        .ok_or_else(|| Error::Parse(format!("invalid locspec: '{s}'")))?;
    let mut chars = remain.chars();
    let mut loc = String::new();
    loop {
        match chars.next() {
            Some(c) if c.is_whitespace() => {
                return Err(Error::Parse(format!("invalid locspec: '{s}'")))
            }
            Some(c) => loc.push(c),
            None => return Ok((elref, Some(loc.parse()?))),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::geometry::BoundingBox;

    #[test]
    fn test_parse_loc() {
        assert_eq!(
            parse_el_loc("#a@b").unwrap(),
            (
                ElRef::Id("a".to_string()),
                Some(ElementLoc::LocSpec(LocSpec::Bottom))
            )
        );
        assert_eq!(
            parse_el_loc("#id@tl").unwrap(),
            (
                ElRef::Id("id".to_string()),
                Some(ElementLoc::LocSpec(LocSpec::TopLeft))
            )
        );
        assert_eq!(
            parse_el_loc("#id@t:25%").unwrap(),
            (
                ElRef::Id("id".to_string()),
                Some(ElementLoc::LocSpec(LocSpec::TopEdge(Length::Ratio(0.25))))
            )
        );
        assert_eq!(
            parse_el_loc("#id").unwrap(),
            (ElRef::Id("id".to_string()), None)
        );
        assert!(parse_el_loc("#id@").is_err());
        assert!(parse_el_loc("#id@ l").is_err());
    }

    #[test]
    fn test_strp_length() {
        assert_eq!(strp_length("1").ok(), Some(Length::Absolute(1.)));
        assert_eq!(strp_length("123").ok(), Some(Length::Absolute(123.)));
        assert_eq!(strp_length("-0.0123").ok(), Some(Length::Absolute(-0.0123)));
        assert_eq!(strp_length("0.5%").ok(), Some(Length::Ratio(0.005)));
        assert_eq!(strp_length("150%").ok(), Some(Length::Ratio(1.5)));
        assert_eq!(
            strp_length("1/5").ok(),
            Some(Length::Rational(1, NonZeroU32::new(5).unwrap()))
        );
        assert_eq!(
            strp_length("-2/37").ok(),
            Some(Length::Rational(-2, NonZeroU32::new(37).unwrap()))
        );
        assert_eq!(strp_length("1.2.3").ok(), None);
        assert_eq!(strp_length("a").ok(), None);
        assert_eq!(strp_length("a%").ok(), None);
    }

    #[test]
    fn test_length() {
        let def_len = Length::default();
        assert_eq!(def_len.absolute(), Some(0.));
        assert_eq!(def_len.ratio(), None);

        let abs_len = Length::Absolute(123.5);
        assert_eq!(abs_len.absolute(), Some(123.5));
        assert_eq!(abs_len.ratio(), None);
        assert_eq!(abs_len.adjust(3.125), 123.5 + 3.125);

        let ratio_len = Length::Ratio(0.75);
        assert_eq!(ratio_len.absolute(), None);
        assert_eq!(ratio_len.ratio(), Some(0.75));
        assert_eq!(ratio_len.adjust(3.125), 0.75 * 3.125);

        let rat_len = Length::Rational(3, NonZeroU32::new(4).unwrap());
        assert_eq!(rat_len.absolute(), None);
        assert_eq!(rat_len.ratio(), Some(0.75));
        assert_eq!(rat_len.adjust(3.125), 0.75 * 3.125);
    }

    #[test]
    fn test_length_calc_offset() {
        assert_eq!(strp_length("25%").expect("test").calc_offset(10., 50.), 20.);
        assert_eq!(
            strp_length("50%").expect("test").calc_offset(-10., -9.),
            -9.5
        );
        assert_eq!(
            strp_length("200%").expect("test").calc_offset(10., 50.),
            90.
        );
        assert_eq!(
            strp_length("-3.5").expect("test").calc_offset(10., 50.),
            46.5
        );
        assert_eq!(
            strp_length("3.5").expect("test").calc_offset(-10., 90.),
            -6.5
        );
        // Test with start > end
        assert_eq!(
            strp_length("3.5").expect("test").calc_offset(30., 10.),
            26.5
        );
        assert_eq!(strp_length("10%").expect("test").calc_offset(30., 10.), 28.);
    }

    #[test]
    fn test_length_adjust() {
        assert_eq!(strp_length("25%").expect("test").adjust(10.), 2.5);
        assert_eq!(strp_length("-50%").expect("test").adjust(150.), -75.);
        assert_eq!(strp_length("125%").expect("test").adjust(20.), 25.);
        assert_eq!(strp_length("1").expect("test").adjust(23.), 24.);
        assert_eq!(strp_length("-12").expect("test").adjust(123.), 111.);
    }

    #[test]
    fn test_locspec() {
        assert_eq!("tl".parse::<LocSpec>().expect("test"), LocSpec::TopLeft);
        assert_eq!("t".parse::<LocSpec>().expect("test"), LocSpec::Top);
        assert_eq!("tr".parse::<LocSpec>().expect("test"), LocSpec::TopRight);
        assert_eq!("r".parse::<LocSpec>().expect("test"), LocSpec::Right);
        assert_eq!("br".parse::<LocSpec>().expect("test"), LocSpec::BottomRight);
        assert_eq!("b".parse::<LocSpec>().expect("test"), LocSpec::Bottom);
        assert_eq!("bl".parse::<LocSpec>().expect("test"), LocSpec::BottomLeft);
        assert_eq!("l".parse::<LocSpec>().expect("test"), LocSpec::Left);
        assert_eq!("c".parse::<LocSpec>().expect("test"), LocSpec::Center);
        assert_eq!(
            "t:10".parse::<LocSpec>().expect("test"),
            LocSpec::TopEdge(Length::Absolute(10.))
        );
        assert_eq!(
            "r:25%".parse::<LocSpec>().expect("test"),
            LocSpec::RightEdge(Length::Ratio(0.25))
        );
        assert_eq!(
            "b:10".parse::<LocSpec>().expect("test"),
            LocSpec::BottomEdge(Length::Absolute(10.))
        );
        assert_eq!(
            "l:75%".parse::<LocSpec>().expect("test"),
            LocSpec::LeftEdge(Length::Ratio(0.75))
        );
    }

    #[test]
    fn test_get_point() {
        let bb = BoundingBox::new(10., 10., 20., 20.);
        assert_eq!(bb.locspec("t:2".parse().expect("test")), (12., 10.));
        assert_eq!(bb.locspec("r:25%".parse().expect("test")), (20., 12.5));
        assert_eq!(bb.locspec("b:6".parse().expect("test")), (16., 20.));
        assert_eq!(bb.locspec("l:150%".parse().expect("test")), (10., 25.));
        assert_eq!(bb.locspec("tl".parse().expect("test")), (10., 10.));
        assert_eq!(bb.locspec("t".parse().expect("test")), (15., 10.));
        assert_eq!(bb.locspec("tr".parse().expect("test")), (20., 10.));
        assert_eq!(bb.locspec("r".parse().expect("test")), (20., 15.));
        assert_eq!(bb.locspec("br".parse().expect("test")), (20., 20.));
        assert_eq!(bb.locspec("b".parse().expect("test")), (15., 20.));
        assert_eq!(bb.locspec("bl".parse().expect("test")), (10., 20.));
        assert_eq!(bb.locspec("l".parse().expect("test")), (10., 15.));
        assert_eq!(bb.locspec("c".parse().expect("test")), (15., 15.));
    }
}
