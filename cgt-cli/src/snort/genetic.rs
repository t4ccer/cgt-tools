use crate::snort::common::{Log, Scored};
use anyhow::Result;
use cgt::{graph::undirected, rational::Rational, snort, transposition_table::TranspositionTable};
use clap::{self, Parser};
use rand::{seq::SliceRandom, Rng};
use std::{
    cmp::min,
    collections::HashSet,
    fs::File,
    io::{stderr, BufReader, BufWriter, Write},
    path::Path,
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
    snapshot_load_file: Option<String>,

    #[arg(long)]
    snapshot_save_file: String,

    #[arg(long)]
    out_file: String,

    #[arg(long, default_value_t = 50)]
    cleanup_interval: usize,

    #[arg(long, default_value_t = Rational::from(0))]
    save_eq_or_above: Rational,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Snapshot {
    specimen: Vec<snort::Position>,
}

struct Alg {
    args: Args,
    specimen: Vec<Scored>,
    cache: TranspositionTable<snort::Position>,
    all_time_best: HashSet<Scored>,
    log_writer: BufWriter<Box<dyn Write>>,
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
    if position.vertices().is_empty() || degree_sum == 0 || !position.graph.is_connected() {
        return Rational::NegativeInfinity;
    }

    temp_dif(position, cache)
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

fn seed_positions() -> Vec<snort::Position> {
    // 0   5   6     11
    //  \   \ /     /
    // 1--3--4--10-12
    //  /   /|\     \
    // 2   7 8 9     13
    let g1 = undirected::Graph::from_edges(
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
    );
    let pos1 = snort::Position::new(g1);

    //         9
    //         |
    // 0  5   7-10
    //  \  \ / \11
    // 1-3--4
    //  /  / \ /12
    // 2  6   8 -13
    //        |
    //        14
    let g2 = undirected::Graph::from_edges(
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
    );
    let pos2 = snort::Position::new(g2);
    vec![pos1, pos2]
}

impl Alg {
    fn new_random(args: Args) -> Alg {
        let mut specimen = Vec::with_capacity(args.generation_size);

        // TODO: Add --no-seed flag to omit this
        specimen.extend(seed_positions().into_iter().map(Scored::without_score));

        for _ in specimen.len()..args.generation_size {
            let scored = Scored {
                position: random_position(args.max_graph_vertices),
                score: Rational::NegativeInfinity,
            };
            specimen.push(scored);
        }
        Alg::with_specimen(args, specimen)
    }

    fn from_snapshot(args: Args, snapshot: Snapshot) -> Alg {
        let specimen = snapshot
            .specimen
            .into_iter()
            .map(|position| Scored {
                position,
                score: Rational::NegativeInfinity,
            })
            .collect();
        Alg::with_specimen(args, specimen)
    }

    fn with_specimen(args: Args, specimen: Vec<Scored>) -> Alg {
        let file = if Path::new(&args.out_file).exists() {
            File::open(&args.out_file)
        } else {
            File::create(&args.out_file)
        }
        .unwrap();
        let log_writer: BufWriter<Box<dyn Write>> = BufWriter::new(Box::new(file));

        Alg {
            args,
            specimen,
            cache: TranspositionTable::new(),
            all_time_best: HashSet::new(),
            log_writer,
        }
    }

    // TODO: parallel with rayon
    fn score(&mut self) {
        let specimen = &mut self.specimen;
        for mut spec in specimen {
            spec.score = score(&spec.position, &self.cache);
            if spec.score >= self.args.save_eq_or_above {
                if self.all_time_best.insert(spec.clone()) {
                    let canonical_form = spec.position.canonical_form(&self.cache);
                    let log = Log::HighFitness {
                        position: spec.clone(),
                        canonical_form: self
                            .cache
                            .game_backend()
                            .print_game_to_str(&canonical_form),
                        temperature: self.cache.game_backend().temperature(&canonical_form),
                        degree: spec.position.graph.degree(),
                    };
                    drop(spec);
                    Alg::emit_log(&mut self.log_writer, &log);
                }
            }
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

    fn emit_log(writer: &mut BufWriter<Box<dyn Write>>, log: &Log) {
        writeln!(writer, "{}", serde_json::ser::to_string(log).unwrap()).unwrap();
        writer.flush().unwrap();
    }
}

pub fn run(args: Args) -> Result<()> {
    let generation_limit = args.generation_limit;
    let output_file_path = args.snapshot_save_file.clone();

    let mut alg = if let Some(snapshot_file) = args.snapshot_load_file.clone() {
        let f = BufReader::new(File::open(snapshot_file).unwrap());
        let snapshot: Snapshot = serde_json::de::from_reader(f).unwrap();
        Alg::from_snapshot(args, snapshot)
    } else {
        Alg::new_random(args)
    };

    let mut generation = 0;

    loop {
        if generation_limit.map_or(false, |limit| generation >= limit) {
            break;
        }

        alg.score();

        let mut output = BufWriter::new(File::create(&output_file_path).unwrap());
        alg.save_progress(&mut output);

        let top = &alg.specimen.last().unwrap().position;
        let top_score = alg.highest_score();
        Alg::emit_log(
            &mut BufWriter::new(Box::new(stderr())),
            &Log::Generation {
                generation,
                top_score,
                temperature: alg
                    .cache
                    .game_backend()
                    .temperature(&top.canonical_form(&alg.cache)),
            },
        );
        alg.cross();

        generation += 1;

        if generation % alg.args.cleanup_interval == 0 {
            alg.cache = TranspositionTable::new();
        }
    }

    Ok(())
}
