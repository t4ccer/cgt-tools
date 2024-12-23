//! Digraph Placement Game
//!
//! http://arxiv.org/abs/2407.12219

use std::{hash::Hash, marker::PhantomData};

use crate::{
    graph::{Graph, VertexIndex},
    has::Has,
    short::partizan::partizan_game::PartizanGame,
};

/// Vertex color of Digraph Placement Game
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum VertexColor {
    /// Blue vertex where only Left can move
    Left,

    /// Right vertex where only Right can move
    Right,
}

impl VertexColor {
    const fn try_from_u8(value: u8) -> Self {
        match value {
            value if value == Self::Left as u8 => Self::Left,
            value if value == Self::Right as u8 => Self::Right,
            _invalid => panic!("Invalid value"),
        }
    }
}

/// Digraph Placement Game
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DigraphPlacement<V, G> {
    /// Underlying game graph
    pub graph: G,
    _ty: PhantomData<V>,
}

impl<V, G> DigraphPlacement<V, G>
where
    V: Has<VertexColor> + Clone,
    G: Graph<V> + Clone,
{
    /// Create new Digraph Placement position from underyling graph
    pub fn new(graph: G) -> Self {
        Self {
            graph,
            _ty: PhantomData,
        }
    }

    /// Return position after player move in a given vertex. Note that it does not check
    /// if the move is legal
    pub fn move_in_vertex(&self, move_vertex_idx: VertexIndex) -> Self {
        let mut position = self.clone();
        let mut to_remove = vec![move_vertex_idx];
        to_remove.extend(self.graph.adjacent_to(move_vertex_idx));
        position.graph.remove_vertices(&mut to_remove);
        position
    }

    fn moves_for<const COLOR: u8>(&self) -> Vec<Self> {
        let own_color: VertexColor = const { VertexColor::try_from_u8(COLOR) };
        let mut moves = Vec::new();
        for v in self.graph.vertex_indices() {
            if *self.graph.get_vertex(v).get_inner() == own_color {
                moves.push(self.move_in_vertex(v));
            }
        }
        moves
    }
}

impl<G> PartizanGame for DigraphPlacement<VertexColor, G>
where
    G: Graph<VertexColor> + Clone + Hash + Eq + Send + Sync,
{
    fn left_moves(&self) -> Vec<Self> {
        self.moves_for::<{ VertexColor::Left as u8 }>()
    }

    fn right_moves(&self) -> Vec<Self> {
        self.moves_for::<{ VertexColor::Right as u8 }>()
    }
}

#[test]
fn two_down_star() {
    use crate::{
        graph::adjacency_matrix::directed::DirectedGraph,
        short::partizan::transposition_table::ParallelTranspositionTable,
    };

    let game = DigraphPlacement::new(DirectedGraph::from_edges(
        &[
            (VertexIndex { index: 1 }, VertexIndex { index: 0 }),
            (VertexIndex { index: 2 }, VertexIndex { index: 0 }),
            (VertexIndex { index: 3 }, VertexIndex { index: 0 }),
            (VertexIndex { index: 4 }, VertexIndex { index: 0 }),
            //
            (VertexIndex { index: 2 }, VertexIndex { index: 1 }),
            (VertexIndex { index: 3 }, VertexIndex { index: 1 }),
            (VertexIndex { index: 4 }, VertexIndex { index: 1 }),
            //
            (VertexIndex { index: 1 }, VertexIndex { index: 4 }),
            (VertexIndex { index: 2 }, VertexIndex { index: 4 }),
            (VertexIndex { index: 3 }, VertexIndex { index: 4 }),
        ],
        &[
            VertexColor::Right,
            VertexColor::Right,
            VertexColor::Left,
            VertexColor::Left,
            VertexColor::Left,
        ],
    ));

    let tt = ParallelTranspositionTable::new();
    assert_eq!(game.canonical_form(&tt).to_string(), "2v*");
}
