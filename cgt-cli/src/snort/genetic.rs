use anyhow::Result;
use cgt::{graph::undirected, rational::Rational, snort, transposition_table::TranspositionTable};
use clap::{self, Parser};
use rand::{seq::SliceRandom, Rng};
use std::{
    cmp::min,
    fs::File,
    io::{BufReader, BufWriter, Write},
};

#[derive(Parser, Debug, Clone)]
pub struct Args {
    #[arg(long)]
    generation_size: usize,

    #[arg(long)]
    max_graph_vertices: usize,

    #[arg(long)]
    mutation_rate: f32,

    #[arg(long, default_value = None)]
    generation_limit: Option<usize>,

    /// Path to saved snapshot
    #[arg(long, default_value = None)]
    load_snapshot: Option<String>,

    #[arg(long)]
    output_file: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Snapshot {
    specimen: Vec<snort::Position>,
}

#[derive(Clone)]
struct Scored {
    position: snort::Position,
    score: Rational,
}

struct Alg {
    args: Args,
    specimen: Vec<Scored>,
    cache: TranspositionTable<snort::Position>,
}

fn random_position(max_graph_vertices: usize) -> snort::Position {
    let mut rng = rand::thread_rng();
    let graph_size = rng.gen_range(1..=max_graph_vertices);
    let graph = undirected::Graph::empty(graph_size);
    let mut position = snort::Position::new(graph);
    mutate(&mut position, 1.0);
    position
}

fn mutate(position: &mut snort::Position, mutation_rate: f32) {
    let mut rng = rand::thread_rng();

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
        position.vertices.push(snort::VertexColor::Empty);
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
        snort::VertexColor::Empty,
        snort::VertexColor::TintLeft,
        snort::VertexColor::TintRight,
    ];
    for idx in 0..position.vertices.len() {
        let mutation_roll: f32 = rng.gen();
        if mutation_roll < mutation_rate {
            position.vertices[idx] = *available_colors.choose(&mut rng).unwrap();
        }
    }
}

fn score(position: &snort::Position, cache: &TranspositionTable<snort::Position>) -> Rational {
    let degree_sum = position.graph.degrees().iter().sum::<usize>();
    if degree_sum == 0 || !position.graph.is_connected() {
        return Rational::NegativeInfinity;
    }

    min(
        temp_dif(position, cache) * Rational::from(1000),
        Rational::from(0),
    ) + Rational::from((position.graph.size() + degree_sum) as i64)
}

fn temp_dif(position: &snort::Position, cache: &TranspositionTable<snort::Position>) -> Rational {
    let game = position.canonical_form(cache);
    let temp = cache.game_backend().temperature(&game);
    let degree = position.graph.degree();
    temp - Rational::from(degree as i64)
}

fn cross(lhs: &snort::Position, rhs: &snort::Position) -> snort::Position {
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

    snort::Position::with_colors(colors, new_graph).unwrap()
}

impl Alg {
    fn new_random(args: Args) -> Alg {
        let mut specimen = Vec::with_capacity(args.generation_size);
        for _ in 0..args.generation_size {
            let scored = Scored {
                position: random_position(args.max_graph_vertices),
                score: Rational::NegativeInfinity,
            };
            specimen.push(scored);
        }

        Alg {
            args,
            specimen,
            cache: TranspositionTable::new(1 << 20),
        }
    }

    fn from_snapshot(args: Args, snapshot: Snapshot) -> Alg {
        Alg {
            args,
            specimen: snapshot
                .specimen
                .into_iter()
                .map(|position| Scored {
                    position,
                    score: Rational::NegativeInfinity,
                })
                .collect(),
            cache: TranspositionTable::new(1 << 20),
        }
    }

    // TODO: parallel with rayon
    fn score(&mut self) {
        for mut spec in &mut self.specimen {
            spec.score = score(&spec.position, &self.cache);
        }
        self.specimen.sort_by_key(|spec| spec.score.clone());
    }

    fn highest_score(&self) -> Rational {
        self.specimen
            .last()
            .expect("to have at least one score")
            .score
            .clone()
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            specimen: self
                .specimen
                .iter()
                .map(|spec| &spec.position)
                .cloned()
                .collect(),
        }
    }

    fn save_progress(&self, mut output: impl Write) {
        writeln!(
            output,
            "{}",
            serde_json::ser::to_string(&self.snapshot()).unwrap()
        )
        .unwrap();
    }

    fn cross(&mut self) {
        let mut rng = rand::thread_rng();
        let mid_point = self.args.generation_size / 2;
        let mut new_specimen = Vec::with_capacity(self.args.generation_size);
        let top_half = &self.specimen[mid_point..];
        new_specimen.extend_from_slice(top_half);
        for _ in new_specimen.len()..self.args.generation_size {
            let lhs = self.specimen.choose(&mut rng).unwrap();
            let rhs = self.specimen.choose(&mut rng).unwrap();
            let mut position = cross(&lhs.position, &rhs.position);
            mutate(&mut position, self.args.mutation_rate);
            new_specimen.push(Scored {
                position,
                score: Rational::NegativeInfinity,
            });
        }
        self.specimen = new_specimen;
    }
}

pub fn run(args: Args) -> Result<()> {
    let generation_limit = args.generation_limit;
    let output_file_path = args.output_file.clone();

    let mut alg = if let Some(snapshot_file) = args.load_snapshot.clone() {
        let f = BufReader::new(File::open(snapshot_file).unwrap());
        let snapshot: Snapshot = serde_json::de::from_reader(f).unwrap();
        Alg::from_snapshot(args, snapshot)
    } else {
        Alg::new_random(args)
    };

    let mut generation = 0;

    loop {
        if let Some(generation_limit) = generation_limit {
            if generation >= generation_limit {
                break;
            }
        }

        alg.score();

        let mut output = BufWriter::new(File::create(&output_file_path).unwrap());
        alg.save_progress(&mut output);

        let top = &alg.specimen.last().unwrap().position;
        eprintln!(
            "Generation {generation}: Top score: {}, Temp: {}, Temp diff: {}",
            alg.highest_score(),
            alg.cache
                .game_backend()
                .temperature(&top.canonical_form(&alg.cache)),
            temp_dif(&top, &alg.cache)
        );
        eprintln!("{}", top.to_graphviz());

        alg.cross();

        generation += 1;
    }

    Ok(())
}
