use anyhow::{Context, Result};
use cgt::short::partizan::canonical_form::CanonicalForm;
use clap::{arg, Parser};
use std::{fmt::Write, str::FromStr};

#[derive(Parser, Debug, Clone)]
pub struct Args {
    /// Games to sum and compute the canonical form
    #[arg(required = true)]
    games: Vec<String>,
}

pub fn run(args: Args) -> Result<()> {
    let mut result = CanonicalForm::new_integer(0);
    let mut buf = String::new();

    for (idx, input) in args.games.iter().enumerate() {
        if idx != 0 {
            buf.write_str(" + ")?;
        }

        let canonical_form = CanonicalForm::from_str(input)
            .ok()
            .context(format!("Could not parse game: '{}'", &input))?;
        buf.write_str(&canonical_form.to_string())?;
        result += canonical_form;
    }

    write!(buf, " = {}", result)?;

    println!("{}", buf);
    println!("temperature({}) = {}", result, result.temperature());

    Ok(())
}
