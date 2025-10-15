use cgt::misere::p_free::GameForm;
use clap::{Parser, ValueEnum};
use quickcheck::{Arbitrary, Gen};

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum Variant {
    DeadEnding,
    Blocking,
}

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long, allow_hyphen_values = true)]
    lhs: GameForm,

    #[arg(long, allow_hyphen_values = true)]
    rhs: GameForm,

    /// Generator size
    #[arg(long)]
    size: u64,

    #[arg(long)]
    max_attempts: u64,

    #[arg(long, value_enum)]
    variant: Variant,
}

pub struct Stats {
    attempted: u64,
    ignored: u64,
    distinguisher: Option<GameForm>,
}

pub fn run_pure(args: &Args) -> Stats {
    let mut stats = Stats {
        attempted: 0,
        ignored: 0,
        distinguisher: None,
    };

    let variant = |x: &GameForm| match args.variant {
        Variant::DeadEnding => x.is_dead_ending(),
        Variant::Blocking => x.is_blocking(),
    };

    let mut rnd = Gen::new(args.size as usize);
    for _ in 0..args.max_attempts {
        stats.attempted += 1;
        let x = GameForm::arbitrary(&mut rnd);
        if x.is_p_free() && variant(&x) {
            fn shrink_result(
                x: &GameForm,
                lhs: &GameForm,
                rhs: &GameForm,
                variant: &impl Fn(&GameForm) -> bool,
            ) -> Option<GameForm> {
                for t in x.shrink() {
                    if x.is_p_free() && variant(&x) {
                        let ol = GameForm::sum(lhs, &x).outcome();
                        let or = GameForm::sum(rhs, &x).outcome();
                        if ol != or {
                            let shrunk = shrink_result(&t, lhs, rhs, variant);
                            return Some(shrunk.unwrap_or(t));
                        }
                    }
                }
                None
            }

            let ol = GameForm::sum(&args.lhs, &x).outcome();
            let or = GameForm::sum(&args.rhs, &x).outcome();
            if ol != or {
                stats.distinguisher =
                    Some(match shrink_result(&x, &args.lhs, &args.rhs, &variant) {
                        Some(x) => x,
                        None => x,
                    });
                break;
            }
        } else {
            stats.ignored += 1;
        }
    }

    stats
}

pub fn run(args: Args) -> anyhow::Result<()> {
    let stats = run_pure(&args);
    eprintln!("Attempted: {}", stats.attempted);
    eprintln!(
        "Ignored: {} ({:.2}%)",
        stats.ignored,
        stats.ignored as f32 / stats.attempted as f32 * 100.0
    );
    match stats.distinguisher {
        Some(x) => println!("{x}"),
        None => anyhow::bail!("Could not find a distinguishing game"),
    }

    Ok(())
}
