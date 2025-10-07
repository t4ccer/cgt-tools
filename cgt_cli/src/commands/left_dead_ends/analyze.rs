use crate::io::FilePathOr;
use anyhow::{Context, Result};
use cgt::{
    misere::left_dead_end::{LeftDeadEndContext, interned::Interner},
    parsing::Parser,
};
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::{
    io::{BufRead, BufReader, BufWriter, Stdin, Stdout, Write},
    sync::Mutex,
};

/// Convert human-readable format from `factorizations` command into `jsonl`
#[derive(clap::Parser, Debug)]
pub struct Args {
    #[arg(long)]
    input: FilePathOr<Stdin>,

    #[arg(long, default_value = "-")]
    output: FilePathOr<Stdout>,

    #[arg(long, default_value = None)]
    threads: Option<u32>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Game {
    position: String,
    birthday: u32,
    race: u32,
    flexibility: u32,
    number_of_options: u32,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Log {
    a: Game,
    b: Game,
    c: Game,
    sum: Game,
    good_option_to_atom: bool,
}

#[allow(clippy::needless_pass_by_value)]
pub fn run(args: Args) -> Result<()> {
    if let Some(threads) = args.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads as usize)
            .build_global()
            .context("Could not build the thread pool")?;
    }

    let input = BufReader::new(
        args.input
            .open()
            .with_context(|| format!("Could not open input file `{}`", &args.input))?,
    );
    let output =
        Mutex::new(BufWriter::new(args.output.create().with_context(|| {
            format!("Could not open output file `{}`", &args.output)
        })?));

    let interner = Interner::new();
    input
        .lines()
        .par_bridge()
        .try_for_each::<_, Result<()>>(|line| {
            let line = line.context("Could not read input line")?;

            // Parses only first factorization but no non-unique factorization is known anyway
            let parse = || {
                let p = Parser::new(&line);
                let (p, summed) = interner.parse(p)?;
                let p = p.trim_whitespace();
                let p = p.parse_ascii_char('=')?;
                let p = p.trim_whitespace();
                let (p, a) = interner.parse(p)?;
                let p = p.trim_whitespace();
                let p = p.parse_ascii_char('+')?;
                let p = p.trim_whitespace();
                let (p, b) = interner.parse(p)?;
                let p = p.trim_whitespace();
                let p = p.parse_ascii_char('+')?;
                let p = p.trim_whitespace();
                let (_, c) = interner.parse(p)?;
                Some((a, b, c, summed))
            };
            let (a, b, c, g) =
                parse().with_context(|| format!("Could not parse input line: `{}`", line))?;

            let mk_game = |game| -> Game {
                Game {
                    position: interner.to_string(game),
                    birthday: interner.birthday(game),
                    race: interner.race(game),
                    flexibility: interner.flexibility(game),
                    number_of_options: interner.moves(game).count() as u32,
                }
            };

            let log = Log {
                a: mk_game(&a),
                b: mk_game(&b),
                c: mk_game(&c),
                sum: mk_game(&g),
                good_option_to_atom: interner.moves(&g).any(|h| interner.is_atom(&h)),
            };

            {
                let mut output = output.lock().ok().context("Output lock is poisoned")?;
                serde_json::ser::to_writer(&mut *output, &log)
                    .context("Could not write jsonl line")?;
                output
                    .write_all(b"\n")
                    .context("Could not write jsonl line")?;
            }

            Ok(())
        })?;

    Ok(())
}
