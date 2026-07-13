//! Capacity bounded, moka-based transposition table

use crate::short::partizan::{
    canonical_form::CanonicalForm, transposition_table::TranspositionTable,
};
use moka::sync::Cache;
use std::{fmt::Debug, hash::Hash};

/// Transaction table (cache) of game positions and canonical forms.
pub struct CacheTranspositionTable<G> {
    cache: Cache<G, CanonicalForm, crate::hash::RandomState>,
}

impl<G> Debug for CacheTranspositionTable<G>
where
    G: Debug + Eq + Hash + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CacheTranspositionTable")
            .field("cache", &self.cache)
            .finish()
    }
}

impl<G> CacheTranspositionTable<G>
where
    G: Eq + Hash + Send + Sync + 'static,
{
    /// Create new empty transposition table.
    #[inline]
    pub fn new(max_capacity: u64) -> Self {
        Self {
            cache: Cache::builder()
                .max_capacity(max_capacity)
                .build_with_hasher(crate::hash::RandomState::new()),
        }
    }

    /// Get number of saved positions
    #[inline]
    pub fn len(&self) -> usize {
        self.cache.entry_count() as usize
    }

    /// Check if table stores any position
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.cache.entry_count() == 0
    }
}

impl<G> TranspositionTable<G> for CacheTranspositionTable<G>
where
    G: Eq + Hash + Send + Sync + 'static,
{
    #[inline]
    fn lookup_position(&self, position: &G) -> Option<CanonicalForm> {
        self.cache.get(position)
    }

    #[inline]
    fn insert_position(&self, position: G, value: CanonicalForm) {
        self.cache.insert(position, value);
    }
}
