use crate::numeric::nimber::Nimber;
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
    pub fn pivot(&self, pivot: u32) -> Self {
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

    pub fn game(&self) -> Nimber {
        let moves = self.moves();
        let mut game_moves = Vec::with_capacity(moves.len());
        for m in moves {
            game_moves.push(m.game());
        }
        Nimber::mex(game_moves)
    }
}
