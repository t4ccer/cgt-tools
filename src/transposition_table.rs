// TODO: Move to short positional game module
use crate::{
    rw_hash_map::RwHashMap,
    short_canonical_game::{Game, GameBackend, PlacementGame},
};
use std::hash::Hash;

/// Transaction table (cache) of game positions and canonical forms.
pub struct TranspositionTable<G> {
    grids: Vec<RwHashMap<G, Game>>,
    game_backend: GameBackend,
}

impl<G> TranspositionTable<G>
where
    G: Eq + Hash + Clone + Sync + Send + PlacementGame,
{
    /// Create new empty transposition table.
    #[inline]
    pub fn new(max_classes: u8) -> Self {
        TranspositionTable::with_game_backend(GameBackend::new(), max_classes)
    }

    /// Create new transposition table with pre-existing game backend.
    /// Useful if you load game backend from file, or re-use it from earlier computations.
    #[inline]
    pub fn with_game_backend(game_backend: GameBackend, max_classes: u8) -> Self {
        let mut grids = Vec::with_capacity(max_classes as usize);
        for _ in 0..=max_classes {
            grids.push(RwHashMap::new());
        }

        TranspositionTable {
            grids,
            game_backend,
        }
    }

    /// Get the underlying game storage backend.
    #[inline]
    pub fn game_backend(&self) -> &GameBackend {
        &self.game_backend
    }

    #[inline]
    fn get_cache<'a>(&'a self, grid: &G) -> &'a RwHashMap<G, Game> {
        let idx = grid.free_places();
        &self.grids[idx]
    }

    #[inline]
    pub fn grids_get(&self, grid: &G) -> Option<Game> {
        self.get_cache(grid).get(grid)
    }

    #[inline]
    pub fn grids_insert(&self, grid: G, game: Game) {
        self.get_cache(&grid).insert(grid, game);
    }

    #[inline]
    pub fn grids_saved(&self) -> usize {
        self.grids.iter().map(|cache| cache.len()).sum::<usize>()
    }

    #[inline]
    pub fn clear_class(&self, class: usize) {
        self.grids[class].clear();
    }
}
