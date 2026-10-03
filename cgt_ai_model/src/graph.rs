//! Graph convolutional network for games played on a graph, see
//! [`Input::Graph`](cgt_ai_core::ruleset::Input::Graph).
//!
//! The fields are named like those of the `PyTorch` network of `haaland3000`, so that its
//! checkpoints can be imported.

use burn::{
    module::Module,
    nn::{Linear, LinearConfig},
    tensor::{
        Tensor,
        activation::{relu, tanh},
        backend::Backend,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphNetConfig {
    pub nodes: usize,
    pub features: usize,
    pub channels: usize,
    pub num_blocks: usize,
    pub head_channels: usize,
}

/// Mixes the features of each vertex with those of its neighbours, by the normalized adjacency
/// matrix of the graph.
#[derive(Module, Debug)]
pub struct GraphConv<B: Backend> {
    linear: Linear<B>,
}

impl<B: Backend> GraphConv<B> {
    fn forward(&self, x: Tensor<B, 3>, adjacency: Tensor<B, 3>) -> Tensor<B, 3> {
        adjacency.matmul(self.linear.forward(x))
    }
}

#[derive(Module, Debug)]
pub struct GraphNet<B: Backend> {
    conv_layers: Vec<GraphConv<B>>,
    policy_head: Linear<B>,
    value_fc1: Linear<B>,
    value_fc2: Linear<B>,
}

impl GraphNetConfig {
    pub fn init<B: Backend>(&self, device: &B::Device) -> GraphNet<B> {
        let c = self.channels;
        GraphNet {
            conv_layers: (0..self.num_blocks)
                .map(|i| GraphConv {
                    linear: LinearConfig::new(if i == 0 { self.features } else { c }, c)
                        .init(device),
                })
                .collect(),
            policy_head: LinearConfig::new(c, 1).init(device),
            value_fc1: LinearConfig::new(c, self.head_channels).init(device),
            value_fc2: LinearConfig::new(self.head_channels, 1).init(device),
        }
    }
}

impl<B: Backend> GraphNet<B> {
    /// Policy logits `[batch, nodes]` and values `[batch]` in `[-1, 1]` for vertex features
    /// `[batch, nodes, features]` and normalized adjacency matrices `[batch, nodes, nodes]`.
    pub fn forward(
        &self,
        features: Tensor<B, 3>,
        adjacency: &Tensor<B, 3>,
    ) -> (Tensor<B, 2>, Tensor<B, 1>) {
        let [batch, nodes, _] = features.dims();
        let mut h = features;
        for conv in &self.conv_layers {
            h = relu(conv.forward(h, adjacency.clone()));
        }
        let policy_logits = self.policy_head.forward(h.clone()).reshape([batch, nodes]);
        let channels = h.dims()[2];
        let pooled = h.mean_dim(1).reshape([batch, channels]);
        let value = relu(self.value_fc1.forward(pooled));
        let value = tanh(self.value_fc2.forward(value)).reshape([batch]);
        (policy_logits, value)
    }
}
