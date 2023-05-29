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
}

// serde cannot derive it and breaks on constraints

#[cfg(feature = "serde")]
impl<K, V> serde::Serialize for RwHashMap<K, V>
where
    K: Eq + Hash + serde::Serialize,
    V: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde::Serialize::serialize(self.0.read().unwrap().deref(), serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, K, V> serde::Deserialize<'de> for RwHashMap<K, V>
where
    K: Eq + Hash + serde::Deserialize<'de>,
    V: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let hash_map = serde::Deserialize::deserialize(deserializer)?;
        Ok(RwHashMap(RwLock::new(hash_map)))
    }
}
