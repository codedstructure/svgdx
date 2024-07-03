use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::{self, Display};
use std::num::ParseFloatError;

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
    attrs: BTreeMap<(isize, String), String>,
    index_map: HashMap<String, isize>,
    next_index: isize,
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

    fn tweak_index(&self, key: &str, index: isize) -> isize {
        match key {
            "id" => -10000,
            "version" => -9900,
            "xmlns" => -9800,
            "href" => -9700,
            "x" => -9000,
            "cx" => -8500,
            "x1" => -8250,
            "y" => -8000,
            "cy" => -7500,
            "y1" => -7250,
            "x2" => -7100,
            "y2" => -7050,
            "width" => -7000,
            "rx" => -6500,
            "height" => -6000,
            "ry" => -5500,
            "r" => -5000,
            _ => index,
        }
    }

    /// Insert-or-update the given key/value into the `AttrMap`.
    /// If the key is already present, update in place; otherwise append.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();
        let tweaked = self.tweak_index(&key, self.next_index + 1);
        if tweaked >= 0 {
            self.next_index += 1;
        }
        let index = *self.index_map.entry(key.clone()).or_insert_with(|| tweaked);
        self.attrs.insert((index, key), value);
    }

    pub fn insert_first(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        if !self.contains_key(&key) {
            self.insert(key, value);
        }
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
