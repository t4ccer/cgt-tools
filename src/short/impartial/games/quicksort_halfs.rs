use crate::short::partizan::short_canonical_game::{Game, GameBackend, Moves};
use std::fmt::Display;

#[derive(Debug, PartialEq, Eq)]
pub struct Quicksort(pub Vec<u32>);

impl Display for Quicksort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for elem in &self.0 {
            write!(f, "{}", elem)?;
        }
        Ok(())
    }
}

impl Quicksort {
    /// pivot on `pivot+0.5`
    fn pivot(&self, pivot: u32) -> Self {
        let mut res = Quicksort(Vec::with_capacity(self.0.len()));
        for elem in &self.0 {
            if *elem <= pivot {
                res.0.push(*elem);
            }
        }
        for elem in &self.0 {
            if *elem > pivot {
                res.0.push(*elem);
            }
        }
        res
    }

    pub fn moves(&self) -> Vec<Self> {
        let mut res = vec![];
        for pivot in 1..self.0.len() {
            let new = self.pivot(pivot as u32);
            if !res.contains(&new) && &new != self {
                res.push(new);
            }
        }
        res
    }

    pub fn game(&self, b: &GameBackend) -> Game {
        let moves = self.moves();
        let mut game_moves = Vec::with_capacity(moves.len());
        for m in moves {
            game_moves.push(m.game(b));
        }
        b.construct_from_moves(Moves {
            left: game_moves.clone(),
            right: game_moves,
        })
    }
}

#[test]
fn qs() {
    let game = Quicksort(vec![4, 2, 3, 1]);
    for m in game.moves() {
        eprintln!("{m}");
    }
    panic!();
}
