use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BoundingBox {
    Unknown,
    BBox(f32, f32, f32, f32),
}

impl BoundingBox {
    pub fn new() -> Self {
        BoundingBox::Unknown
    }

    pub fn extend(&mut self, other: &BoundingBox) -> &Self {
        *self = match (&self, other) {
            (Self::BBox(x1, y1, x2, y2), Self::BBox(ox1, oy1, ox2, oy2)) => {
                Self::BBox(x1.min(*ox1), y1.min(*oy1), x2.max(*ox2), y2.max(*oy2))
            }
            (Self::BBox(_, _, _, _), Self::Unknown) => *self,
            (Self::Unknown, Self::BBox(_, _, _, _)) => *other,
            (Self::Unknown, Self::Unknown) => Self::Unknown,
        };
        self
    }

    /// dilate the bounding box by the given absolute amount in each direction
    pub fn expand(&mut self, amount: f32) -> &Self {
        if let BoundingBox::BBox(x1, y1, x2, y2) = self {
            let new = BoundingBox::BBox(*x1 - amount, *y1 - amount, *x2 + amount, *y2 + amount);
            *self = new;
        }
        self
    }

    pub fn width(&self) -> Option<f32> {
        if let Self::BBox(x1, _, x2, _) = self {
            Some(x2 - x1)
        } else {
            None
        }
    }

    pub fn height(&self) -> Option<f32> {
        if let Self::BBox(_, y1, _, y2) = self {
            Some(y2 - y1)
        } else {
            None
        }
    }

    pub fn center(&self) -> Option<(f32, f32)> {
        if let Self::BBox(x1, y1, x2, y2) = self {
            Some((x1 + (x2 - x1) / 2., y1 + (y2 - y1) / 2.))
        } else {
            None
        }
    }

    /// Scale the bounding box by the given amount with origin at the center
    pub fn scale(&mut self, amount: f32) -> &Self {
        if let BoundingBox::BBox(x1, y1, x2, y2) = &self {
            let width = x2 - x1;
            let height = y2 - y1;
            let dx_by_2 = (width * amount - width) / 2.;
            let dy_by_2 = (height * amount - height) / 2.;
            *self = BoundingBox::BBox(*x1 - dx_by_2, *y1 - dy_by_2, *x2 + dx_by_2, *y2 + dy_by_2);
        }
        self
    }

    /// Expand (floor/ceil) BBox to integer coords surrounding current extent.
    pub fn round(&mut self) -> &Self {
        if let BoundingBox::BBox(x1, y1, x2, y2) = &self {
            *self = BoundingBox::BBox(x1.floor(), y1.floor(), x2.ceil(), y2.ceil());
        }
        self
    }
}

/// AttrMap - an order preserving map for storing element attributes.
///
/// Implemented with a BTreeMap for key-ordered iteration, and a separate
/// mapping from 'user-key' to index, with the BTreeMap keyed on an (index,
/// user-key) pair.
///
/// NOTE: Since next_index is never decremented, a large number of remove/insert
/// operations on the same AttrMap instance could cause overflow, especially for
/// usize < 64 bits. For the target use-case and typical 64-bit target
/// architectures, this is not considered a problem.
#[derive(Debug, Clone, Default)]
pub struct AttrMap {
    attrs: BTreeMap<(usize, String), String>,
    index_map: HashMap<String, usize>,
    next_index: usize,
}

impl AttrMap {
    pub fn new() -> Self {
        Self {
            attrs: BTreeMap::new(),
            index_map: HashMap::new(),
            next_index: 0,
        }
    }

    /// Insert-or-update the given key/value into the AttrMap.
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

    /// Insert the given key/value into the ClassList.
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

impl fmt::Display for ClassList {
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
        let mut bb = BoundingBox::new();
        bb.extend(&BoundingBox::BBox(10., 0., 10., 10.));
        bb.extend(&BoundingBox::BBox(20., 10., 30., 15.));
        bb.extend(&BoundingBox::BBox(25., 20., 25., 30.));
        assert_eq!(bb, BoundingBox::BBox(10., 0., 30., 30.));

        bb.expand(10.);
        assert_eq!(bb, BoundingBox::BBox(0., -10., 40., 40.));

        bb.scale(1.1);
        assert_eq!(bb, BoundingBox::BBox(-2., -12.5, 42., 42.5));
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
            total += value.parse::<i32>().unwrap();
        }
        assert_eq!(total, 35);
        let mut total = 0;
        for (_key, value) in am {
            total += value.parse::<i32>().unwrap();
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
        assert_eq!(format!("{}", cl), r#"ClassList["abc", "xyz", "pqr"]"#);

        let cl: ClassList = target_state.clone().into_iter().collect();
        assert_eq!(cl.to_vec(), target_state);
    }
}
