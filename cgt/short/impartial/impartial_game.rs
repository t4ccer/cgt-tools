//! Impartial game - both players have the same moves

use crate::numeric::nimber::Nimber;

/// Impartial game
pub trait ImpartialGame: Sized {
    /// Get a list of moves from the position
    fn moves(&self) -> Vec<Self>;

    /// Calculate the Nim value of the position
    fn nim_value(&self) -> Nimber {
        let moves = self.moves();
        let mut game_moves = Vec::with_capacity(moves.len());
        for m in moves {
            game_moves.push(m.nim_value());
        }
        Nimber::mex(game_moves)
    }
}
