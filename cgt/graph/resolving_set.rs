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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Distances {
    inner: Vec<u32>,
}

impl std::fmt::Display for Distances {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        display::parens(f, |f| {
            for (idx, dist) in self.inner.iter().enumerate() {
                if idx != 0 {
                    write!(f, ", ")?;
                }

                if *dist == u32::MAX {
                    write!(f, "*")?;
                } else {
                    write!(f, "{dist}")?;
                }
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct WithDistance<T> {
    distance: u32,
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
            inner: vec![u32::MAX; tower_count],
        };
    }

    let mut tower_no = 0;
    for tower_idx in graph.vertex_indices() {
        let tower_vertex: &mut Vertex = graph.get_vertex_mut(tower_idx).get_inner_mut();
        if tower_vertex.tower.is_none() {
            continue;
        };
        tower_vertex.get_inner_mut().distances.inner[tower_no] = 0;
        let mut queue = BTreeSet::from_iter([WithDistance {
            distance: 0,
            inner: tower_idx,
        }]);
        while let Some(v_idx) = queue.pop_first() {
            // Dance around borrow checker
            for adjacent_idx in (0..graph.size()).map(|index| VertexIndex { index }) {
                if !graph.are_adjacent(v_idx.inner, adjacent_idx) {
                    continue;
                }
                let d = v_idx.distance + 1;
                if d < graph.get_vertex(adjacent_idx).get_inner().distances.inner[tower_no] {
                    graph
                        .get_vertex_mut(adjacent_idx)
                        .get_inner_mut()
                        .distances
                        .inner[tower_no] = d;
                    queue.insert(WithDistance {
                        distance: d,
                        inner: adjacent_idx,
                    });
                }
            }
        }
        tower_no += 1;
    }

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
