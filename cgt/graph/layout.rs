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
    pub fn layout<G, V>(&self, graph: &G, positions: &mut [V2f])
    where
        G: Graph<V>,
    {
        assert_eq!(graph.size(), positions.len());

        let mut forces: Vec<V2f> = vec![V2f { x: 0.0, y: 0.0 }; positions.len()];
        let mut cooling = 1.0;

        for _ in 0..self.iterations {
            for u in graph.vertex_indices() {
                for v in graph.vertex_indices() {
                    if u == v {
                        continue;
                    }

                    let u_pos = positions[u.index];
                    let v_pos = positions[v.index];

                    // We add force no matter if connection is unidirectional or not
                    forces[u.index] += if graph.are_adjacent(u, v) || graph.are_adjacent(v, u) {
                        self.c_attractive
                            * f32::log10(V2f::distance(v_pos, u_pos) / self.ideal_spring_length)
                            * V2f::direction(u_pos, v_pos)
                    } else {
                        let d = V2f::distance_squared(u_pos, v_pos);
                        if d == 0.0 {
                            // same position, usually happens only when region bound is tiny
                            // and position gets clamped so to avoid dividing by zero we add
                            // maximum repulsive force that will push nodes to opposite edges of
                            // the bounded region
                            V2f {
                                x: f32::MAX * (u < v) as u8 as f32,
                                y: 0.0,
                            }
                        } else {
                            (self.c_repulsive / d) * V2f::direction(v_pos, u_pos)
                        }
                    };
                }
            }

            for u in graph.vertex_indices() {
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
