//! Shared traits for short partizan games

use crate::{
    numeric::rational::Rational,
    short::partizan::{
        canonical_form::{CanonicalForm, Moves},
        thermograph::Thermograph,
        trajectory::Trajectory,
        transposition_table::TranspositionTable,
    },
};
use std::hash::Hash;

#[cfg(feature = "rayon")]
use rayon::prelude::{IntoParallelIterator, ParallelIterator};

/// A short partizan game
pub trait PartizanGame: Sized + Clone + Hash + Send + Sync + Eq {
    /// List of all moves for the Left player
    fn left_moves(&self) -> Vec<Self>;

    /// List of all moves for the Right player
    fn right_moves(&self) -> Vec<Self>;

    /// Split game into disjoint sum
    ///
    /// Note that default implementation doesn't find any decompisitons and may lead to performance
    /// issues
    fn decompositions(&self) -> Vec<Self> {
        vec![self.clone()]
    }

    /// Compute the thermograph without going through canonical form
    /// Taken from Elwyn Berlekamp - The Economist’s View of Combinatorial Games
    /// This is copy-pasted from [`super::canonical_form`] module, but works on positions rather than
    /// it's canonical forms, although the algorithm is the same
    ///
    /// Note that for some games, going through canonical form to compute the thermograph may
    /// be faster
    ///
    /// See: zubzero-thermography
    fn thermograph_direct(&self) -> Thermograph {
        let left_moves = self.left_moves();
        let right_moves = self.right_moves();
        if left_moves.is_empty() && right_moves.is_empty() {
            return Thermograph::with_mast(Rational::from(0));
        }

        let mut left_scaffold = Trajectory::new_constant(Rational::NegativeInfinity);
        let mut right_scaffold = Trajectory::new_constant(Rational::PositiveInfinity);

        for left_move in &left_moves {
            left_scaffold = left_scaffold.max(&left_move.thermograph_direct().right_wall);
        }
        for right_move in &right_moves {
            right_scaffold = right_scaffold.min(&right_move.thermograph_direct().left_wall);
        }

        left_scaffold.tilt(Rational::from(-1));
        right_scaffold.tilt(Rational::from(1));

        Thermograph::thermographic_intersection(left_scaffold, right_scaffold)
    }

    /// Handle special cases when computing canonical form doesn't have to compute all moves.
    fn reductions(&self) -> Option<CanonicalForm> {
        None
    }

    /// Get the canonical form of the game position
    fn canonical_form<TT>(&self, transposition_table: &TT) -> CanonicalForm
    where
        TT: TranspositionTable<Self> + Sync,
    {
        if let Some(id) = transposition_table.lookup_position(self) {
            return id;
        }

        if let Some(cf) = self.reductions() {
            return cf;
        }

        #[cfg(feature = "rayon")]
        let decompositions = self.decompositions().into_par_iter();
        #[cfg(not(feature = "rayon"))]
        let decompositions = self.decompositions().into_iter();

        let sub_results = decompositions.map(|position| {
            transposition_table.lookup_position(&position).map_or_else(
                || {
                    #[cfg(feature = "rayon")]
                    let left = position.left_moves().into_par_iter();
                    #[cfg(not(feature = "rayon"))]
                    let left = position.left_moves().into_iter();

                    #[cfg(feature = "rayon")]
                    let right = position.right_moves().into_par_iter();
                    #[cfg(not(feature = "rayon"))]
                    let right = position.right_moves().into_iter();

                    let moves = Moves {
                        left: left
                            .map(|o| o.canonical_form(transposition_table))
                            .collect(),
                        right: right
                            .map(|o| o.canonical_form(transposition_table))
                            .collect(),
                    };

                    CanonicalForm::new_from_moves(moves)
                },
                |cached_sub_result| cached_sub_result,
            )
        });

        #[cfg(feature = "rayon")]
        let result = sub_results.reduce(|| CanonicalForm::new_integer(0), |a, b| a + b);
        #[cfg(not(feature = "rayon"))]
        let result = sub_results.fold(CanonicalForm::new_integer(0), |a, b| a + b);

        transposition_table.insert_position(self.clone(), result.clone());
        result
    }

    // TODO: Find a way to reduce duplication - maybe macro?

    /// List of canonical moves for the Left player
    fn sensible_left_moves<TT>(&self, transposition_table: &TT) -> Vec<Self>
    where
        TT: TranspositionTable<Self> + Sync,
    {
        let canonical_form = self.canonical_form(transposition_table);
        let moves = canonical_form.to_moves();

        self.left_moves()
            .into_iter()
            .filter(|m| {
                let move_game_form = m.canonical_form(transposition_table);
                let res = moves.left.iter().any(|k| k <= &move_game_form);
                res
            })
            .collect::<Vec<_>>()
    }

    /// List of canonical moves for the Right player
    fn sensible_right_moves<TT>(&self, transposition_table: &TT) -> Vec<Self>
    where
        TT: TranspositionTable<Self> + Sync,
    {
        let canonical_form = self.canonical_form(transposition_table);
        let moves = canonical_form.to_moves();

        self.right_moves()
            .into_iter()
            .filter(|m| {
                let move_game_form = m.canonical_form(transposition_table);
                let res = moves.right.iter().any(|k| k >= &move_game_form);
                res
            })
            .collect::<Vec<_>>()
    }
}
