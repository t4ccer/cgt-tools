//! Partizan games

pub mod canonical_form;
pub mod games;
pub mod partizan_game;
pub mod thermograph;
pub mod trajectory;
pub mod transposition_table;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[allow(missing_docs)]
pub enum Player {
    Left,
    Right,
}
