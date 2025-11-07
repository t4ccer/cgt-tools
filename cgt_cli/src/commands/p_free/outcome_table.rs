use std::{
    collections::BTreeMap,
    sync::{Arc, atomic::AtomicBool},
};

use crate::commands::p_free::distinguish;
use anyhow::Result;
use cgt::misere::p_free::{GameForm, Outcome};
use clap::Parser;
use quickcheck::{Arbitrary, Gen};

/// Perform frobination (WIP, DO NOT USE)
#[derive(Parser, Debug, Clone)]
pub struct Args {
    #[arg(long)]
    size: u64,

    #[arg(long)]
    variant: distinguish::Variant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct TippingPoints {
    rl: u32,
    nl: u32,
    ln: u32,
    rn: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct PossibleOutcome {
    outcomes: [bool; 4],
}

impl PossibleOutcome {
    #[inline(always)]
    const fn none() -> PossibleOutcome {
        PossibleOutcome {
            outcomes: [false; 4],
        }
    }

    #[inline(always)]
    const fn mark_as_possible(&mut self, outcome: Outcome) -> bool {
        let was_not_possible = self.outcomes[outcome as usize];
        self.outcomes[outcome as usize] = true;
        !was_not_possible
    }

    #[inline(always)]
    const fn has_outcome(self, outcome: Outcome) -> bool {
        self.outcomes[outcome as usize]
    }
}

struct Latex<T>(T);

impl<T> std::fmt::Display for Latex<T>
where
    T: TexDisplay,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

trait TexDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

impl std::fmt::Display for PossibleOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_set();
        [Outcome::L, Outcome::R, Outcome::P, Outcome::N]
            .into_iter()
            .filter(|o| self.has_outcome(*o))
            .for_each(|o| {
                s.entry(&o);
            });
        s.finish()
    }
}

impl TexDisplay for PossibleOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\\{{")?;
        let mut first = true;
        [Outcome::L, Outcome::R, Outcome::P, Outcome::N]
            .into_iter()
            .filter(|o| self.has_outcome(*o))
            .try_for_each(|o| {
                if !first {
                    write!(f, ", ")?;
                }
                write!(f, "\\mathcal{{{o}}}")?;
                first = false;
                Ok(())
            })?;
        write!(f, "\\}}")
    }
}

#[allow(clippy::needless_pass_by_value)]
pub fn run(args: Args) -> Result<()> {
    let finished = Arc::new(AtomicBool::new(false));
    ctrlc::set_handler({
        let finished = Arc::clone(&finished);
        move || {
            finished.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    })
    .unwrap();

    let mut known = BTreeMap::<TippingPoints, PossibleOutcome>::new();

    let mut rnd = Gen::new(args.size as usize);
    eprintln!("r(l), n(l), l(n), r(n), o(n + l), n, l");
    while !finished.load(std::sync::atomic::Ordering::Relaxed) {
        let l = GameForm::arbitrary(&mut rnd);
        let n = GameForm::arbitrary(&mut rnd);
        if l.is_p_free()
            && args.variant.matches(&l)
            && l.outcome() == Outcome::L
            && n.is_p_free()
            && args.variant.matches(&n)
            && n.outcome() == Outcome::N
        {
            let tipping_points = TippingPoints {
                rl: l.right_tipping_point(),
                nl: l.next_tipping_point(),
                ln: n.left_tipping_point(),
                rn: n.right_tipping_point(),
            };
            let possible_outcomes = known
                .entry(tipping_points)
                .or_insert(PossibleOutcome::none());
            if possible_outcomes.has_outcome(Outcome::L)
                && possible_outcomes.has_outcome(Outcome::N)
            {
                continue;
            }

            let sum = GameForm::sum(&l, &n);
            let outcome = sum.outcome();
            if possible_outcomes.mark_as_possible(outcome) {
                eprintln!(
                    "{},    {},    {},    {},    {},        {}, {}",
                    tipping_points.rl,
                    tipping_points.nl,
                    tipping_points.ln,
                    tipping_points.rn,
                    outcome,
                    n,
                    l
                );
            }
        }
    }

    eprintln!();
    println!("r(l), n(l), l(n), r(n), o(n + l)");
    for (tipping_points, outcomes) in known {
        println!(
            "${}$ & ${}$ & ${}$ & ${}$ & ${}$ \\\\",
            tipping_points.rl,
            tipping_points.nl,
            tipping_points.ln,
            tipping_points.rn,
            Latex(outcomes),
        );
    }

    Ok(())
}
