use crate::io::FileOrStdout;
use anyhow::{Context, Result};
use cgt::{
    drawing::{svg, tiny_skia, Draw},
    short::partizan::{
        partizan_game::PartizanGame, transposition_table::ParallelTranspositionTable,
    },
};
use clap::Parser;
use std::{
    fmt::Debug,
    io::{BufWriter, Write},
    str::FromStr,
};

/// Evaluate single position
#[derive(Parser, Debug)]
pub struct Args {
    /// Position to evaluate
    #[arg(long)]
    position: String,

    /// SVG render output path
    #[arg(long, default_value = None)]
    output_svg: Option<FileOrStdout>,

    /// PNG render output path
    #[arg(long, default_value = None)]
    output_png: Option<FileOrStdout>,
}

pub fn run<Game>(args: Args) -> Result<()>
where
    Game: FromStr + Draw + PartizanGame,
    <Game as FromStr>::Err: Debug,
{
    let position: Game = Game::from_str(&args.position).expect("Could not parse position");

    if let Some(svg_fp) = &args.output_svg {
        let mut w = BufWriter::new(
            svg_fp
                .create()
                .context(format!("Could not create file '{}'", svg_fp))?,
        );

        let canvas_size = position.required_canvas::<svg::Canvas>();
        let mut canvas = svg::Canvas::new(canvas_size);
        position.draw(&mut canvas);
        let svg = canvas.to_svg();
        w.write_all(svg.as_bytes())
            .context(format!("Could not write to file '{}'", svg_fp))?;
    }

    if let Some(png_fp) = &args.output_png {
        let mut w = BufWriter::new(
            png_fp
                .create()
                .context(format!("Could not create file '{}'", png_fp))?,
        );
        let canvas_size = position.required_canvas::<tiny_skia::Canvas>();
        let mut canvas = tiny_skia::Canvas::new(canvas_size);
        position.draw(&mut canvas);
        let png_bytes = canvas.to_png();
        w.write_all(&png_bytes)
            .context(format!("Could not write to file '{}'", png_fp))?;
    }

    let tt = ParallelTranspositionTable::new();
    let canonical_form = position.canonical_form(&tt);
    println!("Canonical Form: {}", canonical_form);
    println!("Temperature: {}", canonical_form.temperature());

    Ok(())
}
