use anyhow::{bail, Context, Result};
use cgt::{
    graph::undirected::Graph,
    numeric::rational::Rational,
    short::partizan::{
        games::snort::{Snort, VertexColor},
        partizan_game::PartizanGame,
        transposition_table::TranspositionTable,
    },
};
use clap::Parser;
use std::{
    io::{self, stderr, Write},
    process::{Command, Stdio},
    str::FromStr,
    time,
};

use crate::snort::common::{Log, Scored};

#[derive(Debug, Clone)]
pub struct Edge {
    from: u32,
    to: u32,
}

impl Edge {
    fn parse<'s>(input: &'s str) -> nom::IResult<&'s str, Edge> {
        let (input, from) = nom::character::complete::u32(input)?;
        let (input, _) = nom::character::complete::char('-')(input)?;
        let (input, to) = nom::character::complete::u32(input)?;

        Ok((input, Edge { from, to }))
    }
}

impl FromStr for Edge {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Edge::parse(s)
            .map(|(_, edge)| edge)
            .map_err(|e| e.to_string())
    }
}

fn dump_edges(w: &mut impl Write, graph: &Graph) -> io::Result<()> {
    let mut first = true;

    for v in graph.vertices() {
        for u in graph.vertices() {
            if v < u && graph.are_adjacent(v, u) {
                if !first {
                    write!(w, ",")?;
                }
                write!(w, "{}-{}", v, u)?;
                first = false;
            }
        }
    }

    write!(w, "\n")?;

    Ok(())
}

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
    let degree = graph.degree();

    let mut vertices = vec![VertexColor::Empty; graph.size()];
    for v in args.tinted_left {
        vertices[v as usize] = VertexColor::TintLeft;
    }
    for v in args.tinted_right {
        vertices[v as usize] = VertexColor::TintRight;
    }

    let position = Snort::with_colors(vertices, graph).unwrap();

    let tt = TranspositionTable::new();
    let canonical_form = position.canonical_form(&tt);
    let temperature = canonical_form.temperature();

    let timestamp = time::SystemTime::now()
        .duration_since(time::SystemTime::UNIX_EPOCH)
        .context("Could not get system time")?
        .as_millis();
    let filename = format!("snort{}.png", timestamp);
    render_snort(&position, &filename, "png", "fdp")?;
    render_snort(&position, "snort.png", "png", "fdp")?;
    eprintln!("Graph: {}", filename);

    for (idx, m) in position.sensible_left_moves(&tt).iter().enumerate() {
        let filename = format!("snort{}-left{}.png", timestamp, idx);
        render_snort(&m, &filename, "png", "fdp")?;
        eprintln!("Left Move {} Graph: {}", idx, filename);
        dump_edges(&mut stderr(), &m.graph)?;
    }
    for (idx, m) in position.sensible_right_moves(&tt).iter().enumerate() {
        let filename = format!("snort{}-right{}.png", timestamp, idx);
        render_snort(&m, &filename, "png", "fdp")?;
        eprintln!("Right Move {} Graph: {}", idx, filename);
        dump_edges(&mut stderr(), &m.graph)?;
    }

    let score = temperature - Rational::from(degree as i32);

    let log = Log::HighFitness {
        position: Scored { position, score },
        canonical_form: canonical_form.to_string(),
        temperature,
        degree,
    };
    eprintln!("{}", serde_json::ser::to_string(&log).unwrap());

    Ok(())
}

fn render_snort(position: &Snort, filename: &str, format: &str, engine: &str) -> Result<()> {
    let mut graphviz_proc = Command::new(engine)
        .stdin(Stdio::piped())
        .arg(format!("-T{}", format))
        .arg(format!("-o{}", filename))
        .spawn()
        .context("Could not spawn graphviz")?;

    // Pipe dot to the running engine via stdin
    graphviz_proc
        .stdin
        .take()
        .context("Could not open graphviz stdin")?
        .write_all(position.to_graphviz().as_bytes())
        .context("Could not write to graphviz stdin")?;

    // Await result and check for errors
    if !graphviz_proc
        .wait()
        .context("Could not wait for graphviz")?
        .success()
    {
        bail!("Graphviz failed");
    };

    Ok(())
}