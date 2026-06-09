use cgt::{
    drawing::{Draw, tikz},
    graph::{adjacency_matrix::undirected::UndirectedGraph, bipartite::BipartiteGraph},
    latex::LatexMathEscape,
    short::partizan::{
        canonical_form::CanonicalForm,
        games::bipartite_snort::{BipartiteSnortIterator, VertexColor},
        partizan_game::PartizanGame,
        transposition_table::ParallelTranspositionTable,
    },
    total::TotalWrapper,
};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufWriter, Write},
};

#[derive(Parser, Debug, Clone)]
pub struct Args {
    #[arg(long)]
    blue: u32,

    #[arg(long)]
    red: u32,

    #[arg(long)]
    out_dir: String,

    #[arg(long, default_value_t = false)]
    sum_images: bool,
}

pub fn run(args: Args) -> anyhow::Result<()> {
    let padding = 0.5;

    let mut out = BufWriter::new(File::create(format!("{}/feasible.tex", args.out_dir))?);
    let mut seen_cf = HashMap::<TotalWrapper<CanonicalForm>, _>::new();
    let mut seen_reduced = HashMap::<TotalWrapper<CanonicalForm>, _>::new();
    let tt = ParallelTranspositionTable::new();

    for blue in 0..=args.blue {
        for red in blue..=args.red {
            writeln!(out, "\\begin{{longtable}}{{cccc}}")?;
            writeln!(
                out,
                "  \\caption{{Positions with {blue} Blue and {red} Red Vertices}}\\\\",
            )?;
            writeln!(out, "  Graph & Position & Canonical Form & Temp. \\\\")?;
            writeln!(out, "  \\midrule \\endhead%")?;

            eprintln!("Enumerating {blue} blue {red} red");
            let bar = ProgressBar::new(
                BipartiteSnortIterator::<UndirectedGraph<VertexColor>, fn()>::upper_bound(
                    blue, red,
                ),
            )
            .with_style(progress_style());
            for (graph, snort) in
                BipartiteSnortIterator::<UndirectedGraph<VertexColor>, _>::with_callback(
                    blue,
                    red,
                    || {
                        bar.inc(1);
                    },
                )
            {
                let canonical_form = snort.canonical_form(&tt);
                if seen_cf
                    .get(TotalWrapper::from_ref(&canonical_form))
                    .is_some()
                {
                    continue;
                }

                let temperature = canonical_form.temperature();

                let mut canvas = tikz::Canvas::new();
                snort.draw(&mut canvas);
                writeln!(
                    out,
                    "  {} & \\begin{{tikzpicture}}[baseline={{([yshift=-{padding}cm]current bounding box.center)}}]{}\\useasboundingbox (current bounding box.south west) rectangle ([yshift={padding}cm]current bounding box.north east);\\end{{tikzpicture}} & $\\mathsmaller{{{}}}$ & ${}$ \\\\",
                    graph,
                    canvas.to_tikz(),
                    LatexMathEscape(&canonical_form),
                    temperature,
                )?;
                let reduced = canonical_form.reduced();
                if seen_reduced.get(TotalWrapper::from_ref(&reduced)).is_none() {
                    seen_reduced.insert(TotalWrapper::new(reduced), (graph, snort.clone()));
                }

                seen_cf.insert(TotalWrapper::new(canonical_form), (graph, snort));
            }

            writeln!(out, "\\end{{longtable}}")?;

            drop(bar);
            eprintln!();
        }
    }
    drop(out);

    let mut out = BufWriter::new(File::create(format!("{}/reduced.tex", args.out_dir))?);
    writeln!(out, "\\begin{{longtable}}{{cccc}}")?;
    writeln!(out, "  \\caption{{Reduced Canonical Form}}\\\\",)?;
    writeln!(out, "  Graph & Position & Reduced & Birthday \\\\")?;
    writeln!(out, "  \\midrule \\endhead%")?;
    for (canonical_form, (graph, snort)) in seen_reduced.iter() {
        let canonical_form: &CanonicalForm = &canonical_form;
        let mut canvas = tikz::Canvas::new();
        snort.draw(&mut canvas);
        writeln!(
            out,
            "  {} & \\begin{{tikzpicture}}[baseline={{([yshift=-{padding}cm]current bounding box.center)}}]{}\\useasboundingbox (current bounding box.south west) rectangle ([yshift={padding}cm]current bounding box.north east);\\end{{tikzpicture}} & $\\mathsmaller{{{}}}$ & ${}$ \\\\",
            graph,
            canvas.to_tikz(),
            LatexMathEscape(canonical_form),
            canonical_form.birthday(),
        )?;
    }
    writeln!(out, "\\end{{longtable}}")?;

    eprintln!("Finding own inverses");
    let mut out = BufWriter::new(File::create(format!("{}/own-negative.tex", args.out_dir))?);
    writeln!(out, "\\begin{{longtable}}{{cccc}}")?;
    writeln!(out, "  \\caption{{Own Inverses}}\\\\",)?;
    writeln!(out, "  Graph & Position & Canonical Form \\\\")?;
    writeln!(out, "  \\midrule \\endhead%")?;
    let bar = ProgressBar::new(seen_cf.len() as u64).with_style(progress_style());
    for (cf, (graph, snort)) in seen_cf.iter() {
        let cf: &CanonicalForm = &cf;

        if *cf == -cf {
            let mut canvas = tikz::Canvas::new();
            snort.draw(&mut canvas);
            writeln!(
                out,
                "  {} & \\begin{{tikzpicture}}[baseline={{([yshift=-{padding}cm]current bounding box.center)}}]{}\\useasboundingbox (current bounding box.south west) rectangle ([yshift={padding}cm]current bounding box.north east);\\end{{tikzpicture}} & $\\mathsmaller{{{}}}$ \\\\",
                graph,
                canvas.to_tikz(),
                LatexMathEscape(cf),
            )?;
        }
        bar.inc(1);
    }
    writeln!(out, "\\end{{longtable}}")?;
    drop(bar);
    eprintln!();
    drop(out);

    eprintln!("Finding sums");
    let tikz_prologue = format!(
        "\\begin{{tikzpicture}}[scale=0.5,baseline={{([yshift=-{padding}cm]current bounding box.center)}}]"
    );
    let tikz_epilogue = format!(
        "\\useasboundingbox (current bounding box.south west) rectangle ([yshift={padding}cm]current bounding box.north east);\\end{{tikzpicture}}"
    );
    let bar =
        ProgressBar::new(seen_cf.len() as u64 * seen_cf.len() as u64).with_style(progress_style());
    let mut out = BufWriter::new(File::create(format!(
        "{}/realisable-as-sums.tex",
        args.out_dir
    ))?);
    writeln!(out, "\\begin{{longtable}}{{ccc}}")?;
    writeln!(out, "  \\caption{{Realisable as Sums}}\\\\",)?;
    writeln!(out, "  $G$ & $H$ & $G + H$\\\\")?;
    writeln!(out, "  \\midrule \\endhead%")?;
    for (cf1, (graph1, snort1)) in seen_cf.iter() {
        let cf1: &CanonicalForm = &cf1;
        if *cf1 == CanonicalForm::new_integer(0) {
            continue;
        }

        let mut canvas1 = tikz::Canvas::new();
        snort1.draw(&mut canvas1);

        for (cf2, (graph2, snort2)) in seen_cf.iter() {
            bar.inc(1);

            let cf2: &CanonicalForm = &cf2;
            if *cf2 == CanonicalForm::new_integer(0) {
                continue;
            }

            let cf = cf1 + cf2;
            let Some((graph, snort)) = seen_cf.get(TotalWrapper::from_ref(&cf)) else {
                continue;
            };

            if cf == CanonicalForm::new_integer(0) {
                continue;
            }

            let mut canvas2 = tikz::Canvas::new();
            snort2.draw(&mut canvas2);

            let mut canvas = tikz::Canvas::new();
            snort.draw(&mut canvas);

            let write = |out: &mut BufWriter<File>,
                         graph: &BipartiteGraph,
                         cf: &CanonicalForm,
                         canvas: &tikz::Canvas| {
                if args.sum_images {
                    write!(
                        out,
                        "\\makecell[t]{{{} \\\\ $\\mathsmaller{{{}}}$ \\\\ {tikz_prologue}{}{tikz_epilogue}}}",
                        graph,
                        LatexMathEscape(cf),
                        canvas.to_tikz()
                    )
                } else {
                    write!(
                        out,
                        "\\makecell[t]{{{} \\\\ $\\mathsmaller{{{}}}$}}",
                        graph,
                        LatexMathEscape(cf),
                    )
                }
            };
            write(&mut out, graph1, cf1, &canvas1)?;
            write!(out, " & ")?;
            write(&mut out, graph2, cf2, &canvas2)?;
            write!(out, " & ")?;
            write(&mut out, graph, &cf, &canvas)?;
            writeln!(out, " \\\\ \\midrule")?;
        }
    }
    writeln!(out, "\\end{{longtable}}")?;
    drop(bar);
    eprintln!();
    drop(out);

    Ok(())
}

fn progress_style() -> ProgressStyle {
    ProgressStyle::with_template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
        .unwrap()
        .progress_chars("#> ")
}
