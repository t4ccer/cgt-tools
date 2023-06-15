use std::{collections::HashMap, hash::Hash, ops::Deref, sync::RwLock};

#[derive(Debug)]
pub(crate) struct RwHashMap<K, V>(RwLock<HashMap<K, V>>);

impl<K, V> RwHashMap<K, V>
where
    K: Eq + Hash,
    V: Clone,
{
    pub(crate) fn new() -> Self {
        RwHashMap(RwLock::new(HashMap::new()))
    }

    pub(crate) fn get(&self, key: &K) -> Option<V> {
        self.0.read().unwrap().deref().get(key).cloned()
    }

    pub(crate) fn insert(&self, key: K, value: V) {
        let mut cache = self.0.write().unwrap();
        cache.insert(key, value);
    }

    pub(crate) fn len(&self) -> usize {
        self.0.read().unwrap().len()
    }

    pub(crate) fn clear(&self) {
        let mut cache = self.0.write().unwrap();
        cache.clear();
    }
}
