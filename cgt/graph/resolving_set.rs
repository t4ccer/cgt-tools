#![allow(missing_docs)]

use crate::{
    display,
    drawing::{Canvas, Color, TextAlignment},
    graph::{Graph, VertexIndex},
    has::Has,
    numeric::v2f::V2f,
};
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tower {
    Unrestricted,
    Restricted(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Distance {
    Finite(u32),
    Infinite,
}

impl Distance {
    fn succ(self) -> Distance {
        match self {
            Distance::Finite(n) => Distance::Finite(n + 1),
            Distance::Infinite => Distance::Infinite,
        }
    }

    fn into_finite(self) -> Option<u32> {
        match self {
            Distance::Finite(n) => Some(n),
            Distance::Infinite => None,
        }
    }
}

impl std::fmt::Display for Distance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Distance::Finite(finite) => write!(f, "{finite}"),
            Distance::Infinite => write!(f, "*"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Distances {
    inner: Vec<Distance>,
}

impl std::fmt::Display for Distances {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        display::parens(f, |f| {
            for (idx, dist) in self.inner.iter().enumerate() {
                if idx != 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{dist}")?;
            }
            Ok(())
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Vertex {
    tower: Option<Tower>,
    distances: Distances,
    is_unique: bool,
}

impl Vertex {
    pub fn new(tower: Option<Tower>) -> Self {
        Self {
            tower,
            distances: Distances { inner: Vec::new() },
            is_unique: false,
        }
    }

    pub fn tower(&self) -> Option<Tower> {
        self.tower
    }

    pub fn set_tower(&mut self, tower: Option<Tower>) {
        self.tower = tower;
    }

    pub fn is_unique(&self) -> bool {
        self.is_unique
    }

    pub fn distances(&self) -> &Distances {
        &self.distances
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct WithDistance<T> {
    distance: Distance,
    inner: T,
}

pub fn label_distances<G, V>(graph: &mut G)
where
    G: Graph<V>,
    V: Has<Vertex>,
{
    let tower_count = graph
        .vertices()
        .filter(|v| (*v).get_inner().tower.is_some())
        .count();

    for v_idx in graph.vertex_indices() {
        let v: &mut Vertex = graph.get_vertex_mut(v_idx).get_inner_mut();
        v.distances = Distances {
            inner: vec![Distance::Infinite; tower_count],
        };
    }

    let mut tower_no = 0;
    for tower_idx in graph.vertex_indices() {
        let tower_vertex: &mut Vertex = graph.get_vertex_mut(tower_idx).get_inner_mut();
        if tower_vertex.tower.is_none() {
            continue;
        };
        tower_vertex.get_inner_mut().distances.inner[tower_no] = Distance::Finite(0);
        let mut queue = BTreeSet::from_iter([WithDistance {
            distance: Distance::Finite(0),
            inner: tower_idx,
        }]);
        while let Some(v_idx) = queue.pop_first() {
            for adjacent_idx in graph.vertex_indices() {
                if !graph.are_adjacent(v_idx.inner, adjacent_idx) {
                    continue;
                }
                let new_distance = v_idx.distance.succ();
                let current_distance: &mut Distance = &mut graph
                    .get_vertex_mut(adjacent_idx)
                    .get_inner_mut()
                    .distances
                    .inner[tower_no];
                if new_distance < *current_distance {
                    *current_distance = new_distance;
                    queue.insert(WithDistance {
                        distance: new_distance,
                        inner: adjacent_idx,
                    });
                }
            }
        }
        tower_no += 1;
    }

    mark_duplicates(graph);
}

fn mark_duplicates<G, V>(graph: &mut G)
where
    G: Graph<V>,
    V: Has<Vertex>,
{
    let mut seen_distances = HashMap::new();
    for v in graph.vertices() {
        *seen_distances
            .entry(v.get_inner().distances.clone())
            .or_insert(0) += 1;
    }
    for v_idx in graph.vertex_indices() {
        let v: &mut Vertex = graph.get_vertex_mut(v_idx).get_inner_mut();
        v.is_unique = *seen_distances.get(&v.distances).unwrap() == 1;
    }
}

pub fn draw_graph<G, V, C>(canvas: &mut C, graph: &G)
where
    G: Graph<V>,
    V: Has<Vertex> + Has<V2f>,
    C: Canvas,
{
    let mut tower_no = 0;
    graph.draw(canvas, move |canvas, idx| {
        let vertex: &Vertex = graph.get_vertex(idx).get_inner();
        let vertex_position: V2f = *graph.get_vertex(idx).get_inner();
        canvas.vertex(
            vertex_position,
            if vertex.tower.is_some() {
                Color::DARK_GRAY
            } else {
                Color::LIGHT_GRAY
            },
            idx,
        );

        let text_position = vertex_position
            + V2f {
                x: 0.0,
                y: C::vertex_radius(),
            };
        let text_color = if vertex.is_unique {
            Color::BLACK
        } else {
            Color::RED
        };

        if vertex.tower.is_some() {
            tower_no += 1;
            canvas.text(
                text_position,
                format_args!("\n{} = {}", tower_no, vertex.distances),
                TextAlignment::Center,
                text_color,
            );
        } else {
            canvas.text(
                text_position,
                format_args!("\n{}", vertex.distances),
                TextAlignment::Center,
                text_color,
            );
        }
    });
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodeVertex {
    distances: Distances,
    is_original: bool,
    is_colliding: bool,
}

impl CodeVertex {
    pub fn is_colliding(&self) -> bool {
        self.is_colliding
    }

    pub fn distances(&self) -> &Distances {
        &self.distances
    }

    pub fn is_original(&self) -> bool {
        self.is_original
    }
}

pub fn one_bit_error_auxiliary_graph<G1, V1, G2>(graph: &G1, finite_to_infinite_error: bool) -> G2
where
    G1: Graph<V1>,
    V1: Has<Vertex>,
    G2: Graph<CodeVertex>,
{
    let largest_distance: u32 = graph
        .vertices()
        .flat_map(|v| v.get_inner().distances.inner.iter().copied())
        .filter_map(Distance::into_finite)
        .max()
        .unwrap_or(0);
    let bit_length = size_of_val(&largest_distance) as u32 * 8 - largest_distance.leading_zeros();

    let mut aux = G2::empty(&[]);
    let mut vertices = HashMap::<Distances, VertexIndex>::new();
    for v1_idx in graph.vertex_indices() {
        let v1_real = graph.get_vertex(v1_idx);
        let v1_aux = CodeVertex {
            distances: v1_real.get_inner().distances.clone(),
            is_original: true,
            is_colliding: false,
        };
        let v1_aux_d = v1_aux.distances.clone();

        let v1_aux_idx = *vertices
            .entry(v1_aux.distances.clone())
            .or_insert_with(|| aux.add_vertex(v1_aux));
        aux.get_vertex_mut(v1_aux_idx).is_original = true;

        for tower_idx in 0..v1_aux_d.inner.len() {
            let v2_aux = CodeVertex {
                distances: v1_real.get_inner().distances.clone(),
                is_original: false,
                is_colliding: false,
            };

            match v1_real.get_inner().distances.inner[tower_idx] {
                Distance::Finite(real_distance) => {
                    for bit_idx in 0..bit_length {
                        let mut v2_aux = v2_aux.clone();
                        v2_aux.distances.inner[tower_idx] =
                            Distance::Finite(real_distance ^ (1 << bit_idx));
                        let v2_idx = *vertices
                            .entry(v2_aux.get_inner().distances.clone())
                            .or_insert_with(|| aux.add_vertex(v2_aux));
                        aux.connect(v1_aux_idx, v2_idx, true);
                    }

                    if finite_to_infinite_error {
                        let mut v2_aux = v2_aux.clone();
                        v2_aux.distances.inner[tower_idx] = Distance::Infinite;
                        let v2_idx = *vertices
                            .entry(v2_aux.get_inner().distances.clone())
                            .or_insert_with(|| aux.add_vertex(v2_aux));
                        aux.connect(v1_aux_idx, v2_idx, true);
                    }
                }
                Distance::Infinite => {
                    for possible_distance in 0..largest_distance {
                        let mut v2_aux = v2_aux.clone();
                        v2_aux.distances.inner[tower_idx] = Distance::Finite(possible_distance);
                        let v2_idx = *vertices
                            .entry(v2_aux.get_inner().distances.clone())
                            .or_insert_with(|| aux.add_vertex(v2_aux));
                        aux.connect(v1_aux_idx, v2_idx, true);
                    }
                }
            }
        }
    }

    for v1_idx in aux.vertex_indices() {
        if !aux.get_vertex(v1_idx).is_original {
            continue;
        }

        for v2_idx in aux.vertex_indices() {
            if !aux.are_adjacent(v1_idx, v2_idx) {
                continue;
            }

            if aux.get_vertex(v2_idx).is_original {
                aux.get_vertex_mut(v1_idx).is_colliding = true;
                aux.get_vertex_mut(v2_idx).is_colliding = true;
            }

            for v3_idx in aux.vertex_indices() {
                if !aux.are_adjacent(v2_idx, v3_idx) {
                    continue;
                }

                if aux.get_vertex(v3_idx).is_original && v1_idx != v3_idx {
                    aux.get_vertex_mut(v1_idx).is_colliding = true;
                    aux.get_vertex_mut(v3_idx).is_colliding = true;
                }
            }
        }
    }

    aux
}

pub fn draw_code_graph<G, V, C>(canvas: &mut C, graph: &G)
where
    G: Graph<V>,
    V: Has<CodeVertex> + Has<V2f>,
    C: Canvas,
{
    graph.draw(canvas, move |canvas, idx| {
        let vertex: &CodeVertex = graph.get_vertex(idx).get_inner();
        let vertex_position: V2f = *graph.get_vertex(idx).get_inner();
        canvas.vertex(
            vertex_position,
            if vertex.is_original {
                Color::DARK_GRAY
            } else {
                Color::LIGHT_GRAY
            },
            idx,
        );

        let text_position = vertex_position
            + V2f {
                x: 0.0,
                y: C::vertex_radius(),
            };
        let text_color = if vertex.is_colliding {
            Color::RED
        } else {
            Color::BLACK
        };

        canvas.text(
            text_position,
            format_args!("\n{}", vertex.distances),
            TextAlignment::Center,
            text_color,
        );
    });
}
