//! Misere Domineering variant

#![allow(missing_docs)]

use crate::{
    drawing::{self, BoundingBox, Canvas, Color, Draw},
    grid::{FiniteGrid, Grid},
};

#[derive(Debug, Clone, Copy)]
pub enum Tile {
    Empty,
    Blue,
    Red,
}

#[derive(Debug, Clone, Copy)]
pub struct Quelhas<G> {
    grid: G,
}

impl<G> Quelhas<G>
where
    G: Grid<Item = Tile>,
{
    pub fn new(grid: G) -> Quelhas<G> {
        Quelhas { grid }
    }

    pub fn grid(&self) -> &G {
        &self.grid
    }

    pub fn grid_mut(&mut self) -> &mut G {
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
                color: Color::LIGHT_GRAY,
            },
            Tile::Blue => drawing::Tile::Square { color: Color::BLUE },
            Tile::Red => drawing::Tile::Square { color: Color::RED },
        });
    }

    fn required_canvas<C>(&self) -> BoundingBox
    where
        C: Canvas,
    {
        self.grid().canvas_size::<C>()
    }
}
