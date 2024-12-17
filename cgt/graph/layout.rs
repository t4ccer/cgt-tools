#![allow(missing_docs)]

use crate::{graph::Graph, has::Has, numeric::v2f::V2f};
use std::f32::consts::PI;

pub struct SpringEmbedder {
    pub cooling_rate: f32,
    pub c_attractive: f32,
    pub c_repulsive: f32,
    pub ideal_spring_length: f32,
    pub iterations: usize,
    pub bounds: Option<(V2f, V2f)>,
}

impl SpringEmbedder {
    pub fn layout<G, V>(&self, graph: &mut G)
    where
        G: Graph<V>,
        V: Has<V2f>,
    {
        let mut forces: Vec<V2f> = vec![V2f { x: 0.0, y: 0.0 }; graph.size()];
        let mut cooling = 1.0;

        for _ in 0..self.iterations {
            for u in graph.vertex_indices() {
                for v in graph.vertex_indices() {
                    if u == v {
                        continue;
                    }

                    let u_pos = *graph.get_vertex(u).get_inner();
                    let v_pos = *graph.get_vertex(v).get_inner();

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
                *graph.get_vertex_mut(u).get_inner_mut() += cooling * forces[u.index];
                if let Some((lower_bound, upper_bound)) = self.bounds {
                    graph.get_vertex_mut(u).get_inner_mut().x = f32::clamp(
                        graph.get_vertex(u).get_inner().x,
                        lower_bound.x,
                        upper_bound.x,
                    );
                    graph.get_vertex_mut(u).get_inner_mut().y = f32::clamp(
                        graph.get_vertex(u).get_inner().y,
                        lower_bound.y,
                        upper_bound.y,
                    );
                }
                forces[u.index] = V2f { x: 0.0, y: 0.0 };
            }
            cooling *= self.cooling_rate;
        }
    }
}

pub struct CircleEdge {
    pub circle_radius: f32,
    pub vertex_radius: f32,
}

impl CircleEdge {
    pub fn layout<G, V>(&self, graph: &mut G)
    where
        G: Graph<V>,
        V: Has<V2f>,
    {
        let n = graph.size();
        for i in graph.vertex_indices() {
            let angle = (2.0 * PI * i.index as f32) / n as f32;
            let vertex_pos = V2f {
                x: (self.circle_radius - self.vertex_radius) * f32::cos(angle) + self.circle_radius,
                y: (self.circle_radius - self.vertex_radius) * f32::sin(angle) + self.circle_radius,
            };
            *graph.get_vertex_mut(i).get_inner_mut() = vertex_pos;
        }
    }
}
