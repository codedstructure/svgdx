use crate::constants::{ELREF_ID_PREFIX, ELREF_NEXT, ELREF_PREVIOUS};
use crate::errors::{Result, SvgdxError};
use std::fmt::{self, Display};
use std::num::NonZeroU8;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

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
pub fn strp(s: &str) -> Result<f32> {
    s.trim()
        .parse::<f32>()
        .map_err(|_| SvgdxError::ParseError(format!("Expected a number: '{s}'")))
}

/// Parse a string such as "32.5mm" into a value (32.5) and unit ("mm")
pub fn split_unit(s: &str) -> Result<(f32, String)> {
    let mut value = String::new();
    let mut unit = String::new();
    let mut got_value = false;
    for ch in s.trim().chars() {
        if ch.is_ascii_digit() || ch == '.' || ch == '-' {
            if got_value {
                return Err(SvgdxError::ParseError(format!(
                    "Invalid character in numeric value: '{ch}'"
                )));
            }
            value.push(ch);
        } else {
            if value.is_empty() {
                return Err(SvgdxError::ParseError(format!(
                    "'{s}' does not start with numeric value"
                )));
            }
            got_value = true;
            unit.push(ch);
        }
    }
    Ok((strp(&value)?, unit))
}

/// Returns iterator over whitespace-or-comma separated values
pub fn attr_split(input: &str) -> impl Iterator<Item = String> + '_ {
    input
        .split_whitespace()
        .flat_map(|v| v.split(','))
        .filter(|&v| !v.is_empty())
        .map(|v| v.to_string())
}

pub fn split_compound_attr(value: &str) -> (String, String) {
    // wh="10" -> width="10", height="10"
    // wh="10 20" -> width="10", height="20"
    // wh="#thing" -> width="#thing", height="#thing"
    // wh="#thing 50%" -> width="#thing 50%", height="#thing 50%"
    // wh="#thing 10 20" -> width="#thing 10", height="#thing 20"
    if value.starts_with([ELREF_ID_PREFIX, ELREF_PREVIOUS]) {
        let mut parts = value.splitn(2, char::is_whitespace);
        let prefix = parts.next().expect("nonempty");
        if let Some(remain) = parts.next() {
            let mut parts = attr_split_cycle(remain);
            let x_suffix = parts.next().unwrap_or_default();
            let y_suffix = parts.next().unwrap_or_default();
            ([prefix, &x_suffix].join(" "), [prefix, &y_suffix].join(" "))
        } else {
            (value.to_owned(), value.to_owned())
        }
    } else {
        let mut parts = attr_split_cycle(value);
        let x = parts.next().unwrap_or_default();
        let y = parts.next().unwrap_or_default();
        (x, y)
    }
}

pub fn extract_urlref(input: &str) -> Option<ElRef> {
    input
        .trim()
        .strip_prefix("url(#")
        .and_then(|s| s.strip_suffix(')'))
        .map(|id| ElRef::Id(id.to_string()))
}

/// Returns iterator *cycling* over whitespace-or-comma separated values
pub fn attr_split_cycle(input: &str) -> impl Iterator<Item = String> + '_ {
    let x: Vec<String> = attr_split(input).collect();
    x.into_iter().cycle()
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OrderIndex(Vec<usize>);

impl OrderIndex {
    pub fn new(idx: usize) -> Self {
        Self(vec![idx])
    }

    pub fn depth(&self) -> usize {
        self.0.len()
    }

    pub fn step(&mut self) {
        if let Some(v) = self.0.last_mut() {
            *v += 1;
        }
    }

    pub fn down(&mut self) {
        *self = self.with_index(1);
    }

    pub fn up(&mut self) {
        self.0.pop().expect("OrderIndex underflow");
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

    /// is `other` a strict prefix of self?
    pub fn has_prefix(&self, other: &Self) -> bool {
        // other is shorter and all elements match
        other.0.len() < self.0.len() && self.0.iter().zip(other.0.iter()).all(|(a, b)| a == b)
    }
}

impl Display for OrderIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        for (i, v) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, ".")?;
            }
            write!(f, "{v}")?;
        }
        write!(f, "}}")
    }
}

/// `OrderedMap` - an order preserving map for low-cardinality string pairs.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OrderedMap(Vec<(String, String)>);

impl FromIterator<(std::string::String, std::string::String)> for OrderedMap {
    fn from_iter<I: IntoIterator<Item = (String, String)>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl OrderedMap {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Insert-or-update the given key/value into the `AttrMap`.
    /// If the key is already present, update in place; otherwise append.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();
        if let Some((_, v)) = self.0.iter_mut().find(|(k, _)| *k == key) {
            *v = value;
        } else {
            self.0.push((key, value));
        }
    }

    pub fn extend(&mut self, other: &Self) {
        for (k, v) in &other.0 {
            self.insert(k.clone(), v.clone());
        }
    }

    /// Insert-or-update the given key/value into the `AttrMap`.
    /// If the key is already present, update in place; otherwise append.
    pub fn insert_first(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        if !self.contains_key(&key) {
            self.insert(key, value.into());
        }
    }

    pub fn contains_key(&self, key: impl Into<String>) -> bool {
        let key = key.into();
        self.0.iter().any(|(k, _)| *k == key)
    }

    pub fn get(&self, key: impl Into<String>) -> Option<&str> {
        let key = key.into();
        self.0
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.as_ref())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> + '_ {
        self.0.iter().map(|(k, v)| (k.as_ref(), v.as_ref()))
    }

    pub fn pop(&mut self, key: impl Into<String>) -> Option<String> {
        let key = key.into();
        if let Some(pos) = self.0.iter().position(|(k, _)| *k == key) {
            Some(self.0.remove(pos).1)
        } else {
            None
        }
    }

    pub fn to_vec(&self) -> Vec<(String, String)> {
        self.0.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }
}

/// `AttrMap` - an order preserving map for storing element attributes.
///
/// Reordered on insert to provide partial ordering of attributes,
/// e.g. 'id' before 'x' before 'width', etc.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AttrMap {
    attrs: OrderedMap,
}

impl Deref for AttrMap {
    type Target = OrderedMap;

    fn deref(&self) -> &Self::Target {
        &self.attrs
    }
}
impl DerefMut for AttrMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.attrs
    }
}

impl Display for AttrMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (idx, (k, v)) in self.attrs.iter().enumerate() {
            write!(f, r#"{k}="{v}""#)?;
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
            attrs: OrderedMap::new(),
        }
    }

    fn priority(key: &str) -> usize {
        match key {
            "id" => 0,
            "version" => 1,
            "xmlns" => 2,
            "href" => 3,
            "x" => 4,
            "cx" => 5,
            "x1" => 6,
            "y" => 7,
            "cy" => 8,
            "y1" => 9,
            "x2" => 10,
            "y2" => 11,
            "width" => 12,
            "height" => 13,
            "rx" => 14,
            "ry" => 15,
            "r" => 16,
            _ => usize::MAX,
        }
    }

    fn reorder(&mut self) {
        self.attrs.0.sort_by_key(|(k, _)| Self::priority(k));
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.attrs.insert(key, value);
        // TODO: if many attributes are being inserted, might want to defer this
        self.reorder();
    }
}

impl From<Vec<(String, String)>> for AttrMap {
    fn from(value: Vec<(String, String)>) -> Self {
        let mut am = Self {
            attrs: OrderedMap(value),
        };
        am.reorder();
        am
    }
}

impl FromIterator<(String, String)> for AttrMap {
    fn from_iter<I: IntoIterator<Item = (String, String)>>(iter: I) -> Self {
        let am_vec = iter.into_iter().collect::<Vec<_>>();
        am_vec.into()
    }
}

impl IntoIterator for AttrMap {
    type Item = (String, String);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.attrs.0.into_iter()
    }
}

impl<'s> IntoIterator for &'s AttrMap {
    type Item = (&'s str, &'s str);
    type IntoIter = std::iter::Map<
        std::slice::Iter<'s, (String, String)>,
        fn(&(String, String)) -> (&str, &str),
    >;

    fn into_iter(self) -> Self::IntoIter {
        // can't just use a closure here because of lifetime issues
        fn to_str_pair((k, v): &(String, String)) -> (&str, &str) {
            (k.as_str(), v.as_str())
        }
        self.attrs
            .0
            .iter()
            .map(to_str_pair as fn(&(String, String)) -> (&str, &str))
    }
}

/// An order preserving map for storing element styles.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StyleMap {
    styles: OrderedMap,
}

impl Deref for StyleMap {
    type Target = OrderedMap;

    fn deref(&self) -> &Self::Target {
        &self.styles
    }
}

impl DerefMut for StyleMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.styles
    }
}

impl Display for StyleMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (idx, (k, v)) in self.styles.iter().enumerate() {
            write!(f, r"{k}: {v};")?;
            if idx < self.styles.len() - 1 {
                write!(f, " ")?;
            }
        }
        Ok(())
    }
}

impl FromStr for StyleMap {
    type Err = SvgdxError;

    fn from_str(s: &str) -> Result<Self> {
        let mut styles = Self::new();
        for rule in s.split(';').filter_map(|kv| {
            let mut parts = kv.splitn(2, ':');
            if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                Some((key.trim().to_string(), value.trim().to_string()))
            } else {
                None
            }
        }) {
            styles.insert(rule.0, rule.1);
        }
        Ok(styles)
    }
}

impl StyleMap {
    pub fn new() -> Self {
        Self {
            styles: OrderedMap::new(),
        }
    }
}

impl From<Vec<(String, String)>> for StyleMap {
    fn from(value: Vec<(String, String)>) -> Self {
        Self {
            styles: OrderedMap(value),
        }
    }
}

impl FromIterator<(String, String)> for StyleMap {
    fn from_iter<I: IntoIterator<Item = (String, String)>>(iter: I) -> Self {
        let am_vec = iter.into_iter().collect::<Vec<_>>();
        am_vec.into()
    }
}

impl IntoIterator for StyleMap {
    type Item = (String, String);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.styles.0.into_iter()
    }
}

impl<'s> IntoIterator for &'s StyleMap {
    type Item = (&'s String, &'s String);
    type IntoIter = std::iter::Map<
        std::slice::Iter<'s, (String, String)>,
        fn(&(String, String)) -> (&String, &String),
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.styles.0.iter().map(|(k, v)| (k, v))
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ClassList {
    classes: Vec<String>,
}

impl ClassList {
    pub fn new() -> Self {
        Self {
            classes: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.classes.clear();
    }

    /// Insert the given class into the `ClassList`.
    pub fn insert(&mut self, class: impl Into<String>) {
        let class = class.into();
        if !self.classes.contains(&class) {
            self.classes.push(class);
        }
    }

    pub fn extend(&mut self, other: &Self) {
        for class in other.iter() {
            self.insert(class.clone());
        }
    }

    pub fn contains(&self, class: impl Into<String>) -> bool {
        let class = class.into();
        self.classes.contains(&class)
    }

    pub fn is_empty(&self) -> bool {
        self.classes.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &String> + '_ {
        self.classes.iter()
    }

    /// Replace a class entry with a new class (or multiple space-separated)
    pub fn replace(&mut self, old: impl Into<String>, new: impl Into<String>) {
        let old = old.into();
        if self.remove(&old) {
            for class in new.into().split_whitespace() {
                self.insert(class);
            }
        }
    }

    pub fn remove(&mut self, class: impl Into<String>) -> bool {
        let class = class.into();
        if let Some(pos) = self.classes.iter().position(|c| *c == class) {
            self.classes.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn to_vec(&self) -> Vec<String> {
        self.classes.clone()
    }
}

impl Display for ClassList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ClassList{:?}", self.classes)
    }
}

impl From<Vec<String>> for ClassList {
    fn from(value: Vec<String>) -> Self {
        let mut cl = Self::new();
        for class in value {
            cl.insert(class);
        }
        cl
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
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.classes.into_iter()
    }
}

impl<'s> IntoIterator for &'s ClassList {
    type Item = &'s String;
    type IntoIter = std::slice::Iter<'s, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.classes.iter()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ElRef {
    Id(String),
    // If zero then selfref which causes circular reference
    Prev(NonZeroU8),
    Next(NonZeroU8),
}

impl FromStr for ElRef {
    type Err = SvgdxError;
    fn from_str(s: &str) -> Result<Self> {
        let (elref, remain) = extract_elref(s)?;
        if remain.is_empty() {
            Ok(elref)
        } else {
            Err(SvgdxError::ParseError(format!(
                "Invalid elref format '{s}'"
            )))
        }
    }
}

impl Display for ElRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ElRef::Id(id) => write!(f, "{ELREF_ID_PREFIX}{id}"),
            ElRef::Prev(num) => write!(
                f,
                "{}",
                ELREF_PREVIOUS.to_string().repeat(num.get() as usize)
            ),
            ElRef::Next(num) => write!(f, "{}", ELREF_NEXT.to_string().repeat(num.get() as usize)),
        }
    }
}

/// return Elref and remaining string
pub fn extract_elref(s: &str) -> Result<(ElRef, &str)> {
    let first_char_match = |c: char| c.is_alphabetic() || c == '_';
    let subseq_char_match = |c: char| c.is_alphanumeric() || c == '_' || c == '-';

    if let Some(s) = s.strip_prefix(ELREF_ID_PREFIX) {
        if s.starts_with(first_char_match) {
            if let Some(split) = s.find(|c: char| !subseq_char_match(c)) {
                let (id, remain) = s.split_at(split);
                return Ok((ElRef::Id(id.to_owned()), remain));
            } else {
                return Ok((ElRef::Id(s.to_owned()), ""));
            }
        }
    } else if s.starts_with([ELREF_PREVIOUS, ELREF_NEXT]) {
        let elref_char = if s.starts_with(ELREF_PREVIOUS) {
            ELREF_PREVIOUS
        } else {
            ELREF_NEXT
        };

        let new_s = s.trim_start_matches(elref_char);
        let num = (s.len() - new_s.len()) as u8; // asummes elref_char is 1 byte

        let non_zero_num = NonZeroU8::new(num).expect("cannot be 0 due to starts_with");

        let elref = if elref_char == ELREF_PREVIOUS {
            ElRef::Prev(non_zero_num)
        } else {
            ElRef::Next(non_zero_num)
        };
        return Ok((elref, new_s));
    }

    Err(SvgdxError::ParseError(format!(
        "Invalid elref format '{s}'"
    )))
}

#[cfg(test)]
mod test {
    use super::*;
    use assertables::assert_lt;

    #[test]
    fn test_split_unit() {
        assert_eq!(split_unit("1.5mm").unwrap(), (1.5, "mm".to_string()));
        assert_eq!(split_unit("123in").unwrap(), (123., "in".to_string()));
        assert_eq!(split_unit("123.5").unwrap(), (123.5, "".to_string()));
        assert!(split_unit("123.5.1").is_err());
        assert!(split_unit("in").is_err());
        assert!(split_unit("123in0").is_err());
        assert!(split_unit("in0").is_err());
    }

    #[test]
    fn test_spread_attr() {
        let (w, h) = split_compound_attr("10");
        assert_eq!(w, "10");
        assert_eq!(h, "10");
        let (w, h) = split_compound_attr("10 20");
        assert_eq!(w, "10");
        assert_eq!(h, "20");
        let (w, h) = split_compound_attr("#thing");
        assert_eq!(w, "#thing");
        assert_eq!(h, "#thing");
        let (w, h) = split_compound_attr("#thing 50%");
        assert_eq!(w, "#thing 50%");
        assert_eq!(h, "#thing 50%");
        let (w, h) = split_compound_attr("#thing 10 20");
        assert_eq!(w, "#thing 10");
        assert_eq!(h, "#thing 20");

        let (x, y) = split_compound_attr("^a@tl");
        assert_eq!(x, "^a@tl");
        assert_eq!(y, "^a@tl");
        let (x, y) = split_compound_attr("^a@tl 5");
        assert_eq!(x, "^a@tl 5");
        assert_eq!(y, "^a@tl 5");
        let (x, y) = split_compound_attr("^a@tl 5 7%");
        assert_eq!(x, "^a@tl 5");
        assert_eq!(y, "^a@tl 7%");
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

        let target_state = vec![("c", "1"), ("a", "2"), ("f", "30"), ("e", "4")];

        let target_state_owned = target_state
            .iter()
            .map(|v| (v.0.to_owned(), v.1.to_owned()))
            .collect::<Vec<_>>();

        // check into_iter() works
        assert_eq!(
            am.clone().into_iter().collect::<Vec<_>>(),
            target_state_owned
        );

        assert_eq!(am.iter().collect::<Vec<_>>(), target_state);

        am.pop("a");

        assert_eq!(
            am.iter().collect::<Vec<_>>(),
            vec![("c", "1"), ("f", "30"), ("e", "4")]
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
    fn test_stylemap() {
        let mut sm = StyleMap::new();
        sm.insert("fill", "red");
        sm.insert("font-size", "12px");
        sm.insert("stroke-width", "0");
        sm.insert("fill", "blue"); // update existing key
        let expected = vec![
            ("fill".to_string(), "blue".to_string()),
            ("font-size".to_string(), "12px".to_string()),
            ("stroke-width".to_string(), "0".to_string()),
        ];
        assert_eq!(sm.to_vec(), expected);

        assert_eq!(
            format!("{sm}"),
            r#"fill: blue; font-size: 12px; stroke-width: 0;"#
        );
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

    #[test]
    fn test_order_index_prefix() {
        let idx1 = OrderIndex(vec![1]);
        let idx2 = OrderIndex(vec![1, 2]);
        let idx3 = OrderIndex(vec![1, 2, 1]);
        let idx4 = OrderIndex(vec![1, 2, 2]);
        assert!(idx2.has_prefix(&idx1));
        assert!(idx3.has_prefix(&idx1));
        assert!(idx3.has_prefix(&idx2));
        assert!(idx4.has_prefix(&idx1));
        assert!(idx4.has_prefix(&idx2));
        assert!(!idx4.has_prefix(&idx3));
        assert!(!idx1.has_prefix(&idx2));
        assert!(!idx1.has_prefix(&idx3));
        // x is not a prefix of x
        assert!(!idx1.has_prefix(&idx1));
    }

    #[test]
    fn test_extract_elref() {
        assert_eq!(
            extract_elref("#id@tl:10%").unwrap(),
            (ElRef::Id("id".to_string()), "@tl:10%")
        );
        assert_eq!(
            extract_elref("#id").unwrap(),
            (ElRef::Id("id".to_string()), "")
        );
        assert_eq!(
            extract_elref("#id@").unwrap(),
            (ElRef::Id("id".to_string()), "@")
        );
        assert_eq!(
            extract_elref("#id_a@xyz 2 3").unwrap(),
            (ElRef::Id("id_a".to_string()), "@xyz 2 3")
        );

        fn nz(num: u8) -> NonZeroU8 {
            NonZeroU8::new(num).expect("not 0")
        }

        assert_eq!(extract_elref("^@bl").unwrap(), (ElRef::Prev(nz(1)), "@bl"));
        assert_eq!(extract_elref("^").unwrap(), (ElRef::Prev(nz(1)), ""));
        assert_eq!(extract_elref("^^^^").unwrap(), (ElRef::Prev(nz(4)), ""));
        assert_eq!(extract_elref("^+").unwrap(), (ElRef::Prev(nz(1)), "+"));
        assert_eq!(extract_elref("^3^^").unwrap(), (ElRef::Prev(nz(1)), "3^^"));
        assert_eq!(extract_elref("^^3^").unwrap(), (ElRef::Prev(nz(2)), "3^"));
        assert_eq!(extract_elref("+").unwrap(), (ElRef::Next(nz(1)), ""));
        assert_eq!(extract_elref("++++").unwrap(), (ElRef::Next(nz(4)), ""));
        assert_eq!(extract_elref("++3+").unwrap(), (ElRef::Next(nz(2)), "3+"));
        assert!(extract_elref("id").is_err());
    }
}
