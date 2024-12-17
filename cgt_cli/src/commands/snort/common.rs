use anyhow::{bail, Context, Result};
use cgt::{
    genetic_algorithm::Scored,
    graph::{adjacency_matrix::undirected::UndirectedGraph, Graph},
    numeric::{dyadic_rational_number::DyadicRationalNumber, rational::Rational},
    short::partizan::{
        canonical_form::CanonicalForm,
        games::snort::{Snort, VertexKind},
        partizan_game::PartizanGame,
        transposition_table::ParallelTranspositionTable,
    },
};
use std::{
    io::{self, stderr, Write},
    process::{Command, Stdio},
    str::FromStr,
    time,
};

#[derive(serde::Serialize, serde::Deserialize)]
pub enum Log {
    Generation {
        generation: usize,
        top_score: Rational,
        temperature: DyadicRationalNumber,
    },
    HighFitness {
        position: Scored<Snort<VertexKind, UndirectedGraph<VertexKind>>, Rational>,
        canonical_form: String,
        temperature: DyadicRationalNumber,
        degree: usize,
    },
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct EvaluationResult {
    pub position: Snort<VertexKind, UndirectedGraph<VertexKind>>,
    pub canonical_form: CanonicalForm,
    pub temperature: DyadicRationalNumber,
    pub degree: usize,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub from: u32,
    pub to: u32,
}

impl Edge {
    fn parse(input: &str) -> nom::IResult<&str, Edge> {
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

fn dump_edges(w: &mut impl Write, graph: &UndirectedGraph<VertexKind>) -> io::Result<()> {
    let mut first = true;

    for v in graph.vertex_indices() {
        for u in graph.vertex_indices() {
            if v < u && graph.are_adjacent(v, u) {
                if !first {
                    write!(w, ",")?;
                }
                write!(w, "{}-{}", v.index, u.index)?;
                first = false;
            }
        }
    }

    writeln!(w)?;

    Ok(())
}

pub fn analyze_position(
    position: Snort<VertexKind, UndirectedGraph<VertexKind>>,
    with_graphviz: bool,
) -> Result<()> {
    let transposition_table = ParallelTranspositionTable::new();
    let canonical_form = position.canonical_form(&transposition_table);
    let temperature = canonical_form.temperature();

    let timestamp = time::SystemTime::now()
        .duration_since(time::SystemTime::UNIX_EPOCH)
        .context("Could not get system time")?
        .as_millis();

    if with_graphviz {
        let filename = format!("snort{}.png", timestamp);
        render_snort(&position, &filename, "png", "fdp")?;
        render_snort(&position, "snort.png", "png", "fdp")?;
        eprintln!("Graph: {}", filename);
    }

    for (idx, m) in position
        .sensible_left_moves(&transposition_table)
        .iter()
        .enumerate()
    {
        if with_graphviz {
            let filename = format!("snort{}-left{}.png", timestamp, idx);
            render_snort(m, &filename, "png", "fdp")?;
            eprintln!("Left Move {} Graph: {}", idx, filename);
        }

        dump_edges(&mut stderr(), &m.graph)?;
    }
    for (idx, m) in position
        .sensible_right_moves(&transposition_table)
        .iter()
        .enumerate()
    {
        if with_graphviz {
            let filename = format!("snort{}-right{}.png", timestamp, idx);
            render_snort(m, &filename, "png", "fdp")?;
            eprintln!("Right Move {} Graph: {}", idx, filename);
        }

        dump_edges(&mut stderr(), &m.graph)?;
    }

    let degree = position.degree();
    let second_degree = position.second_degree();

    eprintln!("Canonical Form: {}", canonical_form);
    eprintln!("Temperature: {}", temperature);
    eprintln!("Degree: {}", degree);
    eprintln!("Degree2: {}", second_degree);
    eprintln!(
        "Conjecture: {} <= {}",
        temperature,
        Rational::from(degree as i64) + (Rational::from(second_degree as i64) / Rational::from(2))
    );

    let result = EvaluationResult {
        position,
        canonical_form,
        temperature,
        degree,
    };

    println!("{}", serde_json::ser::to_string(&result).unwrap());

    Ok(())
}

fn render_snort(
    position: &Snort<VertexKind, UndirectedGraph<VertexKind>>,
    filename: &str,
    format: &str,
    engine: &str,
) -> Result<()> {
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
