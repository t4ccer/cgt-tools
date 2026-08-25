use cgt::{
    grid::{FiniteGrid, Grid as _, vec_grid::VecGrid},
    short::partizan::Player,
};
use cgt_py_messages::Tile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Move {
    pub player: Player,
    pub stone: (u8, u8),
}

impl Move {
    pub fn is_live(self, grid: &VecGrid<Tile>) -> bool {
        holds_stone(grid, self.player, self.stone)
    }
}

pub const fn stone_tile(player: Player) -> Tile {
    match player {
        Player::Left => Tile::BlueStone,
        Player::Right => Tile::RedStone,
    }
}

pub fn holds_stone(grid: &VecGrid<Tile>, player: Player, tile: (u8, u8)) -> bool {
    on_board(grid, tile) && grid.get(tile.0, tile.1) == stone_tile(player)
}

pub fn jumped(
    grid: &VecGrid<Tile>,
    player: Player,
    from: (u8, u8),
    to: (u8, u8),
) -> Option<Vec<(u8, u8)>> {
    if !holds_stone(grid, player, from) || !on_board(grid, to) {
        return None;
    }

    let along_x = i32::from(to.0) - i32::from(from.0);
    let along_y = i32::from(to.1) - i32::from(from.1);

    if (along_x != 0) == (along_y != 0) {
        return None;
    }

    let distance = along_x.abs() + along_y.abs();
    if distance % 2 != 0 {
        return None;
    }

    let (step_x, step_y) = (along_x.signum(), along_y.signum());
    let captured = (1..=distance / 2)
        .map(|hop| {
            let at = |steps: i32| {
                (
                    (i32::from(from.0) + step_x * steps) as u8,
                    (i32::from(from.1) + step_y * steps) as u8,
                )
            };
            (at(2 * hop - 1), at(2 * hop))
        })
        .map(|(over, land)| {
            (grid.get(over.0, over.1) == stone_tile(player.opposite())
                && grid.get(land.0, land.1) == Tile::Empty)
                .then_some(over)
        })
        .collect::<Option<Vec<_>>>()?;

    Some(captured)
}

pub fn play(
    grid: &VecGrid<Tile>,
    player: Player,
    from: (u8, u8),
    to: (u8, u8),
) -> Option<VecGrid<Tile>> {
    let captured = jumped(grid, player, from, to)?;

    let mut played = grid.clone();
    played.set(from.0, from.1, Tile::Empty);
    for (x, y) in captured {
        played.set(x, y, Tile::Empty);
    }
    played.set(to.0, to.1, stone_tile(player));

    Some(played)
}

fn on_board(grid: &VecGrid<Tile>, (x, y): (u8, u8)) -> bool {
    x < grid.width() && y < grid.height()
}
