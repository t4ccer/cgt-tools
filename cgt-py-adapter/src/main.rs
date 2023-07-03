use std::io::stdin;

use cgt::{domineering, graph::undirected, snort, transposition_table::TranspositionTable};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct DomineeringRequest {
    grid: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct SnortRequest {
    vertices: Vec<snort::VertexColor>,
    adjacency_matrix: Vec<bool>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
enum Request {
    Domineering(DomineeringRequest),
    Snort(SnortRequest),
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Response {
    canonical_form: String,
    temperature: String,
}

fn main() -> ! {
    loop {
        let mut line = String::new();
        match stdin().read_line(&mut line) {
            Err(err) => println!("Error: {err}"),
            Ok(_) => {
                let response = process_line(&line);
                match response {
                    Ok(response) => println!("{}", serde_json::ser::to_string(&response).unwrap()),
                    Err(err) => println!("{err:?}"),
                }
            }
        };
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
enum ProcessingError {
    Decoding,
    Parsing,
}

fn process_line(line: &str) -> Result<Response, ProcessingError> {
    let request: Request =
        serde_json::de::from_str(&line).map_err(|_| ProcessingError::Decoding)?;
    match request {
        Request::Domineering(request) => {
            let position = domineering::Position::parse(&request.grid)
                .map_err(|_| ProcessingError::Parsing)?;
            let cache = TranspositionTable::new();
            let game = position.canonical_form(&cache);
            let canonical_form = cache.game_backend().print_game_to_str(&game);
            let temperature = cache.game_backend().temperature(&game).to_string();
            Ok(Response {
                canonical_form,
                temperature,
            })
        }
        Request::Snort(request) => {
            let size = request.vertices.len();
            let graph = undirected::Graph::from_vec(size, request.adjacency_matrix)
                .ok_or(ProcessingError::Parsing)?;
            let position = snort::Position::with_colors(request.vertices, graph)
                .ok_or(ProcessingError::Decoding)?;
            let cache = TranspositionTable::new();
            let game = position.canonical_form(&cache);
            let canonical_form = cache.game_backend().print_game_to_str(&game);
            let temperature = cache.game_backend().temperature(&game).to_string();
            Ok(Response {
                canonical_form,
                temperature,
            })
        }
    }
}
