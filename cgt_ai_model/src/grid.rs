//! Residual convolutional network for games played on a grid, see
//! [`Input::Grid`](cgt_ai_core::ruleset::Input::Grid).

use burn::{
    module::Module,
    nn::{
        BatchNorm, BatchNormConfig, Linear, LinearConfig, PaddingConfig2d,
        conv::{Conv2d, Conv2dConfig},
    },
    tensor::{
        Tensor,
        activation::{relu, tanh},
        backend::Backend,
    },
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
}
