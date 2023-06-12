use crate::{
    rw_hash_map::RwHashMap,
    short_canonical_game::{Game, GameBackend},
};
use std::hash::Hash;

/// Transaction table (cache) of game positions and canonical forms.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound = "G: serde::Serialize + serde::de::DeserializeOwned + Eq + Hash")
)]
pub struct TranspositionTable<G> {
    grids: RwHashMap<G, Game>,
    game_backend: GameBackend,
}

impl<G> TranspositionTable<G>
where
    G: Eq + Hash,
{
    /// Create new empty transposition table.
    #[inline]
    pub fn new() -> Self {
        TranspositionTable::with_game_backend(GameBackend::new())
    }

    /// Create new transposition table with pre-existing game backend.
    /// Useful if you load game backend from file, or re-use it from earlier computations.
    #[inline]
    pub fn with_game_backend(game_backend: GameBackend) -> Self {
        TranspositionTable {
            grids: RwHashMap::new(),
            game_backend,
        }
    }

    /// Get the underlying game storage backend.
    #[inline]
    pub fn game_backend(&self) -> &GameBackend {
        &self.game_backend
    }

    #[inline]
    pub fn grids_get(&self, grid: &G) -> Option<Game> {
        self.grids.get(grid)
    }

    #[inline]
    pub fn grids_insert(&self, grid: G, game: Game) {
        self.grids.insert(grid, game)
    }

    #[inline]
    pub fn grids_saved(&self) -> usize {
        self.grids.len()
    }
}
