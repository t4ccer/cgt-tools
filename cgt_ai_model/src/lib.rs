//! Residual policy and value network for games played on a board.

use burn::{
    config::Config,
    module::Module,
    nn::{
        BatchNorm, BatchNormConfig, Linear, LinearConfig, PaddingConfig2d,
        conv::{Conv2d, Conv2dConfig},
    },
    tensor::{
        Bytes, Tensor, TensorData,
        activation::{relu, tanh},
        backend::Backend,
    },
};
use burn_store::{BurnpackStore, ModuleSnapshot};
use cgt_ai_core::{
    mcts::{Evaluations, Evaluator},
    model_file,
    openings::OpeningTable,
    ruleset::Ruleset,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridNetConfig {
    pub planes: usize,
    pub height: usize,
    pub width: usize,
    pub policy_planes: usize,
    pub channels: usize,
    pub num_blocks: usize,
    pub head_channels: usize,
}

impl Config for GridNetConfig {}

fn conv(channels_in: usize, channels_out: usize, kernel: usize, bias: bool) -> Conv2dConfig {
    let pad = kernel / 2;
    Conv2dConfig::new([channels_in, channels_out], [kernel, kernel])
        .with_padding(PaddingConfig2d::Explicit(pad, pad, pad, pad))
        .with_bias(bias)
}

fn global_pool<B: Backend>(h: Tensor<B, 4>) -> Tensor<B, 2> {
    let [batch, channels, height, width] = h.dims();
    let flat = h.reshape([batch, channels, height * width]);
    let mean = flat.clone().mean_dim(2).reshape([batch, channels]);
    let max = flat.max_dim(2).reshape([batch, channels]);
    Tensor::cat(vec![mean, max], 1)
}

#[derive(Module, Debug)]
pub struct ResidualBlock<B: Backend> {
    conv1: Conv2d<B>,
    bn1: BatchNorm<B>,
    conv2: Conv2d<B>,
    bn2: BatchNorm<B>,
    // Misère outcomes hinge on board-wide parity and move counts, which
    // local convolutions can only see after many layers.
    pool_fc: Option<Linear<B>>,
}

impl<B: Backend> ResidualBlock<B> {
    fn new(channels: usize, use_global_pool: bool, device: &B::Device) -> ResidualBlock<B> {
        ResidualBlock {
            conv1: conv(channels, channels, 3, false).init(device),
            bn1: BatchNormConfig::new(channels).init(device),
            conv2: conv(channels, channels, 3, false).init(device),
            bn2: BatchNormConfig::new(channels).init(device),
            pool_fc: use_global_pool
                .then(|| LinearConfig::new(2 * channels, channels).init(device)),
        }
    }

    fn forward(&self, x: Tensor<B, 4>) -> Tensor<B, 4> {
        let mut h = relu(self.bn1.forward(self.conv1.forward(x.clone())));
        if let Some(fc) = &self.pool_fc {
            let [batch, channels, _, _] = h.dims();
            h = h.clone() + fc.forward(global_pool(h)).reshape([batch, channels, 1, 1]);
        }
        let h = self.bn2.forward(self.conv2.forward(h));
        relu(x + h)
    }
}

#[derive(Module, Debug)]
pub struct GridNet<B: Backend> {
    stem_conv: Conv2d<B>,
    stem_bn: BatchNorm<B>,
    blocks: Vec<ResidualBlock<B>>,
    policy_conv: Conv2d<B>,
    policy_bn: BatchNorm<B>,
    policy_out: Conv2d<B>,
    value_conv: Conv2d<B>,
    value_bn: BatchNorm<B>,
    value_fc1: Linear<B>,
    value_fc2: Linear<B>,
}

impl GridNetConfig {
    pub fn for_ruleset<R: Ruleset>(rules: &R, channels: usize, num_blocks: usize) -> GridNetConfig {
        let shape = rules.input_shape();
        GridNetConfig {
            planes: shape.planes,
            height: shape.height,
            width: shape.width,
            policy_planes: rules.policy_planes(),
            channels,
            num_blocks,
            head_channels: 32,
        }
    }

    pub fn init<B: Backend>(&self, device: &B::Device) -> GridNet<B> {
        let (c, hc) = (self.channels, self.head_channels);
        GridNet {
            stem_conv: conv(self.planes, c, 3, false).init(device),
            stem_bn: BatchNormConfig::new(c).init(device),
            blocks: (0..self.num_blocks)
                .map(|i| ResidualBlock::new(c, i % 2 == 1, device))
                .collect(),
            policy_conv: conv(c, hc, 1, false).init(device),
            policy_bn: BatchNormConfig::new(hc).init(device),
            policy_out: conv(hc, self.policy_planes, 1, true).init(device),
            value_conv: conv(c, hc, 1, false).init(device),
            value_bn: BatchNormConfig::new(hc).init(device),
            value_fc1: LinearConfig::new(2 * hc, 128).init(device),
            value_fc2: LinearConfig::new(128, 1).init(device),
        }
    }
}

impl<B: Backend> GridNet<B> {
    /// Policy logits `[batch, policy_planes * height * width]` and values `[batch]` in
    /// `[-1, 1]`.
    pub fn forward(&self, x: Tensor<B, 4>) -> (Tensor<B, 2>, Tensor<B, 1>) {
        let batch = x.dims()[0];
        let mut h = relu(self.stem_bn.forward(self.stem_conv.forward(x)));
        for block in &self.blocks {
            h = block.forward(h);
        }
        let policy = relu(self.policy_bn.forward(self.policy_conv.forward(h.clone())));
        let policy_logits = self.policy_out.forward(policy).flatten(1, 3);
        let value = relu(self.value_bn.forward(self.value_conv.forward(h)));
        let value = relu(self.value_fc1.forward(global_pool(value)));
        let value = tanh(self.value_fc2.forward(value)).reshape([batch]);
        (policy_logits, value)
    }

    /// Loads weights saved in the Burnpack format.
    ///
    /// # Errors
    ///
    /// When the bytes are not a Burnpack file or do not hold every weight of this network.
    pub fn load_bytes(mut self, bytes: Vec<u8>) -> Result<GridNet<B>, String> {
        let mut store = BurnpackStore::from_bytes(Some(Bytes::from_bytes_vec(bytes)));
        let result = self.load_from(&mut store).map_err(|e| e.to_string())?;
        if !result.missing.is_empty() || !result.errors.is_empty() {
            return Err(format!(
                "missing {:?}, errors {:?}",
                result.missing, result.errors
            ));
        }
        Ok(self)
    }
}

pub fn encode_batch<R: Ruleset, B: Backend>(
    rules: &R,
    states: &[R::State],
    device: &B::Device,
) -> Tensor<B, 4> {
    let shape = rules.input_shape();
    let len = shape.features_len();
    let mut x = vec![0.0; states.len() * len];
    for (state, out) in states.iter().zip(x.chunks_mut(len)) {
        rules.encode(state, out);
    }
    Tensor::from_data(
        TensorData::new(x, [states.len(), shape.planes, shape.height, shape.width]),
        device,
    )
}

#[derive(Debug)]
pub struct NetEvaluator<R: Ruleset, B: Backend> {
    pub rules: R,
    pub net: GridNet<B>,
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
    pub network: GridNetConfig,
    /// For a game played with the pie rule, the values of the first moves.
    pub openings: Option<OpeningTable>,
}

/// A trained network in a single file, with everything needed to play with it, so that one
/// download stands for one player.
#[derive(Debug, Clone)]
pub struct ModelFile {
    pub header: ModelHeader,
    /// The weights of the network in the Burnpack format, see [`GridNet::load_bytes`].
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
    pub fn load_net<B: Backend>(self, device: &B::Device) -> Result<GridNet<B>, String> {
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
        quelhas::{Board, Quelhas, State},
        ruleset::Player,
    };

    #[test]
    fn output_shapes() {
        let device = FlexDevice;
        let net = GridNetConfig::for_ruleset(&Quelhas, 16, 2).init::<Flex>(&device);
        let states = [
            State::initial(),
            State {
                empty: Board::from_bits(0b1011),
                turn: Player::Right,
            },
        ];
        let mut evaluator = NetEvaluator {
            rules: Quelhas,
            net,
            device,
        };
        let evals = evaluator.evaluate(&states);
        assert_eq!(evals.all_logits().len(), 2 * Quelhas.num_actions());
        assert!(evals.values().iter().all(|v| v.abs() <= 1.0));
    }

    #[test]
    fn model_file_roundtrip() {
        let device = FlexDevice;
        let config = GridNetConfig::for_ruleset(&Quelhas, 8, 1);
        let net = config.init::<Flex>(&device);
        let mut store = BurnpackStore::from_bytes(None);
        net.save_into(&mut store).unwrap();
        let file = ModelFile {
            header: ModelHeader {
                game: Quelhas.name().to_owned(),
                network: config,
                openings: Some(OpeningTable {
                    game: Quelhas.name().to_owned(),
                    checkpoint: "test".to_owned(),
                    simulations: 1,
                    values: [(3, 0.25)].into(),
                }),
            },
            weights: store.get_bytes().unwrap().to_vec(),
        };

        let read = ModelFile::from_bytes(&file.to_bytes()).unwrap();
        assert_eq!(read.header.network, config);
        assert_eq!(read.header.openings.as_ref().unwrap().value(3), Some(0.25));
        let loaded = read.load_net::<Flex>(&device).unwrap();
        let x = encode_batch::<_, Flex>(&Quelhas, &[State::initial()], &device);
        let (expected, _) = net.forward(x.clone());
        let (actual, _) = loaded.forward(x);
        expected.into_data().assert_eq(&actual.into_data(), true);

        assert!(ModelFile::from_bytes(b"cgt-ai").is_err());
        assert!(ModelFile::from_bytes(&file.weights).is_err());
    }
}
