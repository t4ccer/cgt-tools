//! Thread safe transposition table for game values

use elsa::FrozenIndexSet;

// TODO: Move to short positional game module
use crate::{rw_hash_map::RwHashMap, short::partizan::canonical_form::CanonicalForm};
use std::{hash::Hash, sync::Mutex};

/// Transaction table (cache) of game positions and canonical forms.
pub struct TranspositionTable<'a, G> {
    known_games: Mutex<FrozenIndexSet<Box<CanonicalForm>>>,
    grids: RwHashMap<G, &'a CanonicalForm, ahash::RandomState>,
}

#[cfg_attr(feature = "cargo-clippy", allow(clippy::new_without_default))]
impl<'a, G> TranspositionTable<'a, G>
where
    G: Eq + Hash + Clone + Sync + Send,
{
    /// Create new empty transposition table.
    #[inline]
    pub fn new() -> Self {
        Self {
            known_games: Mutex::new(FrozenIndexSet::new()),
            grids: RwHashMap::with_hasher(ahash::RandomState::new()),
        }
    }

    // TODO: more generic names

    /// Lookup a position
    #[inline]
    pub fn grids_get(&self, grid: &G) -> Option<CanonicalForm> {
        self.grids.get(grid).cloned()
    }

    /// Save position and its game value
    #[cfg_attr(feature = "cargo-clippy", allow(clippy::missing_panics_doc))]
    #[inline]
    pub fn grids_insert(&'a self, grid: G, game: CanonicalForm) {
        let inserted: *const CanonicalForm =
            self.known_games.lock().unwrap().insert(Box::new(game));
        self.grids.insert(grid, unsafe { &*inserted });
    }

    /// Get number of saved games
    #[inline]
    pub fn grids_saved(&self) -> usize {
        self.grids.len()
    }
}
