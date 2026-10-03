//! Quelhas on a 10 by 10 board: Left crosses out vertical segments of at least two empty
//! squares, Right horizontal ones, and whoever makes the last move loses.

use crate::ruleset::{InputShape, Player, Ruleset};
use std::{fmt, str::FromStr};

pub const BOARD_SIZE: usize = 10;
pub const NUM_CELLS: usize = BOARD_SIZE * BOARD_SIZE;
pub const MIN_LENGTH: usize = 2;
pub const NUM_LENGTHS: usize = BOARD_SIZE - MIN_LENGTH + 1;
pub const NUM_ACTIONS: usize = NUM_LENGTHS * NUM_CELLS;

// planes: [empty, mover_playable, opponent_playable, vertical_run/size, horizontal_run/size, ones]
pub const NUM_PLANES: usize = 6;
pub const FEATURES_LEN: usize = NUM_PLANES * NUM_CELLS;

/// Row and column flips map vertical segments to vertical segments, so they
/// are the symmetries of the canonical frame.
pub const SYMMETRIES: [(bool, bool); 4] =
    [(false, false), (true, false), (false, true), (true, true)];

/// Set of empty cells; bit `row * BOARD_SIZE + col` is set when the cell is empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Board(u128);

impl Board {
    pub const FULL: Board = Board((1 << NUM_CELLS) - 1);

    pub const fn from_bits(bits: u128) -> Board {
        Board(bits & Board::FULL.0)
    }

    pub const fn bits(self) -> u128 {
        self.0
    }

    pub const fn is_empty(self, row: usize, col: usize) -> bool {
        self.0 >> (row * BOARD_SIZE + col) & 1 == 1
    }

    pub fn is_empty_at(self, row: isize, col: isize) -> bool {
        let size = BOARD_SIZE as isize;
        (0..size).contains(&row)
            && (0..size).contains(&col)
            && self.is_empty(row as usize, col as usize)
    }

    pub const fn cross(&mut self, row: usize, col: usize) {
        self.0 &= !(1 << (row * BOARD_SIZE + col));
    }

    fn map_cells(self, f: impl Fn(usize, usize) -> (usize, usize)) -> Board {
        let mut out = 0;
        for row in 0..BOARD_SIZE {
            for col in 0..BOARD_SIZE {
                if self.is_empty(row, col) {
                    let (r, c) = f(row, col);
                    out |= 1 << (r * BOARD_SIZE + c);
                }
            }
        }
        Board(out)
    }

    #[must_use]
    pub fn transpose(self) -> Board {
        self.map_cells(|r, c| (c, r))
    }

    #[must_use]
    pub fn flip_rows(self) -> Board {
        self.map_cells(|r, c| (BOARD_SIZE - 1 - r, c))
    }

    #[must_use]
    pub fn flip_cols(self) -> Board {
        self.map_cells(|r, c| (r, BOARD_SIZE - 1 - c))
    }

    /// Indices of the vertical segments that can be crossed out, in increasing order.
    fn legal_indices(self) -> Vec<usize> {
        let mut indices = Vec::new();
        for (block, masks) in ACTION_MASKS.chunks(NUM_CELLS).enumerate() {
            let before = indices.len();
            indices.extend(
                masks
                    .iter()
                    .enumerate()
                    .filter(|&(_, &mask)| mask != 0 && self.0 & mask == mask)
                    .map(|(i, _)| block * NUM_CELLS + i),
            );
            // a segment of length n + 1 contains one of length n
            if indices.len() == before {
                break;
            }
        }
        indices
    }

    pub fn legal_actions(self) -> Vec<Action> {
        self.legal_indices()
            .into_iter()
            .map(|i| Action(i as u16))
            .collect()
    }

    pub fn has_legal_action(self) -> bool {
        ACTION_MASKS[..NUM_CELLS]
            .iter()
            .any(|&mask| mask != 0 && self.0 & mask == mask)
    }

    pub fn count_legal_actions(self) -> usize {
        (0..BOARD_SIZE)
            .map(|col| {
                let mut total = 0;
                let mut run = 0;
                for row in 0..=BOARD_SIZE {
                    if row < BOARD_SIZE && self.is_empty(row, col) {
                        run += 1;
                    } else {
                        total += run * (run.max(1) - 1) / 2;
                        run = 0;
                    }
                }
                total
            })
            .sum()
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for row in 0..BOARD_SIZE {
            if row > 0 {
                writeln!(f)?;
            }
            for col in 0..BOARD_SIZE {
                write!(f, "{}", if self.is_empty(row, col) { '.' } else { '#' })?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseBoardError;

impl fmt::Display for ParseBoardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "expected {BOARD_SIZE} rows of {BOARD_SIZE} '.' or '#' cells"
        )
    }
}

impl std::error::Error for ParseBoardError {}

impl FromStr for Board {
    type Err = ParseBoardError;

    fn from_str(s: &str) -> Result<Board, ParseBoardError> {
        let rows: Vec<&str> = s.split_whitespace().collect();
        if rows.len() != BOARD_SIZE || rows.iter().any(|r| r.chars().count() != BOARD_SIZE) {
            return Err(ParseBoardError);
        }
        let mut board = Board::FULL;
        for (row, line) in rows.iter().enumerate() {
            for (col, ch) in line.chars().enumerate() {
                match ch {
                    '.' => {}
                    '#' => board.cross(row, col),
                    _ => return Err(ParseBoardError),
                }
            }
        }
        Ok(board)
    }
}

// Actions live in the mover's canonical frame: the board as Left sees it, and
// its transpose for Right, so that the mover always crosses a vertical segment.
// Action a = (length - MIN_LENGTH) * NUM_CELLS + row * BOARD_SIZE + col covers
// canonical cells (row, col) .. (row + length - 1, col).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Action(u16);

const ACTION_MASKS: [u128; NUM_ACTIONS] = {
    let mut masks = [0; NUM_ACTIONS];
    let mut a = 0;
    while a < NUM_ACTIONS {
        let length = a / NUM_CELLS + MIN_LENGTH;
        let row = a % NUM_CELLS / BOARD_SIZE;
        let col = a % BOARD_SIZE;
        if row + length <= BOARD_SIZE {
            let mut k = 0;
            while k < length {
                masks[a] |= 1 << ((row + k) * BOARD_SIZE + col);
                k += 1;
            }
        }
        a += 1;
    }
    masks
};

impl Action {
    pub const fn new(length: usize, row: usize, col: usize) -> Option<Action> {
        if length >= MIN_LENGTH && col < BOARD_SIZE && row + length <= BOARD_SIZE {
            Some(Action(
                ((length - MIN_LENGTH) * NUM_CELLS + row * BOARD_SIZE + col) as u16,
            ))
        } else {
            None
        }
    }

    pub const fn from_index(index: usize) -> Option<Action> {
        if index < NUM_ACTIONS && ACTION_MASKS[index] != 0 {
            Some(Action(index as u16))
        } else {
            None
        }
    }

    pub fn all() -> impl Iterator<Item = Action> {
        (0..NUM_ACTIONS).filter_map(Action::from_index)
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }

    pub const fn length(self) -> usize {
        self.index() / NUM_CELLS + MIN_LENGTH
    }

    pub const fn row(self) -> usize {
        self.index() % NUM_CELLS / BOARD_SIZE
    }

    pub const fn col(self) -> usize {
        self.index() % BOARD_SIZE
    }

    /// Cells crossed out when `turn` plays this action, in board coordinates.
    pub fn cells(self, turn: Player) -> impl Iterator<Item = (usize, usize)> {
        let (row, col) = (self.row(), self.col());
        (0..self.length()).map(move |k| match turn {
            Player::Left => (row + k, col),
            Player::Right => (col, row + k),
        })
    }

    pub const fn segment(self, turn: Player) -> ((usize, usize), (usize, usize)) {
        let (row, col, last) = (self.row(), self.col(), self.length() - 1);
        match turn {
            Player::Left => ((row, col), (row + last, col)),
            Player::Right => ((col, row), (col, row + last)),
        }
    }

    pub fn from_segment(turn: Player, a: (usize, usize), b: (usize, usize)) -> Option<Action> {
        let ((r0, c0), (r1, c1)) = if a <= b { (a, b) } else { (b, a) };
        let ((r0, c0), (r1, c1)) = match turn {
            Player::Left => ((r0, c0), (r1, c1)),
            Player::Right => ((c0, r0), (c1, r1)),
        };
        if c0 != c1 {
            return None;
        }
        Action::new(r1 - r0 + 1, r0, c0)
    }

    #[must_use]
    pub const fn flip(self, flip_rows: bool, flip_cols: bool) -> Action {
        let (length, mut row, mut col) = (self.length(), self.row(), self.col());
        if flip_rows {
            row = BOARD_SIZE - length - row;
        }
        if flip_cols {
            col = BOARD_SIZE - 1 - col;
        }
        match Action::new(length, row, col) {
            Some(action) => action,
            None => unreachable!(),
        }
    }

    const fn mask(self) -> u128 {
        ACTION_MASKS[self.index()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct State {
    pub empty: Board,
    pub turn: Player,
}

impl State {
    pub const fn initial() -> State {
        State {
            empty: Board::FULL,
            turn: Player::Left,
        }
    }

    pub fn canonical(self) -> Board {
        match self.turn {
            Player::Left => self.empty,
            Player::Right => self.empty.transpose(),
        }
    }

    pub fn from_canonical(board: Board, turn: Player) -> State {
        let empty = match turn {
            Player::Left => board,
            Player::Right => board.transpose(),
        };
        State { empty, turn }
    }

    pub fn legal_actions(self) -> Vec<Action> {
        self.canonical().legal_actions()
    }

    pub fn is_legal(self, action: Action) -> bool {
        self.canonical().0 & action.mask() == action.mask()
    }

    pub fn is_terminal(self) -> bool {
        !self.canonical().has_legal_action()
    }

    /// Misère play: whoever cannot move on their turn did not make the last move, so wins.
    pub fn winner(self) -> Option<Player> {
        self.is_terminal().then_some(self.turn)
    }

    #[must_use]
    pub fn apply(self, action: Action) -> State {
        assert!(
            self.is_legal(action),
            "illegal action {action:?} for {:?}",
            self.turn
        );
        let mut empty = self.empty;
        for (row, col) in action.cells(self.turn) {
            empty.cross(row, col);
        }
        State {
            empty,
            turn: self.turn.opposite(),
        }
    }

    /// Flips the board as the player to move sees it.
    #[must_use]
    pub fn flip(self, flip_rows: bool, flip_cols: bool) -> State {
        let mut board = self.canonical();
        if flip_rows {
            board = board.flip_rows();
        }
        if flip_cols {
            board = board.flip_cols();
        }
        State::from_canonical(board, self.turn)
    }
}

fn run_length(board: Board, row: usize, col: usize, dr: isize, dc: isize) -> usize {
    let (mut r, mut c) = (row as isize, col as isize);
    let mut n = 0;
    while board.is_empty_at(r, c) {
        n += 1;
        r += dr;
        c += dc;
    }
    n
}

/// Encodes a canonical board (the mover crosses vertically) into `out`.
pub fn encode(board: Board, out: &mut [f32]) {
    assert_eq!(out.len(), FEATURES_LEN);
    out.fill(0.0);
    let size = BOARD_SIZE as f32;
    for row in 0..BOARD_SIZE {
        for col in 0..BOARD_SIZE {
            let i = row * BOARD_SIZE + col;
            out[5 * NUM_CELLS + i] = 1.0;
            if !board.is_empty(row, col) {
                continue;
            }
            let (r, c) = (row as isize, col as isize);
            out[i] = 1.0;
            if board.is_empty_at(r - 1, c) || board.is_empty_at(r + 1, c) {
                out[NUM_CELLS + i] = 1.0;
            }
            if board.is_empty_at(r, c - 1) || board.is_empty_at(r, c + 1) {
                out[2 * NUM_CELLS + i] = 1.0;
            }
            let vertical =
                run_length(board, row, col, -1, 0) + run_length(board, row, col, 1, 0) - 1;
            let horizontal =
                run_length(board, row, col, 0, -1) + run_length(board, row, col, 0, 1) - 1;
            out[3 * NUM_CELLS + i] = vertical as f32 / size;
            out[4 * NUM_CELLS + i] = horizontal as f32 / size;
        }
    }
}

pub fn cell_name(row: usize, col: usize) -> String {
    format!("{}{}", char::from(b'a' + col as u8), row + 1)
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Quelhas;

impl Ruleset for Quelhas {
    type State = State;

    fn name(&self) -> &'static str {
        "quelhas"
    }

    fn initial_state(&self) -> State {
        State::initial()
    }

    fn to_move(&self, state: &State) -> Player {
        state.turn
    }

    fn legal_actions(&self, state: &State) -> Vec<usize> {
        state.canonical().legal_indices()
    }

    fn apply(&self, state: &State, action: usize) -> State {
        let action = Action::from_index(action)
            .unwrap_or_else(|| panic!("{action} is not a Quelhas action"));
        state.apply(action)
    }

    fn winner(&self, state: &State) -> Option<Player> {
        state.winner()
    }

    fn input_shape(&self) -> InputShape {
        InputShape {
            planes: NUM_PLANES,
            height: BOARD_SIZE,
            width: BOARD_SIZE,
        }
    }

    fn policy_planes(&self) -> usize {
        NUM_LENGTHS
    }

    fn encode(&self, state: &State, out: &mut [f32]) {
        encode(state.canonical(), out);
    }

    fn num_symmetries(&self) -> usize {
        SYMMETRIES.len()
    }

    fn transform_state(&self, state: &State, symmetry: usize) -> State {
        let (flip_rows, flip_cols) = SYMMETRIES[symmetry];
        state.flip(flip_rows, flip_cols)
    }

    fn transform_action(&self, action: usize, symmetry: usize) -> usize {
        let (flip_rows, flip_cols) = SYMMETRIES[symmetry];
        Action::from_index(action).map_or(action, |a| a.flip(flip_rows, flip_cols).index())
    }

    fn describe_action(&self, state: &State, action: usize) -> String {
        Action::from_index(action).map_or_else(
            || format!("invalid action {action}"),
            |a| {
                let ((r0, c0), (r1, c1)) = a.segment(state.turn);
                format!("{}-{}", cell_name(r0, c0), cell_name(r1, c1))
            },
        )
    }

    fn typical_game_length(&self) -> usize {
        30
    }

    fn state_bytes(&self) -> usize {
        17
    }

    fn write_state(&self, state: &State, out: &mut Vec<u8>) {
        out.extend_from_slice(&state.empty.bits().to_le_bytes());
        out.push(u8::from(state.turn == Player::Right));
    }

    fn read_state(&self, bytes: &[u8]) -> Option<State> {
        let (board, turn) = bytes.split_first_chunk::<16>()?;
        let bits = u128::from_le_bytes(*board);
        if bits & !Board::FULL.0 != 0 {
            return None;
        }
        let turn = match turn {
            [0] => Player::Left,
            [1] => Player::Right,
            _ => return None,
        };
        Some(State {
            empty: Board(bits),
            turn,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCATTERED: &str = "..#.......
                             .#..#.....
                             ...#......
                             #.........
                             ....##....
                             .#........
                             ......#...
                             ..#.......
                             ........#.
                             ...#....##";

    #[test]
    fn action_roundtrip() {
        let all: Vec<Action> = Action::all().collect();
        assert_eq!(
            all.len(),
            (MIN_LENGTH..=BOARD_SIZE)
                .map(|l| (BOARD_SIZE - l + 1) * BOARD_SIZE)
                .sum::<usize>()
        );
        for a in all {
            assert_eq!(Action::new(a.length(), a.row(), a.col()), Some(a));
            for turn in [Player::Left, Player::Right] {
                let (start, end) = a.segment(turn);
                assert_eq!(a.cells(turn).next(), Some(start));
                assert_eq!(a.cells(turn).last(), Some(end));
                assert_eq!(Action::from_segment(turn, start, end), Some(a));
                assert_eq!(Action::from_segment(turn, end, start), Some(a));
            }
        }
    }

    #[test]
    fn initial_moves() {
        let state = State::initial();
        assert_eq!(state.legal_actions().len(), 450);
        assert_eq!(state.canonical().count_legal_actions(), 450);
        assert!(!state.is_terminal());
    }

    #[test]
    fn right_crosses_horizontally() {
        let state = State {
            empty: Board::FULL,
            turn: Player::Right,
        };
        let a = Action::from_segment(Player::Right, (3, 2), (3, 5)).unwrap();
        let next = state.apply(a);
        let crossed: Vec<(usize, usize)> = (0..BOARD_SIZE)
            .flat_map(|r| (0..BOARD_SIZE).map(move |c| (r, c)))
            .filter(|&(r, c)| !next.empty.is_empty(r, c))
            .collect();
        assert_eq!(crossed, vec![(3, 2), (3, 3), (3, 4), (3, 5)]);
        assert_eq!(next.turn, Player::Left);
    }

    #[test]
    fn legal_actions_match_count() {
        let board: Board = SCATTERED.parse().unwrap();
        for b in [board, board.transpose()] {
            let actions = b.legal_actions();
            assert_eq!(actions.len(), b.count_legal_actions());
            assert!(actions.windows(2).all(|w| w[0] < w[1]));
            for a in &actions {
                assert!(a.cells(Player::Left).all(|(r, c)| b.is_empty(r, c)));
            }
        }
    }

    #[test]
    fn symmetries_map_legal_actions() {
        let board: Board = SCATTERED.parse().unwrap();
        for turn in [Player::Left, Player::Right] {
            let state = State { empty: board, turn };
            for symmetry in 0..Quelhas.num_symmetries() {
                let flipped = Quelhas.transform_state(&state, symmetry);
                let mut mapped: Vec<usize> = Quelhas
                    .legal_actions(&state)
                    .into_iter()
                    .map(|a| Quelhas.transform_action(a, symmetry))
                    .collect();
                mapped.sort_unstable();
                assert_eq!(mapped, Quelhas.legal_actions(&flipped));
            }
        }
    }

    #[test]
    fn state_bytes_roundtrip() {
        let board: Board = SCATTERED.parse().unwrap();
        for turn in [Player::Left, Player::Right] {
            let state = State { empty: board, turn };
            let mut bytes = Vec::new();
            Quelhas.write_state(&state, &mut bytes);
            assert_eq!(bytes.len(), Quelhas.state_bytes());
            assert_eq!(Quelhas.read_state(&bytes), Some(state));
        }
        assert_eq!(Quelhas.read_state(&[0xff; 17]), None);
    }
}
