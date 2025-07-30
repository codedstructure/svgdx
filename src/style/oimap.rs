#![allow(dead_code)]

use std::collections::HashMap;
use std::hash::Hash;

#[derive(Debug, Clone)]
pub struct InsertOrderMap<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    map: HashMap<K, V>,
    keys: Vec<K>,
}

impl<K, V> Default for InsertOrderMap<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    fn default() -> Self {
        Self {
            map: HashMap::new(),
            keys: Vec::new(),
        }
    }
}

impl<K, V> InsertOrderMap<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.map.clear();
        self.keys.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn insert(&mut self, key: K, value: V) {
        if !self.map.contains_key(&key) {
            self.keys.push(key.clone());
        }
        self.map.insert(key, value);
    }

    pub fn extend(&mut self, other: &Self) {
        for key in &other.keys {
            if let Some(value) = other.map.get(key) {
                self.insert(key.clone(), value.clone());
            }
        }
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.map.get(key)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.map.get_mut(key)
    }

    pub fn get_or_insert_with<F>(&mut self, key: K, f: F) -> &V
    where
        F: FnOnce() -> V,
    {
        if !self.contains_key(&key) {
            self.insert(key.clone(), f());
        }
        self.map.get(&key).unwrap()
    }

    pub fn get_or_insert_with_mut<F>(&mut self, key: K, f: F) -> &mut V
    where
        F: FnOnce() -> V,
    {
        if !self.contains_key(&key) {
            self.insert(key.clone(), f());
        }
        self.map.get_mut(&key).unwrap()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.keys
            .iter()
            .filter_map(|k| self.map.get(k).map(|v| (k, v)))
    }

    pub fn pop(&mut self, key: &K) -> Option<V> {
        if let Some(value) = self.map.remove(key) {
            if let Some(pos) = self.keys.iter().position(|k| k == key) {
                // NOTE: O(n)
                self.keys.remove(pos);
            }
            Some(value)
        } else {
            None
        }
    }

    pub fn to_vec(&self) -> Vec<(K, V)> {
        self.keys
            .iter()
            .filter_map(|k| self.map.get(k).map(|v| (k.clone(), v.clone())))
            .collect()
    }
}

impl<K, V> FromIterator<(K, V)> for InsertOrderMap<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut mapping = Self::new();
        for (k, v) in iter {
            mapping.insert(k, v);
        }
        mapping
    }
}

impl<K, V> IntoIterator for InsertOrderMap<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    type Item = (K, V);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.keys
            .into_iter()
            .filter_map(|k| self.map.get(&k).map(|v| (k, v.clone())))
            .collect::<Vec<_>>()
            .into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ordered_mapping() {
        let mut map = InsertOrderMap::new();
        map.insert("a", 1);
        map.insert("b", 2);
        map.insert("c", 3);
        map.insert("b", 4); // Update existing

        let items: Vec<_> = map.iter().collect();
        assert_eq!(items, vec![(&"a", &1), (&"b", &4), (&"c", &3),]);

        assert_eq!(map.get(&"b"), Some(&4));
        assert!(map.contains_key(&"c"));
        assert!(!map.contains_key(&"d"));

        map.pop(&"b");
        assert_eq!(map.len(), 2);
        assert!(!map.contains_key(&"b"));

        let items: Vec<_> = map.iter().collect();
        assert_eq!(items, vec![(&"a", &1), (&"c", &3),]);
    }
}
