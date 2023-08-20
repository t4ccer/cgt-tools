use std::str::FromStr;

use anyhow::{Context, Result};
use cgt::short::partizan::canonical_form::CanonicalForm;
use clap::{arg, Parser};

#[derive(Parser, Debug, Clone)]
pub struct Args {
    /// Games to sum and compute the canonical form
    #[arg(required = true)]
    games: Vec<String>,
}

pub fn run(args: Args) -> Result<()> {
    let mut result = CanonicalForm::new_integer(0);

    for input in args.games {
        let canonical_form = CanonicalForm::from_str(&input)
            .ok()
            .context(format!("Could not parse game: '{}'", &input))?;
        result += canonical_form;
    }

    println!("{}", result);

    Ok(())
}
