use cgt::misere::p_free::GameForm;
use clap::{Parser, ValueEnum};
use quickcheck::{Arbitrary, Gen};

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum Variant {
    DeadEnding,
    Blocking,
}

impl Variant {
    pub fn matches(self, g: &GameForm) -> bool {
        match self {
            Variant::DeadEnding => g.is_dead_ending(),
            Variant::Blocking => g.is_blocking(),
        }
    }
}

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long, allow_hyphen_values = true)]
    pub lhs: GameForm,

    #[arg(long, allow_hyphen_values = true)]
    pub rhs: GameForm,

    /// Generator size
    #[arg(long)]
    pub size: u64,

    #[arg(long)]
    pub max_attempts: u64,

    #[arg(long, value_enum)]
    pub variant: Variant,
}

pub struct Stats {
    pub attempted: u64,
    pub ignored: u64,
    pub distinguisher: Option<GameForm>,
}

pub fn run_pure(args: &Args) -> Stats {
    let mut stats = Stats {
        attempted: 0,
        ignored: 0,
        distinguisher: None,
    };

    let mut rnd = Gen::new(args.size as usize);
    for _ in 0..args.max_attempts {
        stats.attempted += 1;
        let x = GameForm::arbitrary(&mut rnd);
        if x.is_p_free() && args.variant.matches(&x) {
            fn shrink_result(
                distinguisher: &GameForm,
                lhs: &GameForm,
                rhs: &GameForm,
                variant: Variant,
            ) -> Option<GameForm> {
                for shrunken in distinguisher.shrink() {
                    if shrunken.is_p_free() && variant.matches(&shrunken) {
                        let ol = GameForm::sum(lhs, &shrunken).outcome();
                        let or = GameForm::sum(rhs, &shrunken).outcome();

                        if ol != or {
                            let more_shrunken = shrink_result(&shrunken, lhs, rhs, variant);
                            return Some(more_shrunken.unwrap_or(shrunken));
                        }
                    }
                }
                None
            }

            let ol = GameForm::sum(&args.lhs, &x).outcome();
            let or = GameForm::sum(&args.rhs, &x).outcome();
            if ol != or {
                stats.distinguisher = Some(
                    match shrink_result(&x, &args.lhs, &args.rhs, args.variant) {
                        Some(x) => x,
                        None => x,
                    },
                );
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
