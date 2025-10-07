use crate::{commands::left_dead_ends::common::to_all_factorizations, io::FilePathOr};
use anyhow::{Context, Result};
use cgt::misere::left_dead_end::{
    LeftDeadEndContext,
    interned::{Interner, LeftDeadEnd},
};
use clap::{self, Parser};
use itertools::Itertools;
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::{
    io::{BufWriter, Stdout, Write},
    sync::{Mutex, atomic::AtomicU64},
};

#[derive(Debug, Clone, Parser)]
pub struct Args {
    #[arg(long, default_value_t = 8)]
    from: u32,

    #[arg(long, default_value_t = 10)]
    to: u32,

    #[arg(long, default_value = None)]
    threads: Option<u32>,

    #[arg(long, default_value = "-")]
    output: FilePathOr<Stdout>,
}

#[allow(clippy::needless_pass_by_value)]
pub fn run(args: Args) -> Result<()> {
    if let Some(threads) = args.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads as usize)
            .build_global()
            .context("Could not build the thread pool")?;
    }

    let output =
        Mutex::new(BufWriter::new(args.output.create().with_context(|| {
            format!("Could not open output file `{}`", &args.output)
        })?));

    let interner = Interner::new();

    let day0 = vec![LeftDeadEnd::new_integer(0)];
    let day1 = interner.next_day(day0.into_iter());
    let day2 = interner.next_day(day1);
    let day3 = interner.next_day(day2);
    let day4 = interner.next_day(day3).collect::<Vec<_>>();

    let current = AtomicU64::new(0);
    let total = choose(day4.len() as u64, 3);

    day4.into_iter()
        .tuple_combinations::<(_, _, _)>()
        .par_bridge()
        .for_each(|(a, b, c)| {
            let _logger = ProgressLogger::new(&current, total);

            if !interner.is_atom(&a) || !interner.is_atom(&b) || !interner.is_atom(&c) {
                return;
            }

            let d = interner.new_sum(&a, &b);
            let d = interner.new_sum(&d, &c);
            let d = interner.canonical(&d);

            let birthday = interner.birthday(&d);

            if !(args.from..=args.to).contains(&birthday) {
                return;
            }

            if interner.moves(&d).count() <= 1 {
                return;
            }

            let l = analyze_left_dead_end(&interner, &d);
            output.lock().unwrap().write_all(l.as_bytes()).unwrap();
        });

    eprintln!("len = {}", interner.len());
    output.lock().unwrap().flush().unwrap();

    Ok(())
}

fn analyze_left_dead_end(interner: &Interner, g: &LeftDeadEnd) -> String {
    let mut b = String::new();
    b.push_str(&interner.to_string(g));
    let atoms = to_all_factorizations(interner, g);
    for fs in &atoms {
        if fs.len() == 1 {
            continue;
        }

        b.push_str(" = ");
        for (idx, f) in fs.iter().enumerate() {
            if idx != 0 {
                b.push('+');
            }
            b.push_str(&interner.to_string(f));
        }
    }

    if atoms.len() != 1 {
        use std::fmt::Write;

        b.push_str(" !!! HERE !!! ");
        write!(b, "{atoms:?}").unwrap();
    }

    b.push('\n');
    b
}

fn choose(n: u64, k: u64) -> u64 {
    if k == 0 {
        1
    } else {
        n * choose(n - 1, k - 1) / k
    }
}

struct ProgressLogger<'c> {
    counter: &'c AtomicU64,
    total: u64,
}

impl ProgressLogger<'_> {
    pub const fn new(counter: &AtomicU64, total: u64) -> ProgressLogger<'_> {
        ProgressLogger { counter, total }
    }
}

impl Drop for ProgressLogger<'_> {
    fn drop(&mut self) {
        eprintln!(
            "[{}/{}]",
            self.counter
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            self.total
        );
    }
}
