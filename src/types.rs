use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::{self, Display};
use std::num::ParseFloatError;
use std::str::FromStr;

use anyhow::bail;

/// Return a 'minimal' representation of the given number
pub fn fstr(x: f32) -> String {
    if x.abs() < 0.0001 {
        // Handle very small negative values to avoid '-0'
        return "0".to_string();
    }
    if x == (x as i32) as f32 {
        return (x as i32).to_string();
    }
    let result = format!("{x:.3}");
    // Remove trailing 0s and then trailing '.' if it exists.
    // Note: this assumes `result` is a well-formatted f32, and always
    // contains a '.' - otherwise '1000' would become '1'...
    result.trim_end_matches('0').trim_end_matches('.').into()
}

/// Parse a string to an f32
pub fn strp(s: &str) -> anyhow::Result<f32> {
    s.trim().parse().map_err(|e: ParseFloatError| e.into())
}

/// Returns iterator over whitespace-or-comma separated values
pub fn attr_split(input: &str) -> impl Iterator<Item = String> + '_ {
    input
        .split_whitespace()
        .flat_map(|v| v.split(','))
        .filter(|&v| !v.is_empty())
        .map(|v| v.to_string())
}

/// Returns iterator *cycling* over whitespace-or-comma separated values
pub fn attr_split_cycle(input: &str) -> impl Iterator<Item = String> + '_ {
    let x: Vec<String> = attr_split(input).collect();
    x.into_iter().cycle()
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct OrderIndex(Vec<usize>);

impl OrderIndex {
    pub fn new(idx: usize) -> Self {
        Self(vec![idx])
    }

    pub fn with_sub_index(&self, other: &Self) -> Self {
        let mut new_idx = self.0.clone();
        new_idx.extend(other.0.iter());

        Self(new_idx)
    }

    pub fn with_index(&self, idx: usize) -> Self {
        let mut new_idx = self.0.clone();
        new_idx.push(idx);

        Self(new_idx)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum Length {
    Absolute(f32),
    Ratio(f32),
}

impl Default for Length {
    fn default() -> Self {
        Self::Absolute(0.)
    }
}

impl Length {
    #[allow(dead_code)]
    pub const fn ratio(&self) -> Option<f32> {
        if let Self::Ratio(result) = self {
            Some(*result)
        } else {
            None
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
        }
    }

    /// Given a single value, update it (scale or addition) from
    /// the current Length value
    pub fn adjust(&self, value: f32) -> f32 {
        match self {
            Self::Absolute(abs) => value + abs,
            Self::Ratio(ratio) => value * ratio,
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
        }
    }
}

/// Parse a ratio (float or %age) to an f32
/// Note this deliberately does not clamp to 0..1
pub fn strp_length(s: &str) -> anyhow::Result<Length> {
    if let Some(s) = s.strip_suffix('%') {
        Ok(Length::Ratio(strp(s)? * 0.01))
    } else {
        Ok(Length::Absolute(strp(s)?))
    }
}

/// `DirSpec` defines a location relative to an element's `BoundingBox`
#[derive(Clone, Copy)]
pub enum DirSpec {
    InFront,
    Behind,
    Below,
    Above,
}

impl FromStr for DirSpec {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "h" => Ok(Self::InFront),
            "H" => Ok(Self::Behind),
            "v" => Ok(Self::Below),
            "V" => Ok(Self::Above),
            _ => bail!("Invalid DirSpec format {value}"),
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

/// `EdgeSpec` defines one edge of a `BoundingBox`.
///
/// May be combined with a `Length` to refer to a point along an edge.
#[derive(Clone, Copy)]
pub enum EdgeSpec {
    Top,
    Right,
    Bottom,
    Left,
}

impl FromStr for EdgeSpec {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "t" => Ok(Self::Top),
            "r" => Ok(Self::Right),
            "b" => Ok(Self::Bottom),
            "l" => Ok(Self::Left),
            _ => bail!("Invalid EdgeSpec format {value}"),
        }
    }
}

impl TryFrom<LocSpec> for EdgeSpec {
    type Error = anyhow::Error;

    fn try_from(value: LocSpec) -> Result<Self, Self::Error> {
        match value {
            LocSpec::Top => Ok(Self::Top),
            LocSpec::Right => Ok(Self::Right),
            LocSpec::Bottom => Ok(Self::Bottom),
            LocSpec::Left => Ok(Self::Left),
            _ => bail!("Cannot convert LocSpec value into EdgeSpec"),
        }
    }
}

/// `LocSpec` defines a location on a `BoundingBox`
#[derive(Clone, Copy)]
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
}

impl FromStr for LocSpec {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
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
            _ => bail!("Invalid LocSpec format {value}"),
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
    Width,
    Rx,
    Height,
    Ry,
}

impl FromStr for ScalarSpec {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // TODO: 'r' here is ambiguous vs circle's radius attribute.
        // Perhaps require uppercase 'T/R/B/L' for edge values.
        // TODO: consider x1/x2/y1/y2: note that for e.g. a line it is
        // *not* required that the *attributes* x1 < x2 or y1 < y2.
        // Perhaps a separate 'attribute spec' concept is needed...
        match value {
            "x" | "x1" | "l" => Ok(Self::Minx),
            "y" | "y1" | "t" => Ok(Self::Miny),
            "cx" => Ok(Self::Cx),
            "x2" | "r" => Ok(Self::Maxx),
            "y2" | "b" => Ok(Self::Maxy),
            "cy" => Ok(Self::Cy),
            "w" | "width" => Ok(Self::Width),
            "rx" => Ok(Self::Rx),
            "h" | "height" => Ok(Self::Height),
            "ry" => Ok(Self::Ry),
            _ => bail!("Invalid ScalarSpec format {value}"),
        }
    }
}

/// `BoundingBox` defines an axis-aligned rectangular region is user coordinates.
///
/// Many (not all) `SvgElement` instances will have a corresponding
/// `BoundingBox`, indicating the position and size of the rendered
/// element.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
}

impl BoundingBox {
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self { x1, y1, x2, y2 }
    }

    pub fn locspec(&self, ls: LocSpec) -> (f32, f32) {
        let tl = (self.x1, self.y1);
        let tr = (self.x2, self.y1);
        let br = (self.x2, self.y2);
        let bl = (self.x1, self.y2);
        let c = ((self.x1 + self.x2) / 2., (self.y1 + self.y2) / 2.);
        match ls {
            LocSpec::TopLeft => tl,
            LocSpec::Top => ((self.x1 + self.x2) / 2., self.y1),
            LocSpec::TopRight => tr,
            LocSpec::Right => (self.x2, (self.y1 + self.y2) / 2.),
            LocSpec::BottomRight => br,
            LocSpec::Bottom => ((self.x1 + self.x2) / 2., self.y2),
            LocSpec::BottomLeft => bl,
            LocSpec::Left => (self.x1, (self.y1 + self.y2) / 2.),
            LocSpec::Center => c,
        }
    }

    pub fn scalarspec(&self, ss: ScalarSpec) -> f32 {
        match ss {
            ScalarSpec::Minx => self.x1,
            ScalarSpec::Maxx => self.x2,
            ScalarSpec::Miny => self.y1,
            ScalarSpec::Maxy => self.y2,
            ScalarSpec::Width => (self.x2 - self.x1).abs(),
            ScalarSpec::Height => (self.y2 - self.y1).abs(),
            ScalarSpec::Cx => (self.x1 + self.x2) / 2.,
            ScalarSpec::Cy => (self.y1 + self.y2) / 2.,
            ScalarSpec::Rx => (self.x2 - self.x1).abs() / 2.,
            ScalarSpec::Ry => (self.y2 - self.y1).abs() / 2.,
        }
    }

    pub fn edgespec(&self, es: EdgeSpec, len: Length) -> (f32, f32) {
        match es {
            EdgeSpec::Top => (len.calc_offset(self.x1, self.x2), self.y1),
            EdgeSpec::Right => (self.x2, len.calc_offset(self.y1, self.y2)),
            EdgeSpec::Bottom => (len.calc_offset(self.x1, self.x2), self.y2),
            EdgeSpec::Left => (self.x1, len.calc_offset(self.y1, self.y2)),
        }
    }

    pub fn union(bb_iter: impl IntoIterator<Item = Self>) -> Option<Self> {
        let bb_iter = bb_iter.into_iter();
        bb_iter.reduce(|bb1, bb2| bb1.combine(&bb2))
    }

    pub fn combine(&self, other: &Self) -> Self {
        Self::new(
            self.x1.min(other.x1),
            self.y1.min(other.y1),
            self.x2.max(other.x2),
            self.y2.max(other.y2),
        )
    }

    pub fn intersect(&self, other: &Self) -> Option<Self> {
        let result = Self::new(
            self.x1.max(other.x1),
            self.y1.max(other.y1),
            self.x2.min(other.x2),
            self.y2.min(other.y2),
        );
        if result.width() >= 0. && result.height() >= 0. {
            Some(result)
        } else {
            None
        }
    }

    pub fn intersection(bb_iter: impl IntoIterator<Item = Self>) -> Option<Self> {
        // Ideally want to use `reduce()` here, but want to exit early on None,
        // so do it long-hand.
        let mut bb_iter = bb_iter.into_iter();
        let mut bb = bb_iter.next();
        while bb.is_some() {
            if let Some(other) = bb_iter.next() {
                bb = bb?.intersect(&other);
            } else {
                break;
            }
        }
        bb
    }

    /// dilate the bounding box by the given absolute amount in each direction
    pub fn expand(&mut self, exp_x: f32, exp_y: f32) -> &Self {
        *self = Self {
            x1: self.x1 - exp_x,
            y1: self.y1 - exp_y,
            x2: self.x2 + exp_x,
            y2: self.y2 + exp_y,
        };
        self
    }

    pub fn expand_trbl_length(&mut self, trbl: TrblLength) -> &Self {
        // NOTE: not clear if x values should use width and y values use
        // height, or if having consistent values (as here) is better.
        // Current approach ensures a single-valued `TrblLength`` input
        // has a consistent border on all sides, which is probably the
        // expectation, and matches CSS (where all %ages are in terms
        // of inline-size - typically width - of parent element).
        let base = self.width().max(self.height());
        *self = Self {
            x1: self.x1 - trbl.left.evaluate(base),
            y1: self.y1 - trbl.top.evaluate(base),
            x2: self.x2 + trbl.right.evaluate(base),
            y2: self.y2 + trbl.bottom.evaluate(base),
        };
        self
    }

    pub fn shrink_trbl_length(&mut self, trbl: TrblLength) -> &Self {
        // NOTE: not clear if x values should use width and y values use
        // height, or if having consistent values (as here) is better.
        // Current approach ensures a single-valued `TrblLength`` input
        // has a consistent border on all sides, which is probably the
        // expectation, and matches CSS (where all %ages are in terms
        // of inline-size - typically width - of parent element).

        // Where 'expand_trbl_length' takes the max of width / height,
        // this takes the minimum, so shrink up to 100% still leave some
        // box present.
        let base = self.width().min(self.height());
        *self = Self {
            x1: self.x1 + trbl.left.evaluate(base),
            y1: self.y1 + trbl.top.evaluate(base),
            x2: self.x2 - trbl.right.evaluate(base),
            y2: self.y2 - trbl.bottom.evaluate(base),
        };
        self
    }

    pub fn width(&self) -> f32 {
        self.x2 - self.x1
    }

    pub fn height(&self) -> f32 {
        self.y2 - self.y1
    }

    pub fn center(&self) -> (f32, f32) {
        (
            self.x1 + (self.x2 - self.x1) / 2.,
            self.y1 + (self.y2 - self.y1) / 2.,
        )
    }

    /// Scale the bounding box by the given amount with origin at the center
    #[allow(dead_code)]
    pub fn scale(&mut self, amount: f32) -> &Self {
        let width = self.x2 - self.x1;
        let height = self.y2 - self.y1;
        let dx_by_2 = (width * amount - width) / 2.;
        let dy_by_2 = (height * amount - height) / 2.;
        *self = Self {
            x1: self.x1 - dx_by_2,
            y1: self.y1 - dy_by_2,
            x2: self.x2 + dx_by_2,
            y2: self.y2 + dy_by_2,
        };
        self
    }

    /// Expand (floor/ceil) `BBox` to integer coords surrounding current extent.
    pub fn round(&mut self) -> &Self {
        *self = Self {
            x1: self.x1.floor(),
            y1: self.y1.floor(),
            x2: self.x2.ceil(),
            y2: self.y2.ceil(),
        };
        self
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
    type Err = anyhow::Error;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // convert parts to Length, fail if any conversion fails.
        let parts: Result<Vec<_>, _> = attr_split(value).map(|v| strp_length(&v)).collect();
        let parts = parts?;

        Ok(match parts.len() {
            1 => TrblLength::new(parts[0], parts[0], parts[0], parts[0]),
            2 => TrblLength::new(parts[0], parts[1], parts[0], parts[1]),
            3 => TrblLength::new(parts[0], parts[1], parts[2], parts[1]),
            4 => TrblLength::new(parts[0], parts[1], parts[2], parts[3]),
            _ => bail!("Invalid number of values"),
        })
    }
}

/// `AttrMap` - an order preserving map for storing element attributes.
///
/// Implemented with a `BTreeMap` for key-ordered iteration, and a separate
/// mapping from 'user-key' to index, with the `BTreeMap` keyed on an (index,
/// user-key) pair.
///
/// NOTE: Since `next_index` is never decremented, a large number of remove/insert
/// operations on the same `AttrMap` instance could cause overflow, especially for
/// usize < 64 bits. For the target use-case and typical 64-bit target
/// architectures, this is not considered a problem.
#[derive(Debug, Clone, Default)]
pub struct AttrMap {
    attrs: BTreeMap<(usize, String), String>,
    index_map: HashMap<String, usize>,
    next_index: usize,
}

impl Display for AttrMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (idx, (k, v)) in self.attrs.iter().enumerate() {
            write!(f, r#"{}="{}""#, k.1, v)?;
            if idx < self.attrs.len() - 1 {
                write!(f, " ")?;
            }
        }
        Ok(())
    }
}

impl AttrMap {
    pub fn new() -> Self {
        Self {
            attrs: BTreeMap::new(),
            index_map: HashMap::new(),
            next_index: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.attrs.is_empty()
    }

    /// Insert-or-update the given key/value into the `AttrMap`.
    /// If the key is already present, update in place; otherwise append.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();
        let index = *self.index_map.entry(key.clone()).or_insert_with(|| {
            // Provide a gap so additional attributes can be inserted 'between' these,
            // e.g. in the case of removal with `pop_idx` and insertion of replacement
            // attributes with `insert_idx`.
            // TODO: The value here should ideally be > the max number of attributes supported
            // by this struct so we never have an overlap, though that assumes sensible
            // use of `insert_idx` by clients - not sustainable. Should improve the API here.
            self.next_index += 256;
            self.next_index
        });
        self.attrs.insert((index, key), value);
    }

    pub fn insert_idx(&mut self, key: impl Into<String>, value: impl Into<String>, idx: usize) {
        let key = key.into();
        let value = value.into();
        let index = *self.index_map.entry(key.clone()).or_insert_with(|| idx);
        self.attrs.insert((index, key), value);
    }

    pub fn contains_key(&self, key: impl Into<String>) -> bool {
        let key = key.into();
        self.index_map.contains_key(&key)
    }

    pub fn get(&self, key: impl Into<String>) -> Option<&String> {
        let key = key.into();
        let index = *self.index_map.get(&key)?;
        self.attrs.get(&(index, key))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> + '_ {
        self.attrs.iter().map(|item| (&item.0 .1, item.1))
    }

    pub fn pop(&mut self, key: impl Into<String>) -> Option<String> {
        let key = key.into();
        if let Some(&index) = self.index_map.get(&key) {
            self.index_map.remove(&key);
            self.attrs.remove(&(index, key))
        } else {
            None
        }
    }

    pub fn pop_idx(&mut self, key: impl Into<String>) -> Option<(String, usize)> {
        let key = key.into();
        if let Some(&index) = self.index_map.get(&key) {
            let idx = self.index_map.remove(&key).expect("invariant");
            if let Some(value) = self.attrs.remove(&(index, key)) {
                return Some((value, idx));
            }
        }
        None
    }

    pub fn to_vec(&self) -> Vec<(String, String)> {
        self.clone().into_iter().collect()
    }
}

impl From<Vec<(String, String)>> for AttrMap {
    fn from(value: Vec<(String, String)>) -> Self {
        value.into_iter().collect()
    }
}

impl FromIterator<(String, String)> for AttrMap {
    fn from_iter<I: IntoIterator<Item = (String, String)>>(iter: I) -> Self {
        let mut am = Self::new();
        for (k, v) in iter {
            am.insert(k, v);
        }
        am
    }
}

impl IntoIterator for AttrMap {
    type Item = (String, String);
    type IntoIter = <Vec<Self::Item> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.attrs
            .into_iter()
            .map(|v| (v.0 .1, v.1))
            .collect::<Vec<_>>()
            .into_iter()
    }
}

impl<'s> IntoIterator for &'s AttrMap {
    type Item = (&'s String, &'s String);
    type IntoIter = <Vec<(&'s String, &'s String)> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.attrs
            .iter()
            .map(|v| (&v.0 .1, v.1))
            .collect::<Vec<_>>()
            .into_iter()
    }
}

#[derive(Debug, Clone, Default)]
pub struct ClassList {
    classes: BTreeSet<(usize, String)>,
    index_map: HashMap<String, usize>,
    next_index: usize,
}

impl ClassList {
    pub fn new() -> Self {
        Self {
            classes: BTreeSet::new(),
            index_map: HashMap::new(),
            next_index: 0,
        }
    }

    /// Insert the given key/value into the `ClassList`.
    pub fn insert(&mut self, class: impl Into<String>) {
        let class = class.into();
        let index = *self.index_map.entry(class.clone()).or_insert_with(|| {
            self.next_index += 1;
            self.next_index
        });
        self.classes.insert((index, class));
    }

    pub fn contains(&self, class: impl Into<String>) -> bool {
        let class = class.into();
        self.index_map.contains_key(&class)
    }

    pub fn is_empty(&self) -> bool {
        self.index_map.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &String> + '_ {
        self.classes.iter().map(|item| (&item.1))
    }

    pub fn remove(&mut self, class: impl Into<String>) -> bool {
        let class = class.into();
        if let Some(&index) = self.index_map.get(&class) {
            self.index_map.remove(&class);
            self.classes.remove(&(index, class))
        } else {
            false
        }
    }

    pub fn to_vec(&self) -> Vec<String> {
        self.clone().into_iter().collect()
    }
}

impl Display for ClassList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ClassList{:?}", self.to_vec())
    }
}

impl FromIterator<String> for ClassList {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        let mut cl = Self::new();
        for class in iter {
            cl.insert(class);
        }
        cl
    }
}

impl IntoIterator for ClassList {
    type Item = String;
    type IntoIter = <Vec<Self::Item> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.classes
            .into_iter()
            .map(|v| v.1)
            .collect::<Vec<_>>()
            .into_iter()
    }
}

impl<'s> IntoIterator for &'s ClassList {
    type Item = &'s String;
    type IntoIter = <Vec<&'s String> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.classes
            .iter()
            .map(|v| &v.1)
            .collect::<Vec<_>>()
            .into_iter()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use assertables::{assert_lt, assert_lt_as_result};

    #[test]
    fn test_bbox() {
        let mut bb = BoundingBox::new(10., 0., 10., 10.);
        bb = bb.combine(&BoundingBox::new(20., 10., 30., 15.));
        bb = bb.combine(&BoundingBox::new(25., 20., 25., 30.));
        assert_eq!(bb, BoundingBox::new(10., 0., 30., 30.));

        bb.expand(10., 10.);
        assert_eq!(bb, BoundingBox::new(0., -10., 40., 40.));

        bb.scale(1.1);
        assert_eq!(bb, BoundingBox::new(-2., -12.5, 42., 42.5));
    }

    #[test]
    fn test_attrmap() {
        let mut am = AttrMap::new();
        am.insert("c", "1");
        am.insert("a", "2");
        am.insert("f", "3");
        am.insert("e", "4");
        am.insert("f", "30");

        assert!(am.contains_key("e"));
        assert!(!am.contains_key("z"));

        let target_state = vec![
            ("c".to_string(), "1".to_string()),
            ("a".to_string(), "2".to_string()),
            ("f".to_string(), "30".to_string()),
            ("e".to_string(), "4".to_string()),
        ];

        let target_state_ref = target_state
            .iter()
            .map(|v| (&v.0, &v.1))
            .collect::<Vec<_>>();

        // check into_iter() works
        assert_eq!(am.clone().into_iter().collect::<Vec<_>>(), target_state);

        assert_eq!(am.iter().collect::<Vec<_>>(), target_state_ref);

        am.pop("a");

        assert_eq!(
            am.iter().collect::<Vec<_>>(),
            vec![
                (&"c".to_string(), &"1".to_string()),
                (&"f".to_string(), &"30".to_string()),
                (&"e".to_string(), &"4".to_string())
            ]
        );

        // Check iteration (ref and owned) over the AttrMap works...
        let mut total = 0;
        for (_key, value) in &am {
            total += value.parse::<i32>().expect("test");
        }
        assert_eq!(total, 35);
        let mut total = 0;
        for (_key, value) in am {
            total += value.parse::<i32>().expect("test");
        }
        assert_eq!(total, 35);

        // Check FromIterator via collect()
        let two_attrs = vec![
            ("abc".to_string(), "123".to_string()),
            ("def".to_string(), "blob".to_string()),
        ];
        let am: AttrMap = two_attrs.clone().into_iter().collect();
        assert_eq!(am.to_vec(), two_attrs);
    }

    #[test]
    fn test_classlist() {
        let mut cl = ClassList::new();

        assert!(cl.is_empty());
        cl.insert("abc");
        cl.insert("xyz");
        cl.insert("pqr");
        assert!(!cl.is_empty());

        assert!(cl.contains("abc"));
        assert!(!cl.contains("ijk"));

        let target_state = vec!["abc".to_string(), "xyz".to_string(), "pqr".to_string()];

        assert_eq!(cl.to_vec(), target_state.clone());
        assert_eq!(format!("{cl}"), r#"ClassList["abc", "xyz", "pqr"]"#);

        let cl: ClassList = target_state.clone().into_iter().collect();
        assert_eq!(cl.to_vec(), target_state);
    }

    #[test]
    fn test_fstr() {
        assert_eq!(fstr(1.0), "1");
        assert_eq!(fstr(-100.0), "-100");
        assert_eq!(fstr(1.2345678), "1.235");
        assert_eq!(fstr(-1.2345678), "-1.235");
        assert_eq!(fstr(91.0004), "91");
        // Large-ish integers (up to 24 bit mantissa) should be fine
        assert_eq!(fstr(12345678.0), "12345678");
        assert_eq!(fstr(12340000.0), "12340000");
    }

    #[test]
    fn test_strp() {
        assert_eq!(strp("1").ok(), Some(1.));
        assert_eq!(strp("100").ok(), Some(100.));
        assert_eq!(strp("-100").ok(), Some(-100.));
        assert_eq!(strp("-0.00123").ok(), Some(-0.00123));
        assert_eq!(strp("1234567.8").ok(), Some(1234567.8));
    }

    #[test]
    fn test_strp_length() {
        assert_eq!(strp_length("1").ok(), Some(Length::Absolute(1.)));
        assert_eq!(strp_length("123").ok(), Some(Length::Absolute(123.)));
        assert_eq!(strp_length("-0.0123").ok(), Some(Length::Absolute(-0.0123)));
        assert_eq!(strp_length("0.5%").ok(), Some(Length::Ratio(0.005)));
        assert_eq!(strp_length("150%").ok(), Some(Length::Ratio(1.5)));
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
    }

    #[test]
    fn test_attr_split() {
        let mut parts = attr_split("0 1.5 23");
        assert_eq!(parts.next(), Some(String::from("0")));
        assert_eq!(parts.next(), Some(String::from("1.5")));
        assert_eq!(parts.next(), Some(String::from("23")));
        assert_eq!(parts.next(), None);

        let mut parts = attr_split("0, 1.5, 23");
        assert_eq!(parts.next(), Some(String::from("0")));
        assert_eq!(parts.next(), Some(String::from("1.5")));
        assert_eq!(parts.next(), Some(String::from("23")));
        assert_eq!(parts.next(), None);
    }

    #[test]
    fn test_attr_split_cycle() {
        let mut parts = attr_split_cycle("0 1.5 23");
        assert_eq!(parts.next(), Some(String::from("0")));
        assert_eq!(parts.next(), Some(String::from("1.5")));
        assert_eq!(parts.next(), Some(String::from("23")));
        assert_eq!(parts.next(), Some(String::from("0")));
        assert_eq!(parts.next(), Some(String::from("1.5")));
        assert_eq!(parts.next(), Some(String::from("23")));
        assert_eq!(parts.next(), Some(String::from("0")));
        assert_eq!(parts.next(), Some(String::from("1.5")));
        assert_eq!(parts.next(), Some(String::from("23")));
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
    fn test_order_index() {
        let idx_default = OrderIndex::default();
        let idx0 = OrderIndex::new(0);
        let idx1 = OrderIndex::new(1);
        let idx2 = OrderIndex::new(2);
        assert_lt!(idx_default, idx0);
        assert_lt!(idx1, idx2);
        assert_lt!(idx1, idx1.with_sub_index(&idx1));
        assert_lt!(idx1, idx1.with_sub_index(&idx1).with_sub_index(&idx1));

        let subidx1 = OrderIndex::new(101);
        let subidx2 = OrderIndex::new(102);
        assert_lt!(idx1.with_sub_index(&subidx1), idx1.with_sub_index(&subidx2));
        assert_lt!(idx1.with_sub_index(&subidx2), idx2);
        assert_lt!(idx1.with_sub_index(&subidx2), idx2.with_sub_index(&subidx1));
    }
}
