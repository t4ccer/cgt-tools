use std::{collections::HashMap, hash::Hash, ops::Deref, sync::RwLock};

#[derive(Debug)]
pub struct RwHashMap<K, V>(pub(crate) RwLock<HashMap<K, V>>);

impl<K, V> RwHashMap<K, V>
where
    K: Eq + Hash,
    V: Clone,
{
    pub fn new() -> Self {
        RwHashMap(RwLock::new(HashMap::new()))
    }

    pub fn get(&self, key: &K) -> Option<V> {
        self.0.read().unwrap().deref().get(key).cloned()
    }

    pub fn insert(&self, key: K, value: V) {
        let mut cache = self.0.write().unwrap();
        cache.insert(key, value);
    }

    pub fn len(&self) -> usize {
        self.0.read().unwrap().len()
    }

    pub fn clear(&self) {
        let mut cache = self.0.write().unwrap();
        cache.clear();
    }
}
