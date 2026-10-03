//! Policy and value networks for games, of the kind each game's
//! [`Input`](cgt_ai_core::ruleset::Input) asks for.

mod graph;
mod grid;

pub use graph::{GraphConv, GraphNet, GraphNetConfig, GraphNetRecord};
pub use grid::{GridNet, GridNetConfig, GridNetRecord, ResidualBlock};

use burn::{
    module::Module,
    tensor::{Bytes, Tensor, TensorData, backend::Backend},
};
use burn_store::{BurnpackStore, ModuleSnapshot};
use cgt_ai_core::{
    mcts::{Evaluations, Evaluator},
    model_file,
    openings::OpeningTable,
    ruleset::{Input, Ruleset},
};
use serde::{Deserialize, Serialize};

/// The network of a game, which [`NetConfig::for_ruleset`] picks by the input of the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetConfig {
    Grid(GridNetConfig),
    Graph(GraphNetConfig),
}

impl NetConfig {
    /// The network for `rules`, `num_blocks` layers deep and `channels` wide.
    pub fn for_ruleset<R: Ruleset>(rules: &R, channels: usize, num_blocks: usize) -> NetConfig {
        match rules.input() {
            Input::Grid {
                planes,
                height,
                width,
                policy_planes,
            } => NetConfig::Grid(GridNetConfig {
                planes,
                height,
                width,
                policy_planes,
                channels,
                num_blocks,
                head_channels: 32,
            }),
            Input::Graph { nodes, features } => NetConfig::Graph(GraphNetConfig {
                nodes,
                features,
                channels,
                num_blocks,
                head_channels: channels / 2,
            }),
        }
    }

    pub const fn channels(&self) -> usize {
        match self {
            NetConfig::Grid(config) => config.channels,
            NetConfig::Graph(config) => config.channels,
        }
    }

    pub const fn num_blocks(&self) -> usize {
        match self {
            NetConfig::Grid(config) => config.num_blocks,
            NetConfig::Graph(config) => config.num_blocks,
        }
    }

    pub const fn head_channels(&self) -> usize {
        match self {
            NetConfig::Grid(config) => config.head_channels,
            NetConfig::Graph(config) => config.head_channels,
        }
    }

    #[must_use]
    pub const fn with_head_channels(self, head_channels: usize) -> NetConfig {
        match self {
            NetConfig::Grid(config) => NetConfig::Grid(GridNetConfig {
                head_channels,
                ..config
            }),
            NetConfig::Graph(config) => NetConfig::Graph(GraphNetConfig {
                head_channels,
                ..config
            }),
        }
    }

    pub fn init<B: Backend>(&self, device: &B::Device) -> Net<B> {
        match self {
            NetConfig::Grid(config) => Net::Grid(config.init(device)),
            NetConfig::Graph(config) => Net::Graph(config.init(device)),
        }
    }
}

// A program holds a network or two, so boxing the larger one would only add indirection
#[allow(clippy::large_enum_variant)]
#[derive(Module, Debug)]
pub enum Net<B: Backend> {
    Grid(GridNet<B>),
    Graph(GraphNet<B>),
}

/// A batch of positions encoded for a [`Net`].
#[derive(Debug, Clone)]
pub enum NetInput<B: Backend> {
    Grid(Tensor<B, 4>),
    Graph {
        features: Tensor<B, 3>,
        adjacency: Tensor<B, 3>,
    },
}

impl<B: Backend> Net<B> {
    /// Policy logits `[batch, num_actions]` and values `[batch]` in `[-1, 1]`.
    ///
    /// # Panics
    ///
    /// When `input` is encoded for the other kind of network.
    pub fn forward(&self, input: NetInput<B>) -> (Tensor<B, 2>, Tensor<B, 1>) {
        match (self, input) {
            (Net::Grid(net), NetInput::Grid(x)) => net.forward(x),
            (
                Net::Graph(net),
                NetInput::Graph {
                    features,
                    adjacency,
                },
            ) => net.forward(features, &adjacency),
            _ => panic!("the input is encoded for another kind of network"),
        }
    }

    /// Loads weights saved in the Burnpack format.
    ///
    /// # Errors
    ///
    /// When the bytes are not a Burnpack file or do not hold exactly the weights of this network.
    pub fn load_bytes(mut self, bytes: Vec<u8>) -> Result<Net<B>, String> {
        let mut store = BurnpackStore::from_bytes(Some(Bytes::from_bytes_vec(bytes)));
        let result = self.load_from(&mut store).map_err(|e| e.to_string())?;
        if !result.missing.is_empty() || !result.unused.is_empty() || !result.errors.is_empty() {
            return Err(format!(
                "missing {:?}, unused {:?}, errors {:?}",
                result.missing, result.unused, result.errors
            ));
        }
        Ok(self)
    }
}

/// Encodes `states` as the input of the network of `rules`.
pub fn encode_batch<R: Ruleset, B: Backend>(
    rules: &R,
    states: &[R::State],
    device: &B::Device,
) -> NetInput<B> {
    let input = rules.input();
    let len = input.encoding_len();
    let batch = states.len();
    match input {
        Input::Grid {
            planes,
            height,
            width,
            ..
        } => {
            let mut encoded = vec![0.0; batch * len];
            for (state, out) in states.iter().zip(encoded.chunks_mut(len)) {
                rules.encode(state, out);
            }
            NetInput::Grid(Tensor::from_data(
                TensorData::new(encoded, [batch, planes, height, width]),
                device,
            ))
        }
        Input::Graph { nodes, features } => {
            let (mut x, mut adjacency) = (
                Vec::with_capacity(batch * nodes * features),
                Vec::with_capacity(batch * nodes * nodes),
            );
            let mut position = vec![0.0; len];
            for state in states {
                rules.encode(state, &mut position);
                let (f, a) = position.split_at(nodes * features);
                x.extend_from_slice(f);
                adjacency.extend_from_slice(a);
            }
            NetInput::Graph {
                features: Tensor::from_data(TensorData::new(x, [batch, nodes, features]), device),
                adjacency: Tensor::from_data(
                    TensorData::new(adjacency, [batch, nodes, nodes]),
                    device,
                ),
            }
        }
    }
}

#[derive(Debug)]
pub struct NetEvaluator<R: Ruleset, B: Backend> {
    pub rules: R,
    pub net: Net<B>,
    pub device: B::Device,
}

impl<R: Ruleset, B: Backend> Evaluator<R> for NetEvaluator<R, B> {
    fn evaluate(&mut self, states: &[R::State]) -> Evaluations {
        let x = encode_batch(&self.rules, states, &self.device);
        let (logits, values) = self.net.forward(x);
        Evaluations::new(
            logits
                .into_data()
                .into_vec()
                .expect("network outputs are floats"),
            values
                .into_data()
                .into_vec()
                .expect("network outputs are floats"),
            self.rules.num_actions(),
        )
    }
}

/// What a model file says about the network it holds and how to play with it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelHeader {
    /// [`Ruleset::name`] of the game the network plays.
    pub game: String,
    pub network: NetConfig,
    /// For a game played with the pie rule, the values of the first moves.
    pub openings: Option<OpeningTable>,
}

/// A trained network in a single file, with everything needed to play with it, so that one
/// download stands for one player.
#[derive(Debug, Clone)]
pub struct ModelFile {
    pub header: ModelHeader,
    /// The weights of the network in the Burnpack format, see [`Net::load_bytes`].
    pub weights: Vec<u8>,
}

impl ModelFile {
    pub fn to_bytes(&self) -> Vec<u8> {
        let header = serde_json::to_vec(&self.header).expect("the header serializes to JSON");
        model_file::join(&header, &self.weights)
    }

    /// # Errors
    ///
    /// When `bytes` are not a model file.
    pub fn from_bytes(bytes: &[u8]) -> Result<ModelFile, String> {
        let (header, weights) = model_file::split(bytes).ok_or("not a cgt AI model file")?;
        Ok(ModelFile {
            header: serde_json::from_slice(header).map_err(|e| format!("bad model header: {e}"))?,
            weights: weights.to_vec(),
        })
    }

    /// The network with the weights of this file.
    ///
    /// # Errors
    ///
    /// When the weights do not fit the network the header describes.
    pub fn load_net<B: Backend>(self, device: &B::Device) -> Result<Net<B>, String> {
        self.header
            .network
            .init(device)
            .load_bytes(self.weights)
            .map_err(|e| format!("bad weights: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use burn::backend::{Flex, flex::FlexDevice};
    use cgt_ai_core::{
        fjords::{self, Fjords},
        quelhas::{Board, Quelhas, State},
        ruleset::Player,
    };

    fn evaluate<R: Ruleset>(rules: R, states: &[R::State]) {
        let device = FlexDevice;
        let net = NetConfig::for_ruleset(&rules, 16, 2).init::<Flex>(&device);
        let num_actions = rules.num_actions();
        let mut evaluator = NetEvaluator { rules, net, device };
        let evals = evaluator.evaluate(states);
        assert_eq!(evals.all_logits().len(), states.len() * num_actions);
        assert!(evals.values().iter().all(|v| v.abs() <= 1.0));
    }

    #[test]
    fn output_shapes() {
        evaluate(
            Quelhas,
            &[
                State::initial(),
                State {
                    empty: Board::from_bits(0b1011),
                    turn: Player::Right,
                },
            ],
        );
        evaluate(
            Fjords,
            &[
                fjords::State::deal(1, fjords::EDGE_PROBABILITY),
                fjords::State::deal(2, 0.5),
            ],
        );
    }

    fn roundtrip<R: Ruleset>(rules: &R, state: R::State, openings: Option<OpeningTable>) {
        let device = FlexDevice;
        let config = NetConfig::for_ruleset(rules, 8, 1);
        let net = config.init::<Flex>(&device);
        let mut store = BurnpackStore::from_bytes(None);
        net.save_into(&mut store).unwrap();
        let file = ModelFile {
            header: ModelHeader {
                game: rules.name().to_owned(),
                network: config,
                openings,
            },
            weights: store.get_bytes().unwrap().to_vec(),
        };

        let read = ModelFile::from_bytes(&file.to_bytes()).unwrap();
        assert_eq!(read.header.network, config);
        let loaded = read.load_net::<Flex>(&device).unwrap();
        let x = encode_batch::<_, Flex>(rules, &[state], &device);
        let (expected, _) = net.forward(x.clone());
        let (actual, _) = loaded.forward(x);
        expected.into_data().assert_eq(&actual.into_data(), true);

        assert!(ModelFile::from_bytes(b"cgt-ai").is_err());
        assert!(ModelFile::from_bytes(&file.weights).is_err());
    }

    #[test]
    fn weights_must_fit_the_header() {
        let device = FlexDevice;
        let mut store = BurnpackStore::from_bytes(None);
        NetConfig::for_ruleset(&Quelhas, 8, 2)
            .init::<Flex>(&device)
            .save_into(&mut store)
            .unwrap();
        let file = ModelFile {
            header: ModelHeader {
                game: Quelhas.name().to_owned(),
                network: NetConfig::for_ruleset(&Quelhas, 8, 1),
                openings: None,
            },
            weights: store.get_bytes().unwrap().to_vec(),
        };
        assert!(file.load_net::<Flex>(&device).is_err());
    }

    #[test]
    fn model_files_roundtrip() {
        roundtrip(
            &Quelhas,
            State::initial(),
            Some(OpeningTable {
                game: Quelhas.name().to_owned(),
                checkpoint: "test".to_owned(),
                simulations: 1,
                values: [(3, 0.25)].into(),
            }),
        );
        roundtrip(
            &Fjords,
            fjords::State::deal(3, fjords::EDGE_PROBABILITY),
            None,
        );
    }
}
