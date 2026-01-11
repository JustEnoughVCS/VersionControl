use ahash::AHasher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hash};

type FastHashMap<K, V> = HashMap<K, V, BuildHasherDefault<AHasher>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiMap<A, B>
where
    A: Eq + Hash + Clone,
    B: Eq + Hash + Clone,
{
    #[serde(flatten)]
    a_to_b: FastHashMap<A, B>,
    #[serde(skip)]
    b_to_a: FastHashMap<B, A>,
}

pub struct Entry<'a, A, B>
where
    A: Eq + Hash + Clone,
    B: Eq + Hash + Clone,
{
    bimap: &'a mut BiMap<A, B>,
    key: A,
    value: Option<B>,
}

impl<A, B> BiMap<A, B>
where
    A: Eq + Hash + Clone,
    B: Eq + Hash + Clone,
{
    pub fn new() -> Self {
        Self {
            a_to_b: FastHashMap::default(),
            b_to_a: FastHashMap::default(),
        }
    }

    pub fn entry(&mut self, a: A) -> Entry<'_, A, B> {
        let value = self.a_to_b.get(&a).cloned();
        Entry {
            bimap: self,
            key: a,
            value,
        }
    }

    #[inline(always)]
    pub fn insert(&mut self, a: A, b: B) {
        if let Some(old_b) = self.a_to_b.insert(a.clone(), b.clone()) {
            self.b_to_a.remove(&old_b);
        }
        if let Some(old_a) = self.b_to_a.insert(b.clone(), a.clone()) {
            self.a_to_b.remove(&old_a);
        }
    }

    #[inline(always)]
    pub fn get_by_a(&self, key: &A) -> Option<&B> {
        self.a_to_b.get(key)
    }

    #[inline(always)]
    pub fn get_by_b(&self, key: &B) -> Option<&A> {
        self.b_to_a.get(key)
    }

    pub fn remove_by_a(&mut self, key: &A) -> Option<(A, B)> {
        if let Some(b) = self.get_by_a(key).cloned() {
            let a = self.get_by_b(&b).cloned().unwrap();
            self.a_to_b.remove(key);
            self.b_to_a.remove(&b);
            Some((a, b))
        } else {
            None
        }
    }

    pub fn remove_by_b(&mut self, key: &B) -> Option<(A, B)> {
        if let Some(a) = self.get_by_b(key).cloned() {
            let b = self.get_by_a(&a).cloned().unwrap();
            self.b_to_a.remove(key);
            self.a_to_b.remove(&a);
            Some((a, b))
        } else {
            None
        }
    }

    pub fn reserve(&mut self, additional: usize) {
        self.a_to_b.reserve(additional);
        self.b_to_a.reserve(additional);
    }

    pub fn len(&self) -> usize {
        self.a_to_b.len()
    }

    pub fn is_empty(&self) -> bool {
        self.a_to_b.is_empty()
    }

    pub fn clear(&mut self) {
        self.a_to_b.clear();
        self.b_to_a.clear();
    }

    pub fn contains_a(&self, key: &A) -> bool {
        self.a_to_b.contains_key(key)
    }

    pub fn contains_b(&self, key: &B) -> bool {
        self.b_to_a.contains_key(key)
    }

    pub fn keys_a(&self) -> impl Iterator<Item = &A> {
        self.a_to_b.keys()
    }

    pub fn keys_b(&self) -> impl Iterator<Item = &B> {
        self.b_to_a.keys()
    }

    pub fn iter_a_to_b(&self) -> impl Iterator<Item = (&A, &B)> {
        self.a_to_b.iter()
    }

    pub fn iter_b_to_a(&self) -> impl Iterator<Item = (&B, &A)> {
        self.b_to_a.iter()
    }
}

impl<'a, A, B> Entry<'a, A, B>
where
    A: Eq + Hash + Clone,
    B: Eq + Hash + Clone,
{
    pub fn and_modify<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut B),
    {
        if let Some(ref mut value) = self.value {
            f(value);
        }
        self
    }

    pub fn or_insert(self, default: B) -> Result<&'a mut B, &'static str> {
        self.or_insert_with(|| default)
    }

    pub fn or_insert_with<F>(mut self, default: F) -> Result<&'a mut B, &'static str>
    where
        F: FnOnce() -> B,
    {
        if self.value.is_none() {
            self.value = Some(default());
        }

        let value = self.value.as_ref().ok_or("Value is None")?.clone();
        self.bimap.insert(self.key.clone(), value);

        self.bimap
            .a_to_b
            .get_mut(&self.key)
            .ok_or("Key not found in a_to_b map")
    }
}

impl<A, B> Default for BiMap<A, B>
where
    A: Eq + Hash + Clone,
    B: Eq + Hash + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bimap_basic_operations() {
        let mut bimap = BiMap::new();
        bimap.insert("key1", "value1");

        assert_eq!(bimap.get_by_a(&"key1"), Some(&"value1"));
        assert_eq!(bimap.get_by_b(&"value1"), Some(&"key1"));
        assert!(bimap.contains_a(&"key1"));
        assert!(bimap.contains_b(&"value1"));
    }

    #[test]
    fn test_bimap_remove() {
        let mut bimap = BiMap::new();
        bimap.insert(1, "one");

        assert_eq!(bimap.remove_by_a(&1), Some((1, "one")));
        assert!(bimap.is_empty());
    }

    #[test]
    fn test_bimap_entry() {
        let mut bimap = BiMap::new();
        bimap.entry("key1").or_insert("value1").unwrap();

        assert_eq!(bimap.get_by_a(&"key1"), Some(&"value1"));
    }

    #[test]
    fn test_bimap_iterators() {
        let mut bimap = BiMap::new();
        bimap.insert(1, "one");
        bimap.insert(2, "two");

        let a_keys: Vec<_> = bimap.keys_a().collect();
        assert!(a_keys.contains(&&1) && a_keys.contains(&&2));

        let b_keys: Vec<_> = bimap.keys_b().collect();
        assert!(b_keys.contains(&&"one") && b_keys.contains(&&"two"));
    }

    #[test]
    fn test_bimap_duplicate_insert() {
        let mut bimap = BiMap::new();
        bimap.insert(1, "one");
        bimap.insert(1, "new_one");
        bimap.insert(2, "one");

        assert_eq!(bimap.get_by_a(&1), Some(&"new_one"));
        assert_eq!(bimap.get_by_b(&"one"), Some(&2));
        assert_eq!(bimap.get_by_a(&2), Some(&"one"));
    }
}
