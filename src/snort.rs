//! Game of Snort
use num_derive::FromPrimitive;
use std::fmt::Write;

use crate::{
    graph::undirected::Graph,
    short_canonical_game::{Game, Moves, PartizanShortGame, PlacementGame},
    transposition_table::TranspositionTable,
};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, FromPrimitive)]
#[cfg_attr(
    feature = "serde",
    derive(serde_repr::Serialize_repr, serde_repr::Deserialize_repr)
)]
#[repr(u8)]
pub enum VertexColor {
    Empty = 0,
    TintLeft = 1,
    TintRight = 2,
    Taken = 3,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Position {
    pub vertices: Vec<VertexColor>,
    pub graph: Graph,
}

impl Position {
    /// Create new Snort position with all vertices empty.
    pub fn new(graph: Graph) -> Self {
        Self {
            vertices: vec![VertexColor::Empty; graph.size()],
            graph,
        }
    }

    // TODO: Perform that check
    /// Create a Snort position with initial colors. It's up to the user to ensure that no conflicting
    /// colors are connected in the graph.
    /// Returns `None` if `vertices` and `graph` have conflicting sizes.
    pub fn with_colors(vertices: Vec<VertexColor>, graph: Graph) -> Option<Self> {
        if vertices.len() != graph.size() {
            return None;
        }

        Some(Self { vertices, graph })
    }

    pub fn vertices(&self) -> &Vec<VertexColor> {
        &self.vertices
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    /// Get moves for a given player. Works only for `TintLeft` and `TintRight`.
    /// Any other input is undefined.
    fn moves_for<const COLOR: u8>(&self) -> Vec<Self> {
        // const ADT generics are unsable, so here we go
        let own_tint_color: VertexColor = num::FromPrimitive::from_u8(COLOR).unwrap();

        let mut moves = Vec::with_capacity(self.graph.size());

        // Vertices where player can move
        let move_vertices = self
            .vertices
            .iter()
            .enumerate()
            .filter(|(_, vertex_color)| {
                **vertex_color == own_tint_color || **vertex_color == VertexColor::Empty
            })
            .map(|(idx, _)| idx);

        // Go through list of vertices with legal move
        for move_vertex in move_vertices {
            let mut position: Position = self.clone();

            // Take vertex
            position.vertices[move_vertex] = VertexColor::Taken;

            // Disconnect `move_vertex` from adjecent vertices and tint them
            for adjacent_vertex in self.graph.adjacent_to(move_vertex) {
                // Disconnect move vertex from adjecent
                position.graph.connect(move_vertex, adjacent_vertex, false);

                // No loops
                if adjacent_vertex != move_vertex {
                    // Tint adjacent vertex
                    if position.vertices[adjacent_vertex] == own_tint_color
                        && position.vertices[adjacent_vertex] == VertexColor::Empty
                    {
                        position.vertices[adjacent_vertex] = own_tint_color;
                    } else {
                        position.vertices[adjacent_vertex] = VertexColor::Taken;
                    }
                }
            }
            moves.push(position);
        }
        moves
    }
}

impl PartizanShortGame for Position {
    fn left_moves(&self) -> Vec<Self> {
        self.moves_for::<{ VertexColor::TintLeft as u8 }>()
    }

    fn right_moves(&self) -> Vec<Self> {
        self.moves_for::<{ VertexColor::TintRight as u8 }>()
    }
}

impl PlacementGame for Position {
    fn free_places(&self) -> usize {
        self.vertices
            .iter()
            .filter(|v| {
                [
                    VertexColor::Empty,
                    VertexColor::TintLeft,
                    VertexColor::TintRight,
                ]
                .contains(v)
            })
            .count()
    }
}

impl Position {
    /// Get the canonical form of the position.
    ///
    /// # Arguments
    ///
    /// `cache` - Shared cache of short combinatorial games.
    ///
    /// # Examples
    ///
    /// ```
    /// use cgt::graph::undirected::Graph;
    /// use cgt::short_canonical_game::PartizanShortGame;
    /// use cgt::snort::{Position, VertexColor};
    /// use cgt::transposition_table::TranspositionTable;
    ///
    /// let mut graph = Graph::empty(3);
    /// graph.connect(1, 2, true);
    ///
    /// let colors = vec![
    ///     VertexColor::TintLeft,
    ///     VertexColor::TintRight,
    ///     VertexColor::TintLeft,
    /// ];
    ///
    /// let position = Position::with_colors(colors, graph).unwrap();
    /// assert_eq!(position.left_moves().len(), 2);
    /// assert_eq!(position.right_moves().len(), 1);
    ///
    /// let cache = TranspositionTable::new(1 << 22);
    /// let game = position.canonical_form(&cache);
    /// assert_eq!(&cache.game_backend().print_game_to_str(&game), "{2|0}");
    /// ```
    pub fn canonical_form(&self, cache: &TranspositionTable<Self>) -> Game {
        if let Some(id) = cache.grids_get(self) {
            return id;
        }

        let left_moves = self.left_moves();
        let right_moves = self.right_moves();

        // NOTE: That's redundant, but may increase performance
        // `construct_from_moves` on empty moves results in 0 as well
        if left_moves.is_empty() && right_moves.is_empty() {
            return cache.game_backend().construct_integer(0);
        }

        let moves = Moves {
            left: left_moves.iter().map(|o| o.canonical_form(cache)).collect(),
            right: right_moves
                .iter()
                .map(|o| o.canonical_form(cache))
                .collect(),
        };

        let canonical_form = cache.game_backend().construct_from_moves(moves);
        cache.grids_insert(self.clone(), canonical_form.clone());
        canonical_form
    }
}

#[test]
fn no_moves() {
    let position = Position::new(Graph::empty(0));
    assert_eq!(position.left_moves(), vec![]);
    assert_eq!(position.right_moves(), vec![]);
}

impl Position {
    pub fn to_graphviz(&self) -> String {
        let mut buf = String::new();

        write!(buf, "graph G {{").unwrap();

        for (v, color) in self.vertices.iter().enumerate() {
            let col = match color {
                VertexColor::Empty => "white",
                VertexColor::TintLeft => "blue",
                VertexColor::TintRight => "red",
                VertexColor::Taken => continue,
            };
            write!(buf, "{v} [fillcolor={col}, style=filled];").unwrap();
        }

        for v in self.graph.vertices() {
            for u in self.graph.vertices() {
                if v < u && self.graph.are_adjacent(v, u) {
                    write!(buf, "{v} -- {u};").unwrap();
                }
            }
        }

        write!(buf, "}}").unwrap();
        buf
    }
}
