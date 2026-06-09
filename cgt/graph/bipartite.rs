//! Specialized representation for small bipartite graphs

use crate::{
    graph::{Graph, VertexIndex},
    parsing::{Parser, impl_from_str_via_parser},
};
use std::fmt::Display;

/// Succinct representation of a bipartite graph
#[derive(Debug, Clone, Copy)]
pub struct BipartiteGraph {
    pub(crate) blue: u32,
    pub(crate) red: u32,
    pub(crate) mask: u64,
}

impl Display for BipartiteGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}b{}r{}", self.blue, self.red, self.mask)
    }
}

impl BipartiteGraph {
    fn parse(p: Parser<'_>) -> Option<(Parser<'_>, BipartiteGraph)> {
        let (p, blue) = p.parse_u32()?;
        let p = p.parse_ascii_char('b')?;
        let (p, red) = p.parse_u32()?;
        let p = p.parse_ascii_char('r')?;
        let (p, mask) = p.parse_u64()?;
        Some((p, BipartiteGraph { blue, red, mask }))
    }

    /// Convert to anything that implements [`Graph`]
    pub fn to_graph<G, V>(&self, blue: V, red: V) -> G
    where
        G: Graph<V>,
        V: Clone,
    {
        let mut vertices = Vec::with_capacity((self.blue + self.red) as usize);
        vertices.resize(self.blue as usize, blue);
        vertices.resize((self.blue + self.red) as usize, red);
        let mut graph = G::empty(&vertices);

        let mut edge_index = 0;
        for u in 0..self.blue {
            for v in self.blue..(self.blue + self.red) {
                if (self.mask & (1u64 << edge_index)) != 0 {
                    graph.connect(
                        VertexIndex { index: u as usize },
                        VertexIndex { index: v as usize },
                        true,
                    );
                }
                edge_index += 1;
            }
        }

        graph
    }
}

impl_from_str_via_parser!(BipartiteGraph);
