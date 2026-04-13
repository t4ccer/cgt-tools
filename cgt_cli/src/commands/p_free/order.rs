use cgt::misere::game_form::{
    ConstructionError, DeadEndingContext, DeadEndingFormContext, GameFormContext, Outcome,
    PFreeFormContext, StandardFormContext,
};
use clap::ValueEnum;
use quickcheck::Gen;
use std::error::Error;

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum Variant {
    DeadEnding,
    Blocking,
}

impl Variant {
    pub fn matches<C>(self, context: &C, g: &C::Form) -> bool
    where
        C: GameFormContext,
    {
        match self {
            Variant::DeadEnding => context.is_dead_ending(g),
            Variant::Blocking => context.is_blocking(g),
        }
    }
}

#[derive(clap::Parser, Debug)]
pub struct Args {
    #[arg(long, allow_hyphen_values = true)]
    pub lhs: String,

    #[arg(long, allow_hyphen_values = true)]
    pub rhs: String,

    /// Generator size
    #[arg(long)]
    pub size: u64,

    #[arg(long)]
    pub max_attempts: u64,

    #[arg(long, value_enum)]
    pub variant: Variant,
}

// clap cannot set generic bounds so here we go
pub struct RichArgs<G> {
    pub lhs: G,
    pub rhs: G,
    pub size: u64,
    pub max_attempts: u64,
}

impl<G> RichArgs<G> {
    pub fn new<C>(context: &C, args: &Args) -> anyhow::Result<RichArgs<G>>
    where
        C: GameFormContext<Form = G>,
        C::IntegerConstructionError: Sync + Send + Error + 'static,
        C::DicoticConstructionError: Sync + Send + Error + 'static,
    {
        Ok(RichArgs {
            lhs: context.from_str(&args.lhs)?,
            rhs: context.from_str(&args.rhs)?,
            size: args.size,
            max_attempts: args.max_attempts,
        })
    }
}

pub struct SearchStatistics<T> {
    /// Number of total generated potential witnesses
    pub attempted: u64,

    /// Number of non-tested witnesses due to not being P-free or in the specified variant
    pub skipped: u64,

    pub result: T,
}

#[derive(Debug, Clone)]
pub enum Relation<G> {
    /// Possibly equal
    PossiblyEqual,

    /// Possibly less than, guaranteed not to be greater or equal
    PossiblyLessThan { not_ge_witness: Witness<G> },

    /// Possibly greater than, guaranteed not to be less or equal
    PossiblyGreaterThan { not_le_witness: Witness<G> },

    /// Guaranteed to be incomparable
    Incomparable {
        not_ge_witness: Witness<G>,
        not_le_witness: Witness<G>,
    },
}

#[derive(Debug, Clone, Copy)]
enum CheckedOrder {
    LessThanOrEqual,
    GreaterThanOrEqual,
}

fn violates_order<C>(
    context: &C,
    x: &C::Form,
    lhs: &C::Form,
    rhs: &C::Form,
    order: CheckedOrder,
) -> Option<(Outcome, Outcome)>
where
    C: GameFormContext,
{
    let ol = match context.sum(lhs, x) {
        Ok(sum) => context.outcome(&sum),
        Err(err) => context.base_context().outcome(&err.recover()),
    };

    let or = match context.sum(rhs, x) {
        Ok(sum) => context.outcome(&sum),
        Err(err) => context.base_context().outcome(&err.recover()),
    };

    let violated = match order {
        CheckedOrder::GreaterThanOrEqual => ol < or,
        CheckedOrder::LessThanOrEqual => or < ol,
    };

    violated.then_some((ol, or))
}

fn shrink_witness<C>(
    context: &C,
    distinguisher: &C::Form,
    lhs: &C::Form,
    rhs: &C::Form,
    order: CheckedOrder,
) -> Option<Witness<C::Form>>
where
    C: GameFormContext,
{
    for shrunken in context.shrink(distinguisher) {
        if let Some((ol, or)) = violates_order(context, &shrunken, lhs, rhs, order) {
            let more_shrunken = shrink_witness(context, &shrunken, lhs, rhs, order);
            return Some(more_shrunken.unwrap_or(Witness {
                game: shrunken,
                lhs_outcome: ol,
                rhs_outcome: or,
            }));
        }
    }
    None
}

#[derive(Debug, Clone)]
pub struct Witness<G> {
    game: G,
    lhs_outcome: Outcome,
    rhs_outcome: Outcome,
}

/// Check if games satisfy specified `order` and return the distinguishing `x` if not
fn find_witness<C>(
    context: &C,
    args: &RichArgs<C::Form>,
    order: CheckedOrder,
) -> SearchStatistics<Option<Witness<C::Form>>>
where
    C: GameFormContext,
{
    let mut statistics = SearchStatistics {
        attempted: 0,
        skipped: 0,
        result: None,
    };

    let mut rnd = Gen::new(args.size as usize);
    for _ in 0..args.max_attempts {
        statistics.attempted += 1;

        let Ok(x) = context.arbitrary(&mut rnd) else {
            statistics.skipped += 1;
            continue;
        };

        if let Some((ol, or)) = violates_order(context, &x, &args.lhs, &args.rhs, order) {
            statistics.result = Some(
                shrink_witness(context, &x, &args.lhs, &args.rhs, order).unwrap_or(Witness {
                    game: x,
                    lhs_outcome: ol,
                    rhs_outcome: or,
                }),
            );
            break;
        }
    }

    statistics
}

pub fn check_relation_possibility<C>(
    context: &C,
    args: &RichArgs<C::Form>,
) -> SearchStatistics<Relation<C::Form>>
where
    C: GameFormContext,
{
    let le = find_witness(context, args, CheckedOrder::LessThanOrEqual);
    let ge = find_witness(context, args, CheckedOrder::GreaterThanOrEqual);

    SearchStatistics {
        attempted: le.attempted + ge.attempted,
        skipped: le.attempted + ge.skipped,
        result: match (le.result, ge.result) {
            // No counterexample for either >= or <= found
            (None, None) => Relation::PossiblyEqual,
            (Some(not_le_witness), None) => Relation::PossiblyGreaterThan { not_le_witness },
            (None, Some(not_ge_witness)) => Relation::PossiblyLessThan { not_ge_witness },
            // Counterexample found for both >= and <= so the games are incomparable
            (Some(not_le_witness), Some(not_ge_witness)) => Relation::Incomparable {
                not_le_witness,
                not_ge_witness,
            },
        },
    }
}

fn show_results<C>(context: &C, args: &RichArgs<C::Form>)
where
    C: GameFormContext,
{
    let stats = check_relation_possibility(context, args);
    eprintln!("Attempted: {}", stats.attempted);
    eprintln!(
        "Ignored: {} ({:.2}%)",
        stats.skipped,
        stats.skipped as f32 / stats.attempted as f32 * 100.0
    );

    match stats.result {
        Relation::PossiblyEqual => {
            println!(
                "Possibly: {} = {}",
                context.display(&args.lhs),
                context.display(&args.rhs)
            );
        }
        Relation::PossiblyLessThan { not_ge_witness } => {
            println!(
                "Witness for not (>=): {}",
                context.display(&not_ge_witness.game)
            );
            println!(
                "o({} + {}) = {}",
                context.display(&args.lhs),
                context.display(&not_ge_witness.game),
                not_ge_witness.lhs_outcome,
            );
            println!(
                "o({} + {}) = {}",
                context.display(&args.rhs),
                context.display(&not_ge_witness.game),
                not_ge_witness.rhs_outcome,
            );
            println!(
                "Guaranteed: {} != {}",
                context.display(&args.rhs),
                context.display(&args.lhs)
            );
            println!(
                "Possibly: {} > {}",
                context.display(&args.rhs),
                context.display(&args.lhs)
            );
        }
        Relation::PossiblyGreaterThan { not_le_witness } => {
            println!(
                "Witness for not (<=): {}",
                context.display(&not_le_witness.game)
            );
            println!(
                "o({} + {}) = {}",
                context.display(&args.lhs),
                context.display(&not_le_witness.game),
                not_le_witness.lhs_outcome
            );
            println!(
                "o({} + {}) = {}",
                context.display(&args.rhs),
                context.display(&not_le_witness.game),
                not_le_witness.rhs_outcome
            );
            println!(
                "Guaranteed: {} != {}",
                context.display(&args.lhs),
                context.display(&args.rhs)
            );
            println!(
                "Possibly: {} > {}",
                context.display(&args.lhs),
                context.display(&args.rhs)
            );
        }
        Relation::Incomparable {
            not_ge_witness,
            not_le_witness,
        } => {
            println!(
                "Witness for not (>=): {}",
                context.display(&not_ge_witness.game)
            );
            println!(
                "o({} + {}) = {}",
                context.display(&args.lhs),
                context.display(&not_ge_witness.game),
                not_ge_witness.lhs_outcome,
            );
            println!(
                "o({} + {}) = {}",
                context.display(&args.rhs),
                context.display(&not_ge_witness.game),
                not_ge_witness.rhs_outcome,
            );
            println!(
                "Witness for not (<=): {}",
                context.display(&not_le_witness.game)
            );
            println!(
                "o({} + {}) = {}",
                context.display(&args.lhs),
                context.display(&not_le_witness.game),
                not_le_witness.lhs_outcome
            );
            println!(
                "o({} + {}) = {}",
                context.display(&args.rhs),
                context.display(&not_le_witness.game),
                not_le_witness.rhs_outcome
            );
            println!(
                "Guaranteed: {} || {}",
                context.display(&args.lhs),
                context.display(&args.rhs)
            );
        }
    }
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
pub fn run(args: Args) -> anyhow::Result<()> {
    match args.variant {
        Variant::DeadEnding => {
            let context = PFreeFormContext::new(DeadEndingFormContext::new(StandardFormContext));
            let args = RichArgs::new(&context, &args)?;
            eprintln!("--------------------");
            eprintln!(
                "{} >= {} (mod pf(E)) => {}",
                context.display(&args.lhs),
                context.display(&args.rhs),
                context.ge_mod_dead_ending(&args.lhs, &args.rhs)
            );
            eprintln!(
                "{} <= {} (mod pf(E)) => {}",
                context.display(&args.lhs),
                context.display(&args.rhs),
                context.ge_mod_dead_ending(&args.rhs, &args.lhs)
            );
            eprintln!("--------------------");
            show_results(&context, &args);
        }
        Variant::Blocking => todo!(),
    }
    Ok(())
}
