use anyhow::Result;
use burn::tensor::backend::AutodiffBackend;
use cgt_ai_core::{
    games::{GameId, WithRules},
    ruleset::Ruleset,
};
use clap::builder::{PossibleValuesParser, TypedValueParser};

pub trait GameCommand {
    fn run<B: AutodiffBackend, R: Ruleset>(self, rules: R, device: B::Device) -> Result<()>;
}

/// `name` prefixed by the name of the game, for the files the tools write, so that files of
/// different games can share a directory
pub fn file_name<R: Ruleset>(rules: &R, name: &str) -> String {
    format!("{}_{name}", rules.name())
}

/// Parses the `--game` option, which takes the name of a game
pub fn game_parser() -> impl TypedValueParser<Value = GameId> {
    PossibleValuesParser::new(GameId::ALL.map(GameId::name))
        .map(|name| GameId::from_name(&name).expect("only the names of games are accepted"))
}

struct Run<C, B: AutodiffBackend> {
    command: C,
    device: B::Device,
}

impl<C: GameCommand, B: AutodiffBackend> WithRules for Run<C, B> {
    type Output = Result<()>;

    fn run<R: Ruleset>(self, rules: R) -> Result<()> {
        self.command.run::<B, R>(rules, self.device)
    }
}

/// Runs `command` for the rules of `game`
pub fn run<B: AutodiffBackend>(
    game: GameId,
    command: impl GameCommand,
    device: B::Device,
) -> Result<()> {
    game.with(Run::<_, B> { command, device })
}
