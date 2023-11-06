//! Simple thread safe hashmap

#![cfg_attr(feature = "cargo-clippy", allow(clippy::missing_panics_doc))]

use std::{
    collections::{hash_map::RandomState, HashMap},
    hash::{BuildHasher, Hash},
    ops::Deref,
    sync::RwLock,
};

/// Thread safe hashmap
#[derive(Debug)]
pub struct RwHashMap<K, V, S = RandomState>(pub(crate) RwLock<HashMap<K, V, S>>);

impl<K, V, S> RwHashMap<K, V, S>
where
    K: Eq + Hash,
    V: Clone,
{
    /// Creates an empty `RwHashMap` which will use the given hash builder to hash keys.
    pub fn with_hasher(hash_builder: S) -> Self {
        Self(RwLock::new(HashMap::with_hasher(hash_builder)))
    }
}

impl<K, V, S> Default for RwHashMap<K, V, S>
where
    K: Eq + Hash,
    V: Clone,
    S: Default,
{
    /// Creates an empty `RwHashMap<K, V, S>`, with the `Default` value for the hasher.
    fn default() -> Self {
        Self(RwLock::new(HashMap::with_hasher(S::default())))
    }
}

impl<K, V> RwHashMap<K, V>
where
    K: Eq + Hash,
    V: Clone,
{
    /// Create new, empty collection
    pub fn new() -> Self {
        Self(RwLock::new(HashMap::new()))
    }
}

impl<K, V, S> RwHashMap<K, V, S>
where
    K: Eq + Hash,
    V: Clone,
    S: BuildHasher,
{
    /// Lookup the key in hashmap
    pub fn get(&self, key: &K) -> Option<V> {
        self.0.read().unwrap().deref().get(key).cloned()
    }

    /// Insert a key-value pair to the hashmap
    pub fn insert(&self, key: K, value: V) {
        let mut cache = self.0.write().unwrap();
        cache.insert(key, value);
    }

    /// Get number of elements in the hashmap
    pub fn len(&self) -> usize {
        self.0.read().unwrap().len()
    }

    /// Check if hashmap is empty
    pub fn is_empty(&self) -> bool {
        self.0.read().unwrap().is_empty()
    }

    /// Remove all elements from hashmap
    pub fn clear(&self) {
        let mut cache = self.0.write().unwrap();
        cache.clear();
    }
}
