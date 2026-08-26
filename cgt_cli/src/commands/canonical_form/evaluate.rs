use crate::io::FilePathOr;
use anyhow::{Context, Result};
use cgt::{
    drawing::{Draw, MeasuringCanvas, SvgCanvas, TinySkiaCanvas},
    short::partizan::canonical_form::CanonicalForm,
};
use clap::Parser;
use std::{
    fmt::Debug,
    io::{BufWriter, Stdout, Write},
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
    output_svg: Option<FilePathOr<Stdout>>,

    /// PNG render output path
    #[arg(long, default_value = None)]
    output_png: Option<FilePathOr<Stdout>>,
}

#[allow(clippy::needless_pass_by_value)]
pub fn run(args: Args) -> Result<()> {
    let canonical_form = CanonicalForm::from_str(&args.position).expect("Could not parse position");
    let thermograph = canonical_form.thermograph();

    if let Some(svg_fp) = &args.output_svg {
        let mut w = BufWriter::new(
            svg_fp
                .create()
                .context(format!("Could not create file '{}'", svg_fp))?,
        );

        let canvas_size =
            MeasuringCanvas::<SvgCanvas>::measure(&thermograph, Some(crate::MAX_CANVAS_SIZE));
        let mut canvas = SvgCanvas::new(canvas_size).with_max_canvas_size(crate::MAX_CANVAS_SIZE);
        thermograph.draw(&mut canvas);
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
        let canvas_size =
            MeasuringCanvas::<TinySkiaCanvas>::measure(&thermograph, Some(crate::MAX_CANVAS_SIZE));
        let mut canvas =
            TinySkiaCanvas::new(canvas_size).with_max_canvas_size(crate::MAX_CANVAS_SIZE);
        thermograph.draw(&mut canvas);
        let png_bytes = canvas.to_png();
        w.write_all(&png_bytes)
            .context(format!("Could not write to file '{}'", png_fp))?;
    }

    Ok(())
}
