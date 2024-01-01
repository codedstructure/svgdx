use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::{self, Display};

use crate::Length;
use anyhow::bail;

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

impl TryFrom<&str> for EdgeSpec {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "t" => Ok(Self::Top),
            "r" => Ok(Self::Right),
            "b" => Ok(Self::Bottom),
            "l" => Ok(Self::Left),
            _ => bail!("Invalid EdgeSpec format {value}"),
        }
    }
}

impl TryFrom<String> for EdgeSpec {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
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

impl TryFrom<&str> for LocSpec {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
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

impl TryFrom<String> for LocSpec {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

/// `ScalarSpec` defines a single value from a `BoundingBox`
///
/// These are the min and max x & y values, together with width and height.
#[derive(Clone, Copy)]
pub enum ScalarSpec {
    Minx,
    Maxx,
    Miny,
    Maxy,
    Width,
    Height,
}

impl TryFrom<&str> for ScalarSpec {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "x1" | "l" => Ok(Self::Minx),
            "y1" | "t" => Ok(Self::Miny),
            "x2" | "r" => Ok(Self::Maxx),
            "y2" | "b" => Ok(Self::Maxy),
            "w" => Ok(Self::Width),
            "h" => Ok(Self::Height),
            _ => bail!("Invalid ScalarSpec format {value}"),
        }
    }
}

impl TryFrom<String> for ScalarSpec {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
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
            ScalarSpec::Width => self.x2 - self.x1,
            ScalarSpec::Height => self.y2 - self.y1,
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

    pub fn union(&self, other: &BoundingBox) -> Self {
        Self::new(
            self.x1.min(other.x1),
            self.y1.min(other.y1),
            self.x2.max(other.x2),
            self.y2.max(other.y2),
        )
    }

    pub fn combine(bb_iter: impl IntoIterator<Item = Self>) -> Option<Self> {
        let bb_iter = bb_iter.into_iter();
        bb_iter.reduce(|bb1, bb2| bb1.union(&bb2))
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
            if idx < self.attrs.len() {
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

    /// Insert-or-update the given key/value into the `AttrMap`.
    /// If the key is already present, update in place; otherwise append.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();
        let index = *self.index_map.entry(key.clone()).or_insert_with(|| {
            self.next_index += 1;
            self.next_index
        });
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

    pub fn remove(&mut self, key: impl Into<String>) -> Option<String> {
        let key = key.into();
        if let Some(&index) = self.index_map.get(&key) {
            self.index_map.remove(&key);
            self.attrs.remove(&(index, key))
        } else {
            None
        }
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

    #[test]
    fn test_bbox() {
        let mut bb = BoundingBox::new(10., 0., 10., 10.);
        bb = bb.union(&BoundingBox::new(20., 10., 30., 15.));
        bb = bb.union(&BoundingBox::new(25., 20., 25., 30.));
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

        am.remove("a");

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
}
