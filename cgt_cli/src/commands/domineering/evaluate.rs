use crate::io::FileOrStdout;
use anyhow::{Context, Result};
use cgt::{
    drawing::svg::Svg,
    short::partizan::{
        games::domineering::Domineering, partizan_game::PartizanGame,
        transposition_table::ParallelTranspositionTable,
    },
};
use clap::Parser;
use std::{
    io::{BufWriter, Write},
    str::FromStr,
};

/// Evaluate single domineering position
#[derive(Parser, Debug)]
pub struct Args {
    /// Domineering position to evaluate (e.g. '..#|##.|.#.')
    #[arg(long)]
    position: String,

    /// SVG render output path
    #[arg(long, default_value = None)]
    output_svg: Option<FileOrStdout>,
}

pub fn run(args: Args) -> Result<()> {
    let position: Domineering =
        Domineering::from_str(&args.position).expect("Could not parse position");

    if let Some(ref svg_fp) = args.output_svg {
        let mut w = BufWriter::new(
            svg_fp
                .open()
                .context(format!("Could not create file '{}'", svg_fp))?,
        );
        let mut buf = String::new();
        position.to_svg(&mut buf).expect("Could not render SVG");
        buf.push('\n');
        w.write_all(buf.as_bytes())
            .context(format!("Could not write to file '{}'", svg_fp))?;
    }

    let tt = ParallelTranspositionTable::new();
    let canonical_form = position.canonical_form(&tt);
    println!("Canonical Form: {}", canonical_form);
    println!("Temperature: {}", canonical_form.temperature());

    Ok(())
}
