use crate::commands::p_free::distinguish;
use anyhow::Result;
use cgt::misere::p_free::GameForm;
use clap::Parser;
use quickcheck::{Arbitrary, Gen};

/// Perform frobination (WIP, DO NOT USE)
#[derive(Parser, Debug, Clone)]
pub struct Args {
    #[arg(long)]
    size: u64,

    #[arg(long)]
    variant: distinguish::Variant,

    #[arg(long, default_value_t = 1000)]
    max_attempts: u64,
}

#[allow(clippy::needless_pass_by_value)]
pub fn run(args: Args) -> Result<()> {
    let mut rnd = Gen::new(args.size as usize);
    loop {
        let g = GameForm::arbitrary(&mut rnd);
        if g.is_p_free()
            && args.variant.matches(&g)
            && g.left_tipping_point() == 1
            && g.right_tipping_point() == 1
        {
            eprintln!("o({g}) = {}", g.outcome());
            let res = distinguish::run_pure(&distinguish::Args {
                lhs: g.clone(),
                rhs: GameForm::new_integer(0),
                size: args.size,
                max_attempts: args.max_attempts,
                variant: args.variant,
            });
            if let Some(x) = res.distinguisher {
                eprintln!("o({g} + {x}) = {}", GameForm::sum(&g, &x).outcome());
                eprintln!("FOUND: {g} + {x} /= 0 + {x}");
            } else {
                eprintln!("NOPE");
            }
            eprintln!();
        }
    }

    // Ok(())
}
