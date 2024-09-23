use crate::commands::snort::common::{analyze_position, Edge};
use anyhow::Result;
use cgt::{
    graph::{undirected::UndirectedGraph, Graph},
    short::partizan::games::snort::{Snort, VertexColor, VertexKind},
};
use clap::Parser;

#[derive(Parser, Debug, Clone)]
/// Evaluate a graph of Snort position
pub struct Args {
    #[arg(long, value_delimiter = ',')]
    /// Comma-separated list of edges in the graph in the form `<from>-<to>` (e.g. '0-1,1-2').
    ///
    /// Size of the graph is determined by the maximum vertex index in the list of edges.
    edges: Vec<Edge>,

    #[arg(long, value_delimiter = ',')]
    /// Comma-separated list of vertices that are tinted blue/left.
    tinted_left: Vec<u32>,

    #[arg(long, value_delimiter = ',')]
    /// Comma-separated list of vertices that are tinted red/right.
    tinted_right: Vec<u32>,

    #[arg(long)]
    /// Do not generate a graphviz graph of the position and immediate children.
    no_graphviz: bool,
}

pub fn run(args: Args) -> Result<()> {
    let graph_size = args
        .edges
        .iter()
        .map(|edge| edge.from.max(edge.to))
        .max()
        .unwrap_or(0);
    let edges = args
        .edges
        .iter()
        .map(|edge| (edge.from as usize, edge.to as usize))
        .collect::<Vec<_>>();
    let graph = UndirectedGraph::from_edges((graph_size + 1) as usize, &edges);

    let mut vertices = vec![VertexKind::Single(VertexColor::Empty); graph.size()];
    for v in args.tinted_left {
        vertices[v as usize] = VertexKind::Single(VertexColor::TintLeft);
    }
    for v in args.tinted_right {
        vertices[v as usize] = VertexKind::Single(VertexColor::TintRight);
    }

    let position = Snort::with_colors(vertices, graph).unwrap();
    analyze_position(position, !args.no_graphviz)?;

    Ok(())
}
