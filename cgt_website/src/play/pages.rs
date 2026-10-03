use super::{fjords::FjordsGame, quelhas::QuelhasGame};
use crate::{config::Source, log};
use cgt_ai_core::{games::GameId, model_file};
use leptos::prelude::*;
use serde::Deserialize;
use std::{fs, io};

pub const PLAY_URL: &str = "/play/";

/// A game the site has a page for
pub struct GamePage {
    pub game: GameId,
    pub title: &'static str,
    /// What the list of games says about it
    pub summary: &'static str,
    /// What its page says first
    lead: &'static str,
    /// The rules, a paragraph each
    rules: &'static [&'static str],
    /// How to make a move
    moving: &'static str,
    /// The game to play, against the AIs given by their names and the addresses of their model
    /// files
    island: fn(Vec<(String, String)>) -> AnyView,
}

/// Every game the site can have a page for, in the order the list of games shows them
pub const GAMES: &[GamePage] = &[
    GamePage {
        game: GameId::Quelhas,
        title: "Quelhas",
        summary: "Domineering where a move crosses out a whole line of squares, the last move loses, and the second player may swap sides after the first move.",
        lead: "Cross out lines of empty squares, Left vertically and Right horizontally. Whoever makes the last move loses.",
        rules: &[
            "Quelhas (“narrow tracks”), by Carlos P. Santos and Alfie Davies, is played on a 10×10 grid. Two players, Left and Right, take turns crossing out two or more consecutive empty squares in a straight line. As in Domineering, Left crosses out vertical lines and Right horizontal ones. Left moves first, and the player who makes the last move loses.",
            "The game is played with the pie rule: after the first move, the second player may swap sides, and then plays on as Left while the first player continues as Right. So the first player looks for a move that leaves neither side ahead.",
        ],
        moving: "Click the square a line starts at, and then the square it ends at.",
        island: |models| view! { <QuelhasGame models /> }.into_any(),
    },
    GamePage {
        game: GameId::Fjords,
        title: "Fjords",
        summary: "Claim vertices joined to your own on a board dealt at random, until one player has nowhere left to go.",
        lead: "Claim vertices joined to your own. Whoever has nowhere left to go loses.",
        rules: &[
            "Fjords is played on a board of 64 vertices in offset rows, where most neighbouring vertices are joined by an edge. Blue and Red each start with three vertices, and Blue moves first. In turn, each player claims an empty vertex joined by an edge to one of their own. A player who cannot claim a vertex on their turn loses.",
            "Once no region of empty vertices can be reached by both players, the game is settled: each player has as many moves left as there are vertices in their regions. So the player with more of them wins, and with as many, the player to move runs out first. The page ends settled games unless you choose to play them out.",
        ],
        moving: "Click a vertex to claim it. A seed deals the board, so the same seed gives the same board, which the address of the page keeps.",
        island: |models| view! { <FjordsGame models /> }.into_any(),
    },
];

impl GamePage {
    /// `Ruleset::name` of the game, which also names its page and its models in the
    /// configuration
    pub fn name(&self) -> &'static str {
        self.game.name()
    }

    pub fn url(&self) -> String {
        format!("{PLAY_URL}{}/", self.name())
    }

    /// The page of the game, offering the AIs in `models`, given by their names and the
    /// addresses of their model files
    pub fn page(&self, models: Vec<(String, String)>) -> AnyView {
        let rules = self
            .rules
            .iter()
            .map(|paragraph| view! { <p>{*paragraph}</p> })
            .collect_view();
        view! {
            <div class="container play">
                <h1>{self.title}</h1>
                <p class="lead">{self.lead}</p>
                {(self.island)(models)}
                <section class="play-rules">
                    <h2>"Rules"</h2>
                    {rules}
                    <h2>"Playing"</h2>
                    <p>
                        {self.moving}
                        " The AI is a neural network trained by self-play, which picks its moves with a Monte Carlo tree search in your browser. More simulations make it stronger, and slower."
                    </p>
                </section>
            </div>
        }
        .into_any()
    }
}

/// An AI that the site offers to play against
pub struct AiModel {
    pub name: String,
    /// Where the page loads the model file from
    pub url: String,
    /// The model file, for a model the site serves itself
    bytes: Option<Vec<u8>>,
}

impl AiModel {
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

fn check(game: &GamePage, bytes: &[u8]) -> Result<(), String> {
    let (header, _) =
        model_file::split(bytes).ok_or("not a model file, which `cgt-ai-train export` writes")?;
    let header: Header = serde_json::from_slice(header).map_err(|err| err.to_string())?;
    if header.game == game.name() {
        Ok(())
    } else {
        Err(format!(
            "a model of {}, not of {}",
            header.game,
            game.name()
        ))
    }
}

/// Reads the models of `game` that are files, which the site then serves itself. The page loads
/// the other models from their addresses
///
/// # Errors
///
/// When a model file cannot be read or is not a model of `game`
pub fn read_models(game: &GamePage, models: &[(String, Source)]) -> io::Result<Vec<AiModel>> {
    models
        .iter()
        .map(|(name, source)| match source {
            Source::Path(path) => {
                log(format!(
                    "Reading model `{name}` of {} from `{}`",
                    game.name(),
                    path.display()
                ));
                let bytes = fs::read(path)
                    .map_err(|err| err.to_string())
                    .and_then(|bytes| check(game, &bytes).map(|()| bytes))
                    .map_err(|err| {
                        io::Error::other(format!("model `{name}` at `{}`: {err}", path.display()))
                    })?;
                Ok(AiModel {
                    name: name.clone(),
                    url: format!("{}models/{name}.bin", game.url()),
                    bytes: Some(bytes),
                })
            }
            Source::Url(url) => {
                log(format!(
                    "The page loads model `{name}` of {} from {url}",
                    game.name()
                ));
                Ok(AiModel {
                    name: name.clone(),
                    url: url.clone(),
                    bytes: None,
                })
            }
        })
        .collect()
}

/// The list of `games`
pub fn index_page(games: &[&GamePage]) -> impl IntoView + use<> {
    let items = games
        .iter()
        .map(|game| {
            view! {
                <li>
                    <a href=game.url()>{game.title}</a>
                    <p>{game.summary}</p>
                </li>
            }
        })
        .collect_view();
    view! {
        <div class="container">
            <div class="docs-content">
                <h1>"Play"</h1>
                <p class="lead">
                    "Combinatorial games against AI players trained with cgt-tools, which run in your browser."
                </p>
                <ul class="guide-list">{items}</ul>
            </div>
        </div>
    }
}
