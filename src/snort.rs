//! Game of Snort

use num_derive::FromPrimitive;

use crate::{
    graph::undirected::Graph,
    short_canonical_game::{GameId, Moves, PartizanShortGame},
    transposition_table::TranspositionTable,
};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, FromPrimitive)]
#[repr(u8)]
pub enum VertexColor {
    Empty = 0,
    TintLeft = 1,
    TintRight = 2,
    Left = 3,
    Right = 4,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Position {
    vertices: Vec<VertexColor>,
    graph: Graph,
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
            position.vertices[move_vertex] = if own_tint_color == VertexColor::TintLeft {
                VertexColor::Left
            } else {
                debug_assert_eq!(own_tint_color, VertexColor::TintRight);
                VertexColor::Right
            };

            // Disconnect `move_vertex` from adjecent vertices and tint them
            for adjacent_vertex in self.graph.adjacent_to(move_vertex) {
                // Disconnect move vertex from adjecent
                position.graph.connect(move_vertex, adjacent_vertex, false);

                // No loops
                if adjacent_vertex != move_vertex {
                    // Tint adjacent vertex
                    position.vertices[adjacent_vertex] = own_tint_color;
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
    /// let cache = TranspositionTable::new();
    /// let game = position.canonical_form(&cache);
    /// assert_eq!(&cache.game_backend().print_game_to_str(game), "{2|0}");
    /// ```
    pub fn canonical_form(&self, cache: &TranspositionTable<Self>) -> GameId {
        if let Some(id) = cache.grids.get(&self) {
            return id;
        }

        let left_moves = self.left_moves();
        let right_moves = self.right_moves();

        // NOTE: That's redundant, but may increase performance
        // `construct_from_moves` on empty moves results in 0 as well
        if left_moves.is_empty() && right_moves.is_empty() {
            return cache.game_backend.zero_id;
        }

        let moves = Moves {
            left: left_moves.iter().map(|o| o.canonical_form(cache)).collect(),
            right: right_moves
                .iter()
                .map(|o| o.canonical_form(cache))
                .collect(),
        };

        let canonical_form = cache.game_backend.construct_from_moves(moves);
        cache.grids.insert(self.clone(), canonical_form);
        canonical_form
    }
}

#[test]
fn no_moves() {
    let position = Position::new(Graph::empty(0));
    assert_eq!(position.left_moves(), vec![]);
    assert_eq!(position.right_moves(), vec![]);
}
