use crate::commands::left_dead_ends::common::to_all_factorizations;
use anyhow::{Context, Result};
use cgt::misere::left_dead_end::interned::Interner;
use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long)]
    game: String,
}

#[allow(clippy::needless_pass_by_value)]
pub fn run(args: Args) -> Result<()> {
    let interner = Interner::new();
    let g = interner
        .new_from_string(&args.game)
        .context("Could not parse the game")?;

    println!("{}", interner.to_string(g));
    for factors in to_all_factorizations(&interner, g) {
        print!("  = ");
        for (i, f) in factors.into_iter().enumerate() {
            if i != 0 {
                print!(" + ");
            }
            print!("{}", interner.to_string(f));
        }
        println!();
    }

    println!("is atom: {}", interner.is_atom(g));
    println!("birthday: {}", interner.birthday(g));

    Ok(())
}
