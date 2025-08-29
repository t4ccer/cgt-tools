use crate::{commands::snort::common::Log, io::FileOrStderr};
use anyhow::{Context, Result};
use cgt::{
    genetic_algorithm::{Algorithm, GeneticAlgorithm, Scored},
    graph::{
        Graph, VertexIndex,
        adjacency_matrix::undirected::{self, UndirectedGraph},
    },
    numeric::rational::Rational,
    short::partizan::{
        games::snort::{Snort, VertexColor, VertexKind},
        partizan_game::PartizanGame,
        transposition_table::ParallelTranspositionTable,
    },
};
use clap::{self, Parser};
use rand::{Rng, seq::IndexedRandom};
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
    transposition_table: ParallelTranspositionTable<Snort<VertexKind, UndirectedGraph<VertexKind>>>,
    max_graph_vertices: usize,
    mutation_rate: f32,
}

impl SnortTemperatureDegreeDifference {
    fn mutate_with_rate(
        position: &mut Snort<VertexKind, UndirectedGraph<VertexKind>>,
        rng: &mut rand::rngs::ThreadRng,
        mutation_rate: f32,
    ) {
        // Mutate vertices
        if position.graph.size() > 1 {
            let mutation_roll: f32 = rng.random();
            if mutation_roll < mutation_rate {
                let to_remove = VertexIndex {
                    index: rng.random_range(0..position.graph.size()),
                };
                position.graph.remove_vertex(to_remove);
            }
        }
        // TODO: Check for max size
        // if position.graph.size()
        let mutation_roll: f32 = rng.random();
        if mutation_roll < mutation_rate {
            position
                .graph
                .add_vertex(VertexKind::Single(VertexColor::Empty));
            let another_vertex = VertexIndex {
                index: rng.random_range(0..position.graph.size() - 1),
            };
            position.graph.connect(
                VertexIndex {
                    index: position.graph.size() - 1,
                },
                another_vertex,
                true,
            );
        }

        // Mutate edges
        for v in position.graph.vertex_indices() {
            for u in position.graph.vertex_indices() {
                if v == u {
                    continue;
                }

                let mutation_roll: f32 = rng.random();
                if mutation_roll < mutation_rate {
                    position
                        .graph
                        .connect(v, u, !position.graph.are_adjacent(v, u));
                }
            }
        }

        // Mutate colors
        let available_colors = [
            VertexColor::Empty,
            VertexColor::TintLeft,
            VertexColor::TintRight,
        ];
        for index in position.graph.vertex_indices() {
            let mutation_roll: f32 = rng.random();
            if mutation_roll < mutation_rate {
                *position.graph.get_vertex_mut(index) =
                    VertexKind::Single(*available_colors.choose(rng).unwrap());
            }
        }
    }
}

impl Algorithm<Snort<VertexKind, UndirectedGraph<VertexKind>>, Rational>
    for SnortTemperatureDegreeDifference
{
    fn mutate(
        &self,
        position: &mut Snort<VertexKind, UndirectedGraph<VertexKind>>,
        rng: &mut rand::rngs::ThreadRng,
    ) {
        Self::mutate_with_rate(position, rng, self.mutation_rate);
    }

    fn cross(
        &self,
        lhs: &Snort<VertexKind, UndirectedGraph<VertexKind>>,
        rhs: &Snort<VertexKind, UndirectedGraph<VertexKind>>,
        _rng: &mut rand::rngs::ThreadRng,
    ) -> Snort<VertexKind, UndirectedGraph<VertexKind>> {
        let mut rng = rand::rng();

        let mut positions = [lhs, rhs];
        positions.sort_by_key(|pos| pos.graph.size());
        let [smaller, larger] = positions;

        let new_size = rng.random_range(1..=larger.graph.size());
        let mut new_graph =
            undirected::UndirectedGraph::empty(&vec![
                VertexKind::Single(VertexColor::Empty);
                new_size
            ]);

        for v in 0..(min(new_size, smaller.graph.size())) {
            for u in 0..(min(new_size, smaller.graph.size())) {
                let v = VertexIndex { index: v };
                let u = VertexIndex { index: u };
                new_graph.connect(v, u, smaller.graph.are_adjacent(v, u));
            }
        }
        for v in (min(new_size, smaller.graph.size()))..(min(new_size, larger.graph.size())) {
            for u in (min(new_size, smaller.graph.size()))..(min(new_size, larger.graph.size())) {
                let v = VertexIndex { index: v };
                let u = VertexIndex { index: u };
                new_graph.connect(v, u, larger.graph.are_adjacent(v, u));
            }
        }

        for index in 0..(min(new_size, smaller.graph.size())) {
            let index = VertexIndex { index };
            *new_graph.get_vertex_mut(index) = *smaller.graph.get_vertex(index);
        }
        for index in (min(new_size, smaller.graph.size()))..(min(new_size, larger.graph.size())) {
            let index = VertexIndex { index };
            *new_graph.get_vertex_mut(index) = *larger.graph.get_vertex(index);
        }

        Snort::new(new_graph)
    }

    fn lowest_score(&self) -> Rational {
        Rational::NegativeInfinity
    }

    fn score(&self, position: &Snort<VertexKind, UndirectedGraph<VertexKind>>) -> Rational {
        let degree_sum = position.graph.degrees().sum::<usize>();
        if position.graph.vertex_indices().next().is_none()
            || degree_sum == 0
            || !position.graph.is_connected()
        {
            return Rational::NegativeInfinity;
        }

        let game = position.canonical_form(&self.transposition_table);
        let temp = game.temperature();
        let degree = position.degree();
        temp.to_rational() - Rational::from(degree as i64)
    }

    fn random(
        &self,
        rng: &mut rand::rngs::ThreadRng,
    ) -> Snort<VertexKind, UndirectedGraph<VertexKind>> {
        let graph_size = rng.random_range(1..=self.max_graph_vertices);
        let graph = undirected::UndirectedGraph::empty(&vec![
            VertexKind::Single(VertexColor::Empty);
            graph_size
        ]);
        let mut position = Snort::new(graph);
        Self::mutate_with_rate(&mut position, rng, 1.0);
        position
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Snapshot {
    specimen: Vec<Scored<Snort<VertexKind, UndirectedGraph<VertexKind>>, Rational>>,
}

fn seed_positions() -> Vec<Snort<VertexKind, UndirectedGraph<VertexKind>>> {
    // 0   5   6     11
    //  \   \ /     /
    // 1--3--4--10-12
    //  /   /|\     \
    // 2   7 8 9     13
    let pos_1 = Snort::new(undirected::UndirectedGraph::from_edges(
        &[
            (VertexIndex { index: 0 }, VertexIndex { index: 3 }),
            (VertexIndex { index: 1 }, VertexIndex { index: 3 }),
            (VertexIndex { index: 2 }, VertexIndex { index: 3 }),
            (VertexIndex { index: 3 }, VertexIndex { index: 4 }),
            (VertexIndex { index: 4 }, VertexIndex { index: 5 }),
            (VertexIndex { index: 4 }, VertexIndex { index: 6 }),
            (VertexIndex { index: 4 }, VertexIndex { index: 7 }),
            (VertexIndex { index: 4 }, VertexIndex { index: 8 }),
            (VertexIndex { index: 4 }, VertexIndex { index: 9 }),
            (VertexIndex { index: 4 }, VertexIndex { index: 10 }),
            (VertexIndex { index: 10 }, VertexIndex { index: 11 }),
            (VertexIndex { index: 10 }, VertexIndex { index: 12 }),
            (VertexIndex { index: 10 }, VertexIndex { index: 13 }),
        ],
        &[VertexKind::Single(VertexColor::Empty); 14],
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
    let pos_2 = Snort::new(undirected::UndirectedGraph::from_edges(
        &[
            (VertexIndex { index: 0 }, VertexIndex { index: 3 }),
            (VertexIndex { index: 1 }, VertexIndex { index: 3 }),
            (VertexIndex { index: 2 }, VertexIndex { index: 3 }),
            (VertexIndex { index: 3 }, VertexIndex { index: 4 }),
            (VertexIndex { index: 4 }, VertexIndex { index: 5 }),
            (VertexIndex { index: 4 }, VertexIndex { index: 6 }),
            (VertexIndex { index: 4 }, VertexIndex { index: 7 }),
            (VertexIndex { index: 4 }, VertexIndex { index: 8 }),
            (VertexIndex { index: 7 }, VertexIndex { index: 9 }),
            (VertexIndex { index: 7 }, VertexIndex { index: 10 }),
            (VertexIndex { index: 7 }, VertexIndex { index: 11 }),
            (VertexIndex { index: 8 }, VertexIndex { index: 12 }),
            (VertexIndex { index: 8 }, VertexIndex { index: 13 }),
            (VertexIndex { index: 8 }, VertexIndex { index: 14 }),
        ],
        &[VertexKind::Single(VertexColor::Empty); 15],
    ));

    vec![pos_1, pos_2]
}

#[allow(clippy::needless_pass_by_value)]
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
            .is_some_and(|limit| alg.generation() >= limit)
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
