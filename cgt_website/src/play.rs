use crate::{config::Source, log, quelhas::QuelhasGame};
use cgt_ai_core::{model_file, quelhas::Quelhas, ruleset::Ruleset};
use leptos::prelude::*;
use serde::Deserialize;
use std::{fs, io};

pub const PLAY_URL: &str = "/play/";
const QUELHAS_URL: &str = "/play/quelhas/";
/// Where the site keeps the model files it serves itself
const MODELS_URL: &str = "/play/quelhas/models/";

/// An AI that the site offers to play against
pub struct Model {
    pub name: String,
    /// Where the page loads the model file from
    pub url: String,
    /// The model file, for a model the site serves itself
    bytes: Option<Vec<u8>>,
}

impl Model {
    /// The file the site serves for this model, as its path relative to the site root and its
    /// contents
    pub fn file(&self) -> Option<(&str, &[u8])> {
        let bytes = self.bytes.as_deref()?;
        Some((self.url.trim_start_matches('/'), bytes))
    }
}

/// The part of the header of a model file the site checks, see `cgt_ai_model::ModelHeader`
#[derive(Deserialize)]
struct Header {
    game: String,
}

fn check(bytes: &[u8]) -> Result<(), String> {
    let (header, _) =
        model_file::split(bytes).ok_or("not a model file, which `cgt-ai-train export` writes")?;
    let header: Header = serde_json::from_slice(header).map_err(|err| err.to_string())?;
    if header.game == Quelhas.name() {
        Ok(())
    } else {
        Err(format!(
            "a model of {}, not of {}",
            header.game,
            Quelhas.name()
        ))
    }
}

/// Reads the models of Quelhas that are files, which the site then serves itself. The page loads
/// the other models from their addresses
///
/// # Errors
///
/// When a model file cannot be read or is not a model of Quelhas
pub fn read_models(models: &[(String, Source)]) -> io::Result<Vec<Model>> {
    models
        .iter()
        .map(|(name, source)| match source {
            Source::Path(path) => {
                log(format!("Reading model `{name}` from `{}`", path.display()));
                let bytes = fs::read(path)
                    .map_err(|err| err.to_string())
                    .and_then(|bytes| check(&bytes).map(|()| bytes))
                    .map_err(|err| {
                        io::Error::other(format!("model `{name}` at `{}`: {err}", path.display()))
                    })?;
                Ok(Model {
                    name: name.clone(),
                    url: format!("{MODELS_URL}{name}.bin"),
                    bytes: Some(bytes),
                })
            }
            Source::Url(url) => {
                log(format!("The page loads model `{name}` from {url}"));
                Ok(Model {
                    name: name.clone(),
                    url: url.clone(),
                    bytes: None,
                })
            }
        })
        .collect()
}

pub fn index_page() -> impl IntoView {
    view! {
        <div class="container">
            <div class="docs-content">
                <h1>"Play"</h1>
                <p class="lead">
                    "Combinatorial games against AI players trained with cgt-tools, which run in your browser."
                </p>
                <ul class="guide-list">
                    <li>
                        <a href=QUELHAS_URL>"Quelhas"</a>
                        <p>
                            "Domineering where a move crosses out a whole line of squares, the last move loses, and the second player may swap sides after the first move."
                        </p>
                    </li>
                </ul>
            </div>
        </div>
    }
}

/// The Quelhas page, with `models` as their names and the addresses of their files
pub fn quelhas_page(models: Vec<(String, String)>) -> impl IntoView {
    view! {
        <div class="container play">
            <h1>"Quelhas"</h1>
            <p class="lead">
                "Cross out lines of empty squares, Left vertically and Right horizontally. Whoever makes the last move loses."
            </p>
            <QuelhasGame models />
            <section class="play-rules">
                <h2>"Rules"</h2>
                <p>
                    "Quelhas (“narrow tracks”), by Carlos P. Santos and Alfie Davies, is played on a 10×10 grid. "
                    "Two players, Left and Right, take turns crossing out two or more consecutive empty squares in a straight line. "
                    "As in Domineering, Left crosses out vertical lines and Right horizontal ones. "
                    "Left moves first, and the player who makes the last move loses."
                </p>
                <p>
                    "The game is played with the pie rule: after the first move, the second player may swap sides, and then plays on as Left while the first player continues as Right. "
                    "So the first player looks for a move that leaves neither side ahead."
                </p>
                <h2>"Playing"</h2>
                <p>
                    "Click the square a line starts at, and then the square it ends at. "
                    "The AI is a neural network trained by self-play, which picks its moves with a Monte Carlo tree search in your browser. "
                    "More simulations make it stronger, and slower."
                </p>
            </section>
        </div>
    }
}
