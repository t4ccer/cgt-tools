use cgt::{
    grid::{FiniteGrid, Grid as _, vec_grid::VecGrid},
    short::partizan::Player,
};
use cgt_py_messages::Tile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Move {
    pub player: Player,
    pub queen: (u8, u8),
    pub target: Option<(u8, u8)>,
}

impl Move {
    pub fn is_live(self, grid: &VecGrid<Tile>) -> bool {
        holds_queen(grid, self.player, self.queen)
            && self
                .target
                .is_none_or(|target| can_reach(grid, self.queen, target))
    }
}

pub const fn queen_tile(player: Player) -> Tile {
    match player {
        Player::Left => Tile::BlueStone,
        Player::Right => Tile::RedStone,
    }
}

pub fn holds_queen(grid: &VecGrid<Tile>, player: Player, tile: (u8, u8)) -> bool {
    on_board(grid, tile) && grid.get(tile.0, tile.1) == queen_tile(player)
}

pub fn can_reach(grid: &VecGrid<Tile>, from: (u8, u8), to: (u8, u8)) -> bool {
    if from == to || !on_board(grid, from) || !on_board(grid, to) {
        return false;
    }

    let along_x = i32::from(to.0) - i32::from(from.0);
    let along_y = i32::from(to.1) - i32::from(from.1);
    let steps = i32::max(along_x.abs(), along_y.abs());

    if along_x != 0 && along_y != 0 && along_x.abs() != along_y.abs() {
        return false;
    }

    (1..=steps).all(|step| {
        let x = i32::from(from.0) + along_x.signum() * step;
        let y = i32::from(from.1) + along_y.signum() * step;
        grid.get(x as u8, y as u8) == Tile::Empty
    })
}

pub fn play(
    grid: &VecGrid<Tile>,
    player: Player,
    queen: (u8, u8),
    target: (u8, u8),
    stone: (u8, u8),
) -> Option<VecGrid<Tile>> {
    if !holds_queen(grid, player, queen) || !can_reach(grid, queen, target) {
        return None;
    }

    let mut played = grid.clone();
    played.set(queen.0, queen.1, Tile::Empty);
    played.set(target.0, target.1, queen_tile(player));

    if !can_reach(&played, target, stone) {
        return None;
    }

    played.set(stone.0, stone.1, Tile::Taken);
    Some(played)
}

fn on_board(grid: &VecGrid<Tile>, (x, y): (u8, u8)) -> bool {
    x < grid.width() && y < grid.height()
}
