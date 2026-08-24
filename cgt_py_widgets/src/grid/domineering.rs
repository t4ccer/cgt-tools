use crate::canvas::HtmlCanvas;
use cgt::{
    drawing::Canvas,
    grid::{FiniteGrid, Grid as _, vec_grid::VecGrid},
    numeric::v2f::V2f,
    short::partizan::Player,
};
use cgt_py_messages::Tile;

pub fn hovered_domino(
    grid: &VecGrid<Tile>,
    player: Player,
    (x, y): (u8, u8),
    cursor: V2f,
) -> Option<[(u8, u8); 2]> {
    let tile_size = HtmlCanvas::tile_size();
    let origin = HtmlCanvas::tile_position(x, y);

    let (position, length, past_middle) = match player {
        Player::Left => (y, grid.height(), cursor.y - origin.y > tile_size.y * 0.5),
        Player::Right => (x, grid.width(), cursor.x - origin.x > tile_size.x * 0.5),
    };

    let onwards = (position + 1 < length).then_some(position);
    let back = position.checked_sub(1);

    let (preferred, fallback) = if past_middle {
        (onwards, back)
    } else {
        (back, onwards)
    };

    [preferred, fallback]
        .into_iter()
        .flatten()
        .map(|first| match player {
            Player::Left => [(x, first), (x, first + 1)],
            Player::Right => [(first, y), (first + 1, y)],
        })
        .find(|domino| domino_fits(grid, *domino))
}

pub fn domino_fits(grid: &VecGrid<Tile>, domino: [(u8, u8); 2]) -> bool {
    domino
        .into_iter()
        .all(|(x, y)| grid.get(x, y) == Tile::Empty)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgt::grid::Grid;

    /// Tiles are 64px with a 2px line between them, so tile (x, y) starts at
    /// x * 64 + (x + 1) * 2 and its middle is 32px further in
    fn middle_of(x: u8, y: u8) -> V2f {
        HtmlCanvas::tile_position(x, y) + HtmlCanvas::tile_size() * 0.5
    }

    fn empty(width: u8, height: u8) -> VecGrid<Tile> {
        FiniteGrid::filled(width, height, Tile::Empty).unwrap()
    }

    /// Cursor `offset` px from the middle of the tile, negative being up / left
    fn off_middle(x: u8, y: u8, player: Player, offset: f32) -> V2f {
        let middle = middle_of(x, y);
        match player {
            Player::Left => V2f {
                x: middle.x,
                y: middle.y + offset,
            },
            Player::Right => V2f {
                x: middle.x + offset,
                y: middle.y,
            },
        }
    }

    #[test]
    fn reaches_towards_the_hovered_half() {
        let grid = empty(3, 3);

        // Left plays vertical dominoes: above the middle reaches up, below reaches down
        assert_eq!(
            hovered_domino(
                &grid,
                Player::Left,
                (1, 1),
                off_middle(1, 1, Player::Left, -20.0)
            ),
            Some([(1, 0), (1, 1)])
        );
        assert_eq!(
            hovered_domino(
                &grid,
                Player::Left,
                (1, 1),
                off_middle(1, 1, Player::Left, 20.0)
            ),
            Some([(1, 1), (1, 2)])
        );

        // Right plays horizontal ones
        assert_eq!(
            hovered_domino(
                &grid,
                Player::Right,
                (1, 1),
                off_middle(1, 1, Player::Right, -20.0)
            ),
            Some([(0, 1), (1, 1)])
        );
        assert_eq!(
            hovered_domino(
                &grid,
                Player::Right,
                (1, 1),
                off_middle(1, 1, Player::Right, 20.0)
            ),
            Some([(1, 1), (2, 1)])
        );
    }

    #[test]
    fn edge_tiles_reach_the_only_way_they_can() {
        let grid = empty(3, 3);

        // Top row hovered in its top half still reaches down, bottom row still reaches up
        assert_eq!(
            hovered_domino(
                &grid,
                Player::Left,
                (1, 0),
                off_middle(1, 0, Player::Left, -20.0)
            ),
            Some([(1, 0), (1, 1)])
        );
        assert_eq!(
            hovered_domino(
                &grid,
                Player::Left,
                (1, 2),
                off_middle(1, 2, Player::Left, 20.0)
            ),
            Some([(1, 1), (1, 2)])
        );

        assert_eq!(
            hovered_domino(
                &grid,
                Player::Right,
                (0, 1),
                off_middle(0, 1, Player::Right, -20.0)
            ),
            Some([(0, 1), (1, 1)])
        );
        assert_eq!(
            hovered_domino(
                &grid,
                Player::Right,
                (2, 1),
                off_middle(2, 1, Player::Right, 20.0)
            ),
            Some([(1, 1), (2, 1)])
        );
    }

    #[test]
    fn no_domino_fits_across_a_single_line() {
        // One row: Left has nowhere to put a vertical domino, Right is fine
        let row = empty(3, 1);
        assert_eq!(
            hovered_domino(&row, Player::Left, (1, 0), middle_of(1, 0)),
            None
        );
        assert!(hovered_domino(&row, Player::Right, (1, 0), middle_of(1, 0)).is_some());

        // One column: the other way round
        let column = empty(1, 3);
        assert_eq!(
            hovered_domino(&column, Player::Right, (0, 1), middle_of(0, 1)),
            None
        );
        assert!(hovered_domino(&column, Player::Left, (0, 1), middle_of(0, 1)).is_some());
    }

    #[test]
    fn taken_tiles_block_the_domino() {
        let mut grid = empty(3, 3);
        grid.set(1, 0, Tile::Taken);

        assert!(!domino_fits(&grid, [(1, 0), (1, 1)]));
        assert!(domino_fits(&grid, [(1, 1), (1, 2)]));
    }

    #[test]
    fn a_taken_neighbour_is_as_good_as_an_edge() {
        // The tile above (1, 1) is taken, so hovering the top half of it reaches down
        // rather than up, exactly as it would in the top row
        let mut grid = empty(3, 3);
        grid.set(1, 0, Tile::Taken);
        assert_eq!(
            hovered_domino(
                &grid,
                Player::Left,
                (1, 1),
                off_middle(1, 1, Player::Left, -20.0)
            ),
            Some([(1, 1), (1, 2)])
        );

        // And the same horizontally, reaching back from a taken tile to the right
        let mut grid = empty(3, 3);
        grid.set(2, 1, Tile::Taken);
        assert_eq!(
            hovered_domino(
                &grid,
                Player::Right,
                (1, 1),
                off_middle(1, 1, Player::Right, 20.0)
            ),
            Some([(0, 1), (1, 1)])
        );
    }

    #[test]
    fn walled_in_tiles_take_no_domino() {
        // Both neighbours along Left's axis are taken, so neither way out is left
        let mut grid = empty(3, 3);
        grid.set(1, 0, Tile::Taken);
        grid.set(1, 2, Tile::Taken);
        assert_eq!(
            hovered_domino(&grid, Player::Left, (1, 1), middle_of(1, 1)),
            None
        );

        // A taken tile has nowhere to put a domino either, whichever way it reaches
        let mut grid = empty(3, 3);
        grid.set(1, 1, Tile::Taken);
        assert_eq!(
            hovered_domino(&grid, Player::Left, (1, 1), middle_of(1, 1)),
            None
        );
        assert_eq!(
            hovered_domino(&grid, Player::Right, (1, 1), middle_of(1, 1)),
            None
        );
    }
}
