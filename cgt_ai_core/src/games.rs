//! The games the AI plays, so that code generic over the rules of a game can run for a game
//! picked by name, as on the command line or in a model file.

use crate::{fjords::Fjords, quelhas::Quelhas, ruleset::Ruleset};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameId {
    Quelhas,
    Fjords,
}

/// Code generic over the rules of a game, which [`GameId::with`] runs for the rules of one.
pub trait WithRules {
    type Output;

    fn run<R: Ruleset>(self, rules: R) -> Self::Output;
}

impl GameId {
    pub const ALL: [GameId; 2] = [GameId::Quelhas, GameId::Fjords];

    pub fn with<W: WithRules>(self, code: W) -> W::Output {
        match self {
            GameId::Quelhas => code.run(Quelhas),
            GameId::Fjords => code.run(Fjords),
        }
    }

    /// [`Ruleset::name`] of the game.
    pub fn name(self) -> &'static str {
        struct Name;

        impl WithRules for Name {
            type Output = &'static str;

            fn run<R: Ruleset>(self, rules: R) -> &'static str {
                rules.name()
            }
        }

        self.with(Name)
    }

    pub fn from_name(name: &str) -> Option<GameId> {
        GameId::ALL.into_iter().find(|game| game.name() == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn games_are_found_by_name() {
        for game in GameId::ALL {
            assert_eq!(GameId::from_name(game.name()), Some(game));
        }
        assert_eq!(GameId::from_name("chess"), None);
    }
}
