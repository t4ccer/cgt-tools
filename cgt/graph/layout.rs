#![allow(missing_docs)]

use crate::{graph::Graph, has::Has, numeric::v2f::V2f};
use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Bounds {
    pub lower: V2f,
    pub upper: V2f,
    pub c_middle_attractive: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct SpringEmbedder {
    pub cooling_rate: f32,
    pub c_attractive: f32,
    pub c_repulsive: f32,
    pub ideal_spring_length: f32,
    pub iterations: usize,
    pub bounds: Option<Bounds>,
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

                    let distance_squared = V2f::distance_squared(u_pos, v_pos);

                    forces[u.index] += if distance_squared < 0.1 {
                        // same position, usually happens only when region bound is tiny
                        // and position gets clamped so to avoid dividing by zero we add
                        // maximum repulsive force that will push nodes to opposite edges of
                        // the bounded region
                        V2f {
                            x: f32::MAX * (u < v) as u8 as f32,
                            y: 0.0,
                        }
                    } else if graph.are_adjacent(u, v) || graph.are_adjacent(v, u) {
                        // We add force no matter if connection is unidirectional or not
                        self.c_attractive
                            * f32::log10(distance_squared.sqrt() / self.ideal_spring_length)
                            * V2f::direction(u_pos, v_pos)
                    } else {
                        (self.c_repulsive / distance_squared) * V2f::direction(v_pos, u_pos)
                    };
                }
            }

            if let Some(bounds) = self.bounds
                && let Some(c_middle_attractive) = bounds.c_middle_attractive
            {
                let middle = V2f {
                    x: (bounds.upper.x - bounds.lower.x).mul_add(0.5, bounds.lower.x),
                    y: (bounds.upper.y - bounds.lower.y).mul_add(0.5, bounds.lower.y),
                };

                for u in graph.vertex_indices() {
                    let u_pos = *graph.get_vertex(u).get_inner();
                    let d = V2f::distance(u_pos, middle);
                    forces[u.index] += c_middle_attractive * d * V2f::direction(u_pos, middle);
                }
            }

            for u in graph.vertex_indices() {
                *graph.get_vertex_mut(u).get_inner_mut() += cooling * forces[u.index];
                if let Some(bounds) = self.bounds {
                    graph.get_vertex_mut(u).get_inner_mut().x = f32::clamp(
                        graph.get_vertex(u).get_inner().x,
                        bounds.lower.x,
                        bounds.upper.x,
                    );
                    graph.get_vertex_mut(u).get_inner_mut().y = f32::clamp(
                        graph.get_vertex(u).get_inner().y,
                        bounds.lower.y,
                        bounds.upper.y,
                    );
                }
                forces[u.index] = V2f { x: 0.0, y: 0.0 };
            }
            cooling *= self.cooling_rate;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct CircleEdge {
    /// Radius of the circle that the vertices are laid out inside of. They sit
    /// [`CircleEdge::vertex_radius`] further in, so that they fit within it
    pub circle_radius: f32,
    pub vertex_radius: f32,

    /// Point the circle is centered on
    pub center: V2f,
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
                x: (self.circle_radius - self.vertex_radius)
                    .mul_add(f32::cos(angle), self.center.x),
                y: (self.circle_radius - self.vertex_radius)
                    .mul_add(f32::sin(angle), self.center.y),
            };
            *graph.get_vertex_mut(i).get_inner_mut() = vertex_pos;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{graph::VertexIndex, impl_has};

    #[derive(Clone, Copy)]
    struct Vertex {
        position: V2f,
    }

    impl_has!(Vertex -> position -> V2f);

    #[test]
    fn circle_edge_places_vertices_around_its_center() {
        use crate::graph::adjacency_matrix::undirected::UndirectedGraph;

        let center = V2f { x: 320.0, y: 200.0 };
        let circle = CircleEdge {
            circle_radius: 200.0,
            vertex_radius: 16.0,
            center,
        };

        let mut graph = UndirectedGraph::empty(
            &[Vertex {
                position: V2f::ZERO,
            }; 8],
        );
        circle.layout(&mut graph);

        for vertex in graph.vertex_indices() {
            let position: V2f = *graph.get_vertex(vertex).get_inner();
            let distance = V2f::distance(position, center);

            // Vertices sit a whole vertex radius inside the circle, so that they fit in it
            assert!(
                (distance - (circle.circle_radius - circle.vertex_radius)).abs() < 0.01,
                "vertex {} sits {distance} from the center",
                vertex.index
            );
        }

        // The first vertex starts the circle at angle zero, due right of the center
        let first: V2f = *graph.get_vertex(VertexIndex { index: 0 }).get_inner();
        assert!((first.x - (center.x + 184.0)).abs() < 0.01);
        assert!((first.y - center.y).abs() < 0.01);
    }
}
