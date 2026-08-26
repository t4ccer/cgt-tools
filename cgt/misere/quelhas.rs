//! Misere Domineering variant

#![allow(missing_docs)]

use crate::{
    drawing::{self, Canvas, Color, Draw},
    grid::{FiniteGrid, Grid},
    short::partizan::Player,
};

#[derive(Debug, Clone, Copy)]
pub enum Tile {
    Empty,
    Blue,
    Red,
}

impl From<Player> for Tile {
    fn from(player: Player) -> Tile {
        match player {
            Player::Left => Tile::Blue,
            Player::Right => Tile::Red,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Quelhas<G> {
    grid: G,
}

impl<G> Quelhas<G>
where
    G: Grid<Item = Tile>,
{
    pub const fn new(grid: G) -> Quelhas<G> {
        Quelhas { grid }
    }

    pub const fn grid(&self) -> &G {
        &self.grid
    }

    pub const fn grid_mut(&mut self) -> &mut G {
        &mut self.grid
    }
}

impl<G> Draw for Quelhas<G>
where
    G: Grid<Item = Tile> + FiniteGrid,
{
    fn draw<C>(&self, canvas: &mut C)
    where
        C: Canvas,
    {
        self.grid.draw(canvas, |tile| match tile {
            Tile::Empty => drawing::Tile::Square {
                color: Color::Surface,
            },
            Tile::Blue => drawing::Tile::Square { color: Color::Blue },
            Tile::Red => drawing::Tile::Square { color: Color::Red },
        });
    }
}
