//! Fjords on an 8 by 8 board of vertices in offset rows, each joined to its up to six neighbours
//! by an edge that may be missing. A player claims an empty vertex joined by an edge to one of
//! their own, and whoever cannot move on their turn loses.
//!
//! Left plays blue and Right plays red. A game starts from a board dealt at random: every edge is
//! drawn with some probability, and each side gets [`START_VERTICES`] vertices.

use crate::ruleset::{Input, Player, Ruleset};
use rand::{Rng, RngExt};
use std::sync::LazyLock;

pub const BOARD_SIZE: usize = 8;
pub const NUM_VERTICES: usize = BOARD_SIZE * BOARD_SIZE;
pub const START_VERTICES: usize = 3;

/// Probability of an edge on the boards the network trains on
pub const EDGE_PROBABILITY: f64 = 0.85;

// features of a vertex: [mine, opponent, empty, legal_for_mover, degree/6]
pub const NUM_FEATURES: usize = 5;

/// Index of the vertex in column `q` of row `r`
pub const fn vertex(q: usize, r: usize) -> usize {
    r * BOARD_SIZE + q
}

/// Column and row of `vertex`
pub const fn coordinates(vertex: usize) -> (usize, usize) {
    (vertex % BOARD_SIZE, vertex / BOARD_SIZE)
}

/// Odd rows are shifted half a vertex to the right, so the neighbours of a vertex depend on the
/// parity of its row.
fn potential_neighbours(v: usize) -> impl Iterator<Item = usize> {
    let (q, r) = coordinates(v);
    let directions: [(isize, isize); 6] = if r % 2 == 0 {
        [(1, 0), (-1, 0), (0, -1), (-1, -1), (0, 1), (-1, 1)]
    } else {
        [(1, 0), (-1, 0), (1, -1), (0, -1), (1, 1), (0, 1)]
    };
    let size = BOARD_SIZE as isize;
    directions.into_iter().filter_map(move |(dq, dr)| {
        let (nq, nr) = (q as isize + dq, r as isize + dr);
        ((0..size).contains(&nq) && (0..size).contains(&nr))
            .then(|| vertex(nq as usize, nr as usize))
    })
}

struct Graph {
    /// Every edge a board can have, as its two vertices in increasing order, sorted
    edges: Vec<(usize, usize)>,
    /// The neighbours of each vertex together with the edges to them
    neighbours: Vec<Vec<(usize, usize)>>,
    /// The edge each edge becomes when the board is turned around
    turned: Vec<usize>,
}

static GRAPH: LazyLock<Graph> = LazyLock::new(|| {
    let mut edges: Vec<(usize, usize)> = (0..NUM_VERTICES)
        .flat_map(|v| potential_neighbours(v).map(move |n| (v.min(n), v.max(n))))
        .collect();
    edges.sort_unstable();
    edges.dedup();
    let index = |a: usize, b: usize| {
        edges
            .binary_search(&(a.min(b), a.max(b)))
            .expect("neighbours are joined by a potential edge")
    };
    let neighbours = (0..NUM_VERTICES)
        .map(|v| potential_neighbours(v).map(|n| (n, index(v, n))).collect())
        .collect();
    let turned = edges
        .iter()
        .map(|&(a, b)| index(turn_around(a), turn_around(b)))
        .collect();
    Graph {
        edges,
        neighbours,
        turned,
    }
});

/// Number of edges a board can have
pub fn num_edges() -> usize {
    GRAPH.edges.len()
}

/// The two vertices of edge `edge`
pub fn edge(edge: usize) -> (usize, usize) {
    GRAPH.edges[edge]
}

/// The vertex `vertex` lands on when the board is turned half a circle, which keeps the shifts
/// of the rows, so the turned board is a board again
pub const fn turn_around(vertex: usize) -> usize {
    NUM_VERTICES - 1 - vertex
}

/// A random number generator that stays the same across platforms and versions of `rand`, so
/// that a seed always deals the same board, in the browser as well as when the site is built
struct SplitMix64(u64);

impl SplitMix64 {
    const fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A number in `[0, 1)`
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// A number in `0..n`
    fn below(&mut self, n: usize) -> usize {
        ((u128::from(self.next()) * n as u128) >> 64) as usize
    }
}

/// How a game ends once the empty vertices fall apart into regions that only one side can reach
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settled {
    pub winner: Player,
    /// How many more moves the winner has left than the loser
    pub margin: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct State {
    /// Vertices of Left (blue), as bits indexed by vertex
    pub left: u64,
    /// Vertices of Right (red), as bits indexed by vertex
    pub right: u64,
    /// Edges of the board, as bits indexed by edge
    pub edges: [u64; 3],
    pub turn: Player,
}

impl State {
    /// The board dealt by `seed`, with every edge drawn with probability `edge_probability`
    pub fn deal(seed: u64, edge_probability: f64) -> State {
        let mut rng = SplitMix64(seed);
        let mut edges = [0; 3];
        for e in 0..num_edges() {
            if rng.unit() < edge_probability {
                edges[e / 64] |= 1 << (e % 64);
            }
        }
        let mut vertices: Vec<usize> = (0..NUM_VERTICES).collect();
        for i in 0..2 * START_VERTICES {
            let j = i + rng.below(NUM_VERTICES - i);
            vertices.swap(i, j);
        }
        let bits = |vertices: &[usize]| vertices.iter().fold(0, |bits, &v| bits | 1 << v);
        State {
            left: bits(&vertices[..START_VERTICES]),
            right: bits(&vertices[START_VERTICES..2 * START_VERTICES]),
            edges,
            turn: Player::Left,
        }
    }

    pub const fn has_edge(&self, edge: usize) -> bool {
        self.edges[edge / 64] >> (edge % 64) & 1 == 1
    }

    pub const fn vertices(&self, side: Player) -> u64 {
        match side {
            Player::Left => self.left,
            Player::Right => self.right,
        }
    }

    pub const fn empty(&self) -> u64 {
        !(self.left | self.right)
    }

    /// The vertices joined to `vertex` by an edge
    pub fn neighbours(&self, vertex: usize) -> impl Iterator<Item = usize> + '_ {
        GRAPH.neighbours[vertex]
            .iter()
            .filter(|&&(_, e)| self.has_edge(e))
            .map(|&(n, _)| n)
    }

    pub fn degree(&self, vertex: usize) -> usize {
        self.neighbours(vertex).count()
    }

    /// The empty vertices `side` can claim, as bits indexed by vertex
    pub fn reachable(&self, side: Player) -> u64 {
        let own = self.vertices(side);
        let empty = self.empty();
        (0..NUM_VERTICES)
            .filter(|&v| own >> v & 1 == 1)
            .flat_map(|v| self.neighbours(v))
            .fold(0, |reachable, n| reachable | (empty & 1 << n))
    }

    pub fn legal_actions(&self) -> Vec<usize> {
        let reachable = self.reachable(self.turn);
        (0..NUM_VERTICES)
            .filter(|&v| reachable >> v & 1 == 1)
            .collect()
    }

    pub fn is_legal(&self, vertex: usize) -> bool {
        vertex < NUM_VERTICES && self.reachable(self.turn) >> vertex & 1 == 1
    }

    /// Whoever cannot move on their turn loses
    pub fn winner(&self) -> Option<Player> {
        (self.reachable(self.turn) == 0).then(|| self.turn.opposite())
    }

    #[must_use]
    pub fn apply(&self, vertex: usize) -> State {
        assert!(
            self.is_legal(vertex),
            "{vertex} is not a legal move of {:?}",
            self.turn
        );
        let mut next = *self;
        match self.turn {
            Player::Left => next.left |= 1 << vertex,
            Player::Right => next.right |= 1 << vertex,
        }
        next.turn = self.turn.opposite();
        next
    }

    /// The result of the game once no region of empty vertices can be reached by both sides.
    /// Each side then has exactly as many moves left as there are vertices in its regions, and
    /// with as many as the other side, the side to move runs out first.
    pub fn settled(&self) -> Option<Settled> {
        let empty = self.empty();
        if empty == 0 {
            return None;
        }
        let mut seen = 0u64;
        let mut territory = [0usize, 0];
        for start in (0..NUM_VERTICES).filter(|&v| empty >> v & 1 == 1) {
            if seen >> start & 1 == 1 {
                continue;
            }
            let (mut size, mut touches) = (0, [false, false]);
            let mut frontier = vec![start];
            seen |= 1 << start;
            while let Some(v) = frontier.pop() {
                size += 1;
                for n in self.neighbours(v) {
                    touches[0] |= self.left >> n & 1 == 1;
                    touches[1] |= self.right >> n & 1 == 1;
                    if empty >> n & 1 == 1 && seen >> n & 1 == 0 {
                        seen |= 1 << n;
                        frontier.push(n);
                    }
                }
            }
            if touches == [true, true] {
                return None;
            }
            for (side, touched) in touches.into_iter().enumerate() {
                if touched {
                    territory[side] += size;
                }
            }
        }
        let [left, right] = territory;
        let winner = match left.cmp(&right) {
            std::cmp::Ordering::Greater => Player::Left,
            std::cmp::Ordering::Less => Player::Right,
            std::cmp::Ordering::Equal => self.turn.opposite(),
        };
        Some(Settled {
            winner,
            margin: left.abs_diff(right),
        })
    }

    /// The board turned half a circle
    #[must_use]
    pub fn turned_around(&self) -> State {
        let turn_bits = |bits: u64| bits.reverse_bits();
        let mut edges = [0; 3];
        for e in (0..num_edges()).filter(|&e| self.has_edge(e)) {
            let t = GRAPH.turned[e];
            edges[t / 64] |= 1 << (t % 64);
        }
        State {
            left: turn_bits(self.left),
            right: turn_bits(self.right),
            edges,
            turn: self.turn,
        }
    }
}

/// Writes the features of each vertex as the player to move sees them, followed by the
/// adjacency matrix of the board with a loop at each vertex, normalized by the degrees
pub fn encode(state: &State, out: &mut [f32]) {
    assert_eq!(out.len(), NUM_VERTICES * (NUM_FEATURES + NUM_VERTICES));
    out.fill(0.0);
    let (features, adjacency) = out.split_at_mut(NUM_VERTICES * NUM_FEATURES);
    let (mine, theirs) = (
        state.vertices(state.turn),
        state.vertices(state.turn.opposite()),
    );
    let legal = state.reachable(state.turn);
    let degrees: Vec<usize> = (0..NUM_VERTICES).map(|v| state.degree(v)).collect();
    for v in 0..NUM_VERTICES {
        let row = &mut features[v * NUM_FEATURES..(v + 1) * NUM_FEATURES];
        row[0] = (mine >> v & 1) as f32;
        row[1] = (theirs >> v & 1) as f32;
        row[2] = (state.empty() >> v & 1) as f32;
        row[3] = (legal >> v & 1) as f32;
        row[4] = degrees[v] as f32 / 6.0;
    }
    let scale: Vec<f32> = degrees
        .iter()
        .map(|&d| 1.0 / ((d + 1) as f32).sqrt())
        .collect();
    for v in 0..NUM_VERTICES {
        adjacency[v * NUM_VERTICES + v] = scale[v] * scale[v];
        for n in state.neighbours(v) {
            adjacency[v * NUM_VERTICES + n] = scale[v] * scale[n];
        }
    }
}

/// Name of `vertex` on the board, its column as a letter and its row as a number
pub fn vertex_name(vertex: usize) -> String {
    let (q, r) = coordinates(vertex);
    format!("{}{}", char::from(b'a' + q as u8), r + 1)
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Fjords;

impl Ruleset for Fjords {
    type State = State;

    fn name(&self) -> &'static str {
        "fjords"
    }

    fn start(&self, rng: &mut impl Rng) -> State {
        State::deal(rng.random(), EDGE_PROBABILITY)
    }

    fn fixed_start(&self) -> Option<State> {
        None
    }

    fn to_move(&self, state: &State) -> Player {
        state.turn
    }

    fn legal_actions(&self, state: &State) -> Vec<usize> {
        state.legal_actions()
    }

    fn apply(&self, state: &State, action: usize) -> State {
        state.apply(action)
    }

    fn winner(&self, state: &State) -> Option<Player> {
        state.winner()
    }

    fn input(&self) -> Input {
        Input::Graph {
            nodes: NUM_VERTICES,
            features: NUM_FEATURES,
        }
    }

    fn encode(&self, state: &State, out: &mut [f32]) {
        encode(state, out);
    }

    fn num_symmetries(&self) -> usize {
        2
    }

    fn transform_state(&self, state: &State, symmetry: usize) -> State {
        if symmetry == 0 {
            *state
        } else {
            state.turned_around()
        }
    }

    fn transform_action(&self, action: usize, symmetry: usize) -> usize {
        if symmetry == 0 || action >= NUM_VERTICES {
            action
        } else {
            turn_around(action)
        }
    }

    fn describe_action(&self, _: &State, action: usize) -> String {
        if action < NUM_VERTICES {
            vertex_name(action)
        } else {
            format!("invalid action {action}")
        }
    }

    fn typical_game_length(&self) -> usize {
        50
    }

    fn state_bytes(&self) -> usize {
        41
    }

    fn write_state(&self, state: &State, out: &mut Vec<u8>) {
        out.extend_from_slice(&state.left.to_le_bytes());
        out.extend_from_slice(&state.right.to_le_bytes());
        for word in state.edges {
            out.extend_from_slice(&word.to_le_bytes());
        }
        out.push(u8::from(state.turn == Player::Right));
    }

    fn read_state(&self, bytes: &[u8]) -> Option<State> {
        let mut words = bytes
            .get(..40)?
            .chunks_exact(8)
            .map(|word| u64::from_le_bytes(word.try_into().expect("chunks of 8 bytes")));
        let (left, right) = (words.next()?, words.next()?);
        let edges = [words.next()?, words.next()?, words.next()?];
        let extra = num_edges()..3 * 64;
        let turn = match bytes.get(40..)? {
            [0] => Player::Left,
            [1] => Player::Right,
            _ => return None,
        };
        let state = State {
            left,
            right,
            edges,
            turn,
        };
        (left & right == 0 && extra.into_iter().all(|e| !state.has_edge(e))).then_some(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_has_every_neighbour_once() {
        // 7 edges in each row, and 15 between each two neighbouring rows
        assert_eq!(num_edges(), 7 * 8 + 15 * 7);
        for v in 0..NUM_VERTICES {
            for n in potential_neighbours(v) {
                assert!(potential_neighbours(n).any(|back| back == v), "{v} {n}");
            }
        }
    }

    #[test]
    fn deals_are_reproducible() {
        let state = State::deal(1337, EDGE_PROBABILITY);
        assert_eq!(state, State::deal(1337, EDGE_PROBABILITY));
        assert_ne!(state, State::deal(1338, EDGE_PROBABILITY));
        assert_eq!(state.left.count_ones(), 3);
        assert_eq!(state.right.count_ones(), 3);
        assert_eq!(state.left & state.right, 0);
        assert_eq!(state.turn, Player::Left);
        assert_eq!(
            State::deal(1, 1.0)
                .edges
                .map(u64::count_ones)
                .iter()
                .sum::<u32>(),
            161
        );
    }

    #[test]
    fn moves_claim_neighbours() {
        let mut state = State::deal(0, 1.0);
        state.left = 1 << vertex(0, 0);
        state.right = 1 << vertex(7, 7);
        // (0, 0) is in an unshifted row, so its neighbours are (1, 0) and (0, 1)
        assert_eq!(state.legal_actions(), vec![vertex(1, 0), vertex(0, 1)]);
        let next = state.apply(vertex(0, 1));
        assert_eq!(next.turn, Player::Right);
        // (7, 7) is in a shifted row, so its neighbours are (6, 7), (7, 6) and nothing to the right
        assert_eq!(next.legal_actions(), vec![vertex(7, 6), vertex(6, 7)]);
    }

    #[test]
    fn whoever_cannot_move_loses() {
        let mut state = State::deal(0, 0.0);
        assert_eq!(state.winner(), Some(Player::Right));
        state.edges = State::deal(0, 1.0).edges;
        assert_eq!(state.winner(), None);
    }

    #[test]
    fn settled_games_count_territories() {
        // Left owns a1 and Right owns h8, and both reach the one empty region
        let mut state = State::deal(0, 1.0);
        state.left = 1 << vertex(0, 0);
        state.right = 1 << vertex(7, 7);
        assert_eq!(state.settled(), None);

        // Without the edges out of the corners each side keeps only its corner region
        state.edges = State::deal(0, 0.0).edges;
        let mut join = |a: usize, b: usize| {
            let e = GRAPH.neighbours[a]
                .iter()
                .find(|&&(n, _)| n == b)
                .unwrap()
                .1;
            state.edges[e / 64] |= 1 << (e % 64);
        };
        join(vertex(0, 0), vertex(1, 0));
        join(vertex(1, 0), vertex(2, 0));
        join(vertex(7, 7), vertex(6, 7));
        assert_eq!(
            state.settled(),
            Some(Settled {
                winner: Player::Left,
                margin: 1
            })
        );
        state.left |= 1 << vertex(1, 0);
        state.turn = Player::Right;
        // One move each, and Right, to move, runs out first
        assert_eq!(
            state.settled(),
            Some(Settled {
                winner: Player::Left,
                margin: 0
            })
        );
    }

    #[test]
    fn turning_around_maps_legal_moves() {
        for seed in 0..20 {
            let state = State::deal(seed, EDGE_PROBABILITY);
            let turned = Fjords.transform_state(&state, 1);
            let mut mapped: Vec<usize> = state
                .legal_actions()
                .into_iter()
                .map(|a| Fjords.transform_action(a, 1))
                .collect();
            mapped.sort_unstable();
            assert_eq!(mapped, turned.legal_actions());
            assert_eq!(Fjords.transform_state(&turned, 1), state);
        }
    }

    #[test]
    fn encoding_normalizes_adjacency() {
        let state = State::deal(5, EDGE_PROBABILITY);
        let mut out = vec![0.0; Fjords.input().encoding_len()];
        Fjords.encode(&state, &mut out);
        let adjacency = &out[NUM_VERTICES * NUM_FEATURES..];
        for v in 0..NUM_VERTICES {
            let d = state.degree(v) as f32 + 1.0;
            assert!((adjacency[v * NUM_VERTICES + v] - 1.0 / d).abs() < 1e-6);
            for n in 0..NUM_VERTICES {
                assert_eq!(
                    adjacency[v * NUM_VERTICES + n].to_bits(),
                    adjacency[n * NUM_VERTICES + v].to_bits()
                );
            }
        }
        let legal: f32 = (0..NUM_VERTICES).map(|v| out[v * NUM_FEATURES + 3]).sum();
        assert_eq!(legal as usize, state.legal_actions().len());
    }

    #[test]
    fn state_bytes_roundtrip() {
        let state = State::deal(9, EDGE_PROBABILITY)
            .apply(State::deal(9, EDGE_PROBABILITY).legal_actions()[0]);
        let mut bytes = Vec::new();
        Fjords.write_state(&state, &mut bytes);
        assert_eq!(bytes.len(), Fjords.state_bytes());
        assert_eq!(Fjords.read_state(&bytes), Some(state));
        bytes[0] = 0xff;
        bytes[8] = 0xff;
        assert_eq!(Fjords.read_state(&bytes), None);
    }
}
