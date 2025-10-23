//! Partizan games

pub mod canonical_form;
pub mod games;
pub mod partizan_game;
pub mod thermograph;
pub mod trajectory;
pub mod transposition_table;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(missing_docs)]
pub enum Player {
    Left,
    Right,
}

impl Player {
    /// Opposite player
    #[inline(always)]
    #[must_use]
    pub const fn opposite(self) -> Player {
        match self {
            Player::Left => Player::Right,
            Player::Right => Player::Left,
        }
    }

    /// Run a predicate for both players
    #[inline(always)]
    pub fn forall<P>(mut predicate: P) -> bool
    where
        P: FnMut(Player) -> bool,
    {
        predicate(Player::Left) && predicate(Player::Right)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for Player {
        fn arbitrary(g: &mut Gen) -> Self {
            if Arbitrary::arbitrary(g) {
                Player::Left
            } else {
                Player::Right
            }
        }
    }
}
