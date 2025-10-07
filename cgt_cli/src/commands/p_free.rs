use crate::io::FilePathOr;
use anyhow::Context;
use cgt::{
    misere::p_free::{GameForm, Outcome},
    total::TotalWrapper,
};
use clap::Parser;
use itertools::Itertools;
use std::{
    cmp::Ordering,
    collections::HashMap,
    io::{self, BufWriter, Stdout},
};

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long)]
    dot_output: FilePathOr<Stdout>,
}

pub fn run(args: Args) -> anyhow::Result<()> {
    let next_day = |previous_day| {
        GameForm::next_day(previous_day)
            .filter(|g: &GameForm| g.is_p_free() && g.is_dead_ending())
            .collect::<Vec<_>>()
    };

    let day0 = vec![GameForm::new_integer(0)];
    let day1 = next_day(&day0);
    let day2 = next_day(&day1);

    let mut f = BufWriter::new(
        args.dot_output
            .create()
            .with_context(|| format!("Failed to create dot file `{}`", args.dot_output))?,
    );
    to_graphviz(&day2, &mut f)
        .with_context(|| format!("Failed to write to dot file `{}`", args.dot_output))?;

    Ok(())
}

fn to_graphviz<W>(day: &[GameForm], mut f: W) -> io::Result<()>
where
    W: io::Write,
{
    writeln!(f, "graph {{")?;
    writeln!(f, "splines=false;")?;
    writeln!(f, "edge [penwidth=0.15]")?;

    let mut indices = HashMap::with_capacity(day.len());
    for (i, g) in day.iter().enumerate() {
        writeln!(f, "{i} [label = \"{g}\"];")?;
        indices.insert(TotalWrapper::new(g), i);
    }

    for (g, h) in day.iter().tuple_combinations() {
        if game_lt(g, h) && !day.iter().any(|k| game_lt(g, k) && game_lt(k, h)) {
            let i = *indices.get(TotalWrapper::from_ref(&g)).unwrap();
            let j = *indices.get(TotalWrapper::from_ref(&h)).unwrap();
            writeln!(f, "{j} -- {i}")?;
        }
    }
    writeln!(f, "}}")?;

    Ok(())
}

fn game_lt(g: &GameForm, h: &GameForm) -> bool {
    let sum = GameForm::sum(&g, &h.conjugate());
    matches!(
        Outcome::N.partial_cmp(&sum.outcome()).unwrap(),
        Ordering::Less
    )
}
