#![allow(missing_docs)]

use crate::{graph::Graph, numeric::v2f::V2f};

pub struct SpringEmbedder {
    pub cooling_rate: f32,
    pub c_attractive: f32,
    pub c_repulsive: f32,
    pub ideal_spring_length: f32,
    pub iterations: usize,
    pub bounds: Option<(V2f, V2f)>,
}

impl SpringEmbedder {
    pub fn layout<G>(&self, graph: &G, positions: &mut [V2f])
    where
        G: Graph,
    {
        assert_eq!(graph.size(), positions.len());

        let mut forces: Vec<V2f> = vec![V2f { x: 0.0, y: 0.0 }; positions.len()];
        let mut cooling = 1.0;

        for _ in 0..self.iterations {
            for u in graph.vertices() {
                for v in graph.vertices() {
                    if u == v {
                        continue;
                    }
                    forces[u.index] += if graph.are_adjacent(u, v) {
                        let u = positions[u.index];
                        let v = positions[v.index];

                        self.c_attractive
                            * f32::log10(V2f::distance(v, u) / self.ideal_spring_length)
                            * V2f::direction(u, v)
                    } else {
                        let u = positions[u.index];
                        let v = positions[v.index];

                        (self.c_repulsive / V2f::distance_squared(u, v)) * V2f::direction(u, v)
                    };
                }
            }

            for u in graph.vertices() {
                positions[u.index] += cooling * forces[u.index];
                if let Some((lower_bound, upper_bound)) = self.bounds {
                    positions[u.index].x =
                        f32::clamp(positions[u.index].x, lower_bound.x, upper_bound.x);
                    positions[u.index].y =
                        f32::clamp(positions[u.index].y, lower_bound.y, upper_bound.y);
                }
                forces[u.index] = V2f { x: 0.0, y: 0.0 };
            }
            cooling *= self.cooling_rate;
        }
    }
}
