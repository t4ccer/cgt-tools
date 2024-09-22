use crate::{commands::snort::common::Log, io::FileOrStderr};
use anyhow::{Context, Result};
use cgt::{
    genetic_algorithm::{Algorithm, GeneticAlgorithm, Scored},
    graph::undirected,
    numeric::rational::Rational,
    short::partizan::{
        games::snort::{Snort, VertexColor, VertexKind},
        partizan_game::PartizanGame,
        transposition_table::ParallelTranspositionTable,
    },
};
use clap::{self, Parser};
use rand::{seq::SliceRandom, Rng};
use std::{
    cmp::min,
    fs::File,
    io::{BufReader, BufWriter, Write},
    num::NonZeroUsize,
};

#[derive(Parser, Debug, Clone)]
/// Run genetic algorithm on Snort game to find positions with high difference between temperature and degree
pub struct Args {
    #[arg(long)]
    generation_size: NonZeroUsize,

    /// Do not generate graphs with more that that vertices
    #[arg(long)]
    max_graph_vertices: usize,

    #[arg(long)]
    mutation_rate: f32,

    /// Stop after running that many generations. Run forever otherwise
    #[arg(long, default_value = None)]
    generation_limit: Option<usize>,

    /// Path to saved snapshot to be loaded
    #[arg(long, default_value = None)]
    snapshot_load_file: Option<String>,

    /// Path to save snapshot file
    #[arg(long)]
    snapshot_save_file: String,

    /// Path to output logs
    #[arg(long)]
    out_file: FileOrStderr,

    /// Clean up transpositon table after that many generations
    #[arg(long, default_value_t = 50)]
    cleanup_interval: usize,

    /// Save if score is above that value
    #[arg(long, default_value_t = Rational::from(0))]
    save_eq_or_above: Rational,
}

struct SnortTemperatureDegreeDifference {
    transposition_table: ParallelTranspositionTable<Snort>,
    max_graph_vertices: usize,
    mutation_rate: f32,
}

impl SnortTemperatureDegreeDifference {
    fn mutate_with_rate(
        &self,
        position: &mut Snort,
        rng: &mut rand::rngs::ThreadRng,
        mutation_rate: f32,
    ) {
        // Mutate vertices
        if position.graph.size() > 1 {
            let mutation_roll: f32 = rng.gen();
            if mutation_roll < mutation_rate {
                let to_remove = rng.gen_range(0..position.graph.size());
                position.graph.remove_vertex(to_remove);
                position.vertices.remove(to_remove);
            }
        }
        // TODO: Check for max size
        // if position.graph.size()
        let mutation_roll: f32 = rng.gen();
        if mutation_roll < mutation_rate {
            position.graph.add_vertex();
            position
                .vertices
                .push(VertexKind::Single(VertexColor::Empty));
            let another_vertex = rng.gen_range(0..position.graph.size() - 1);
            position
                .graph
                .connect(position.graph.size() - 1, another_vertex, true);
        }

        // Mutate edges
        for v in position.graph.vertices() {
            for u in position.graph.vertices() {
                if v == u {
                    continue;
                }

                let mutation_roll: f32 = rng.gen();
                if mutation_roll < mutation_rate {
                    position
                        .graph
                        .connect(v, u, !position.graph.are_adjacent(v, u));
                }
            }
        }

        // Mutate colors
        let available_colors = vec![
            VertexColor::Empty,
            VertexColor::TintLeft,
            VertexColor::TintRight,
        ];
        for idx in 0..position.vertices.len() {
            let mutation_roll: f32 = rng.gen();
            if mutation_roll < mutation_rate {
                position.vertices[idx] = VertexKind::Single(*available_colors.choose(rng).unwrap());
            }
        }
    }
}

impl Algorithm<Snort, Rational> for SnortTemperatureDegreeDifference {
    fn mutate(&self, position: &mut Snort, rng: &mut rand::rngs::ThreadRng) {
        self.mutate_with_rate(position, rng, self.mutation_rate);
    }

    fn cross(&self, lhs: &Snort, rhs: &Snort, _rng: &mut rand::rngs::ThreadRng) -> Snort {
        let mut rng = rand::thread_rng();

        let mut positions = [lhs, rhs];
        positions.sort_by_key(|pos| pos.graph.size());
        let [smaller, larger] = positions;

        let new_size = rng.gen_range(1..=larger.graph.size());
        let mut new_graph = undirected::Graph::empty(new_size);

        for v in 0..(min(new_size, smaller.graph.size())) {
            for u in 0..(min(new_size, smaller.graph.size())) {
                new_graph.connect(v, u, smaller.graph.are_adjacent(v, u));
            }
        }
        for v in (min(new_size, smaller.graph.size()))..(min(new_size, larger.graph.size())) {
            for u in (min(new_size, smaller.graph.size()))..(min(new_size, larger.graph.size())) {
                new_graph.connect(v, u, larger.graph.are_adjacent(v, u));
            }
        }

        let mut colors = smaller.vertices[0..(min(new_size, smaller.graph.size()))].to_vec();
        colors.extend(
            &larger.vertices
                [(min(new_size, smaller.graph.size()))..(min(new_size, larger.graph.size()))],
        );

        Snort::with_colors(colors, new_graph).unwrap()
    }

    fn lowest_score(&self) -> Rational {
        Rational::NegativeInfinity
    }

    fn score(&self, position: &Snort) -> Rational {
        let degree_sum = position.graph.degrees().sum::<usize>();
        if position.vertices.is_empty() || degree_sum == 0 || !position.graph.is_connected() {
            return Rational::NegativeInfinity;
        }

        let game = position.canonical_form(&self.transposition_table);
        let temp = game.temperature();
        let degree = position.degree();
        temp.to_rational() - Rational::from(degree as i64)
    }

    fn random(&self, rng: &mut rand::rngs::ThreadRng) -> Snort {
        let graph_size = rng.gen_range(1..=self.max_graph_vertices);
        let graph = undirected::Graph::empty(graph_size);
        let mut position = Snort::new(graph);
        self.mutate_with_rate(&mut position, rng, 1.0);
        position
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Snapshot {
    specimen: Vec<Scored<Snort, Rational>>,
}

fn seed_positions() -> Vec<Snort> {
    // 0   5   6     11
    //  \   \ /     /
    // 1--3--4--10-12
    //  /   /|\     \
    // 2   7 8 9     13
    let pos_1 = Snort::new(undirected::Graph::from_edges(
        14,
        &[
            (0, 3),
            (1, 3),
            (2, 3),
            (3, 4),
            (4, 5),
            (4, 6),
            (4, 7),
            (4, 8),
            (4, 9),
            (4, 10),
            (10, 11),
            (10, 12),
            (10, 13),
        ],
    ));

    //         9
    //         |
    // 0  5   7-10
    //  \  \ / \11
    // 1-3--4
    //  /  / \ /12
    // 2  6   8 -13
    //        |
    //        14
    let pos_2 = Snort::new(undirected::Graph::from_edges(
        15,
        &[
            (0, 3),
            (1, 3),
            (2, 3),
            (3, 4),
            (4, 5),
            (4, 6),
            (4, 7),
            (4, 8),
            (7, 9),
            (7, 10),
            (7, 11),
            (8, 12),
            (8, 13),
            (8, 14),
        ],
    ));

    vec![pos_1, pos_2]
}

pub fn run(args: Args) -> Result<()> {
    let alg = SnortTemperatureDegreeDifference {
        transposition_table: ParallelTranspositionTable::new(),
        max_graph_vertices: args.max_graph_vertices,
        mutation_rate: args.mutation_rate,
    };

    let specimen = if let Some(snapshot_file) = args.snapshot_load_file.clone() {
        let f = BufReader::new(File::open(snapshot_file).context("Could not open snapshot file")?);
        let snapshot: Snapshot =
            serde_json::de::from_reader(f).context("Could not parse snapshot file")?;
        snapshot.specimen.into_iter().map(|s| s.object).collect()
    } else {
        seed_positions()
    };

    let mut alg = GeneticAlgorithm::with_specimen(specimen, args.generation_size, alg);

    let mut log_writer = args.out_file.create().unwrap();

    loop {
        if args
            .generation_limit
            .map_or(false, |limit| alg.generation() >= limit)
        {
            break;
        }

        alg.step_generation();

        // TODO: Save interval
        {
            let mut output = BufWriter::new(
                File::create(&args.snapshot_save_file)
                    .context("Could not create/open output file")?,
            );
            writeln!(
                output,
                "{}",
                serde_json::ser::to_string(&Snapshot {
                    specimen: alg.specimen().to_vec()
                })
                .unwrap()
            )
            .unwrap();
        }

        let best = alg.highest_score();
        let best_cf = best
            .object
            .canonical_form(&alg.algorithm().transposition_table);
        let best_temp = best_cf.temperature();

        {
            let log = Log::Generation {
                generation: alg.generation(),
                top_score: best.score,
                temperature: best_temp,
            };
            writeln!(log_writer, "{}", serde_json::ser::to_string(&log).unwrap()).unwrap();
            log_writer.flush().unwrap();
        }

        {
            let log = Log::HighFitness {
                position: best.clone(),
                canonical_form: best_cf.to_string(),
                temperature: best_temp,
                degree: best.object.degree(),
            };
            writeln!(log_writer, "{}", serde_json::ser::to_string(&log).unwrap()).unwrap();
            log_writer.flush().unwrap();
        }
    }

    Ok(())
}
