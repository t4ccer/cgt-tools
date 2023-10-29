//! Thread safe transposition table for game values

use crate::{rw_hash_map::RwHashMap, short::partizan::canonical_form::CanonicalForm};
use std::{hash::Hash, sync::Mutex};
use typed_arena::Arena;

/// Transaction table (cache) of game positions and canonical forms.
pub struct TranspositionTable<'a, G> {
    known_games: Mutex<Arena<CanonicalForm>>,
    grids: RwHashMap<G, &'a CanonicalForm, ahash::RandomState>,
}

#[cfg_attr(feature = "cargo-clippy", allow(clippy::new_without_default))]
impl<'a, G> TranspositionTable<'a, G>
where
    G: Eq + Hash + Sync + Send,
{
    /// Create new empty transposition table.
    #[inline]
    pub fn new() -> Self {
        Self {
            known_games: Mutex::new(Arena::new()),
            grids: RwHashMap::with_hasher(ahash::RandomState::new()),
        }
    }

    // TODO: more generic names

    /// Lookup a position
    #[inline]
    pub fn grids_get(&'a self, grid: &G) -> Option<&'a CanonicalForm> {
        self.grids.get(grid)
    }

    /// Save position and its game value
    #[cfg_attr(feature = "cargo-clippy", allow(clippy::missing_panics_doc))]
    #[inline]
    pub fn grids_insert(&'a self, grid: G, game: CanonicalForm) {
        let arena = self.known_games.lock().unwrap();
        let inserted = arena.alloc(game) as *const CanonicalForm;
        drop(arena);
        self.grids.insert(grid, unsafe { &*inserted });
    }

    /// Get number of saved games
    #[inline]
    pub fn grids_saved(&self) -> usize {
        self.grids.len()
    }
}
