use crate::{
    domineering::common::DomineeringResult,
    io::{FileOrStderr, FileOrStdout},
};
use anyhow::{bail, Context, Result};
use cgt::{
    genetic_algorithm::{Algorithm, GeneticAlgorithm},
    grid::{small_bit_grid::SmallBitGrid, BitTile, FiniteGrid, Grid},
    numeric::dyadic_rational_number::DyadicRationalNumber,
    short::partizan::{
        games::domineering::{Domineering, Tile},
        partizan_game::PartizanGame,
        transposition_table::ParallelTranspositionTable,
    },
};
use clap::Parser;
use rand::Rng;
use std::{collections::HashSet, io::Write, num::NonZeroUsize, str::FromStr};

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Args {
    #[arg(long)]
    generation_size: NonZeroUsize,

    #[arg(long)]
    mutation_rate: f32,

    #[arg(long)]
    width: u8,

    #[arg(long)]
    height: u8,

    /// SVG render output path
    #[arg(long, default_value = None)]
    output_svg: Option<String>,

    /// Path to output positions
    #[arg(long, default_value = "-")]
    out_file: FileOrStdout,

    /// Path to output diagnostics
    #[arg(long, default_value = "-")]
    diagnostics: FileOrStderr,

    #[arg(long, default_value = "7/4")]
    temperature_threshold: DyadicRationalNumber,

    #[arg(long, default_value = None)]
    seed: Option<String>,
}

struct DomineeringHighTemperature {
    transposition_table: ParallelTranspositionTable<Domineering>,
    mutation_rate: f32,
    grid_width: u8,
    grid_height: u8,
}

impl Algorithm<Domineering, DyadicRationalNumber> for DomineeringHighTemperature {
    fn mutate(&self, object: &mut Domineering, rng: &mut rand::rngs::ThreadRng) {
        for y in 0..object.grid().height() {
            for x in 0..object.grid().width() {
                if rng.gen::<f32>() <= self.mutation_rate {
                    let old = object.grid().get(x, y);
                    let new = match old {
                        Tile::Empty => Tile::Taken,
                        Tile::Taken => Tile::Empty,
                    };
                    object.grid_mut().set(x, y, new);
                }
            }
        }
    }

    fn cross(
        &self,
        lhs: &Domineering,
        rhs: &Domineering,
        rng: &mut rand::rngs::ThreadRng,
    ) -> Domineering {
        let mid_point = rng.gen_range(0..(lhs.grid().height() * lhs.grid().width()));

        let mut new = *lhs;
        for y in 0..new.grid().height() {
            for x in 0..new.grid().width() {
                let tile = if x * y < mid_point {
                    lhs.grid().get(x, y)
                } else {
                    rhs.grid().get(x, y)
                };
                new.grid_mut().set(x, y, tile);
            }
        }

        new
    }

    fn lowest_score(&self) -> DyadicRationalNumber {
        DyadicRationalNumber::from(-100)
    }

    fn score(&self, object: &Domineering) -> DyadicRationalNumber {
        let clean = object.clone().move_top_left();
        if object.decompositions().len() > 1 || object != &clean {
            self.lowest_score()
        } else {
            object
                .canonical_form(&self.transposition_table)
                .temperature()
        }
    }

    fn random(&self, rng: &mut rand::rngs::ThreadRng) -> Domineering {
        let mut new =
            Domineering::new(SmallBitGrid::empty(self.grid_width, self.grid_height).unwrap());

        for y in 0..new.grid().height() {
            for x in 0..new.grid().width() {
                new.grid_mut()
                    .set(x, y, Tile::bool_to_tile(rng.gen_bool(0.5)));
            }
        }

        new
    }
}

// "##.#.##|##...##|....#..|#.....#|..##...|##...##|##.#.##"

pub fn run(args: Args) -> Result<()> {
    let alg = DomineeringHighTemperature {
        transposition_table: ParallelTranspositionTable::new(),
        mutation_rate: args.mutation_rate,
        grid_width: args.width,
        grid_height: args.height,
    };

    let specimen = if let Some(seed_input) = args.seed {
        let pos: Domineering = Domineering::from_str(&seed_input)
            .ok()
            .context("Could not parse seed position")?;

        if pos.grid().width() != args.width {
            bail!(
                "Seed position has width {}, expected {}",
                pos.grid().width(),
                args.width
            );
        }

        if pos.grid().height() != args.height {
            bail!(
                "Seed position has height {}, expected {}",
                pos.grid().height(),
                args.height
            );
        }

        vec![pos]
    } else {
        vec![]
    };
    let mut alg = GeneticAlgorithm::with_specimen(specimen, args.generation_size, alg);

    let mut visited = HashSet::new();

    let mut output = args
        .out_file
        .create()
        .context("Could not create/open output file")?;

    let mut diagnostics = args
        .diagnostics
        .create()
        .context("Could not create/open diagnostics file")?;

    loop {
        alg.step_generation();

        alg.specimen()
            .iter()
            .rev()
            .take_while(|s| s.score >= args.temperature_threshold)
            .try_for_each(|s| -> Result<()> {
                if visited.insert(s.object) {
                    let result = DomineeringResult {
                        grid: s.object.to_string(),
                        temperature: s.score.to_string(),
                    };
                    writeln!(output, "{}", serde_json::ser::to_string(&result).unwrap())
                        .context("Could not output position")?;
                    output.flush().context("Could not flush logs")?;
                }
                Ok(())
            })?;

        let best = alg.highest_score();
        writeln!(
            diagnostics,
            "Generation: {}\tBest score: {}",
            alg.generation(),
            best.score
        )
        .context("Could not output logs")?;
    }
}
