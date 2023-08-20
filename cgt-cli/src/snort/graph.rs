use crate::snort::common::{analyze_position, Edge};
use anyhow::Result;
use cgt::{
    graph::undirected::Graph,
    short::partizan::games::snort::{Snort, VertexColor, VertexKind},
};
use clap::Parser;

#[derive(Parser, Debug, Clone)]
pub struct Args {
    #[arg(long, value_delimiter = ',')]
    edges: Vec<Edge>,

    #[arg(long, value_delimiter = ',')]
    tinted_left: Vec<u32>,

    #[arg(long, value_delimiter = ',')]
    tinted_right: Vec<u32>,
}

pub fn run(args: Args) -> Result<()> {
    let graph_size = args
        .edges
        .iter()
        .map(|edge| edge.from.max(edge.to))
        .max()
        .unwrap();
    let edges = args
        .edges
        .iter()
        .map(|edge| (edge.from as usize, edge.to as usize))
        .collect::<Vec<_>>();
    let graph = Graph::from_edges((graph_size + 1) as usize, &edges);

    let mut vertices = vec![VertexKind::Single(VertexColor::Empty); graph.size()];
    for v in args.tinted_left {
        vertices[v as usize] = VertexKind::Single(VertexColor::TintLeft);
    }
    for v in args.tinted_right {
        vertices[v as usize] = VertexKind::Single(VertexColor::TintRight);
    }

    let position = Snort::with_colors(vertices, graph).unwrap();
    analyze_position(position)?;

    Ok(())
}
