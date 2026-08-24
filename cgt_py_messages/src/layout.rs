use cgt::{
    drawing::Canvas,
    graph::{
        Graph,
        layout::{Bounds, CircleEdge, SpringEmbedder},
    },
    has::Has,
    numeric::v2f::V2f,
};

pub const MAX_SPRING_WORK: usize = 1 << 22;

pub const SPRING_ITERATIONS: std::ops::RangeInclusive<usize> = 256..=4096;

pub const PULL_TO_CENTER: f32 = 0.001;

pub fn max_spring_iterations(vertices: usize) -> usize {
    MAX_SPRING_WORK / usize::max(vertices * vertices, 1)
}

pub fn default_circle<C>(available_space: V2f) -> CircleEdge
where
    C: Canvas,
{
    CircleEdge {
        circle_radius: f32::min(available_space.x, available_space.y) * 0.5,
        vertex_radius: C::vertex_radius(),
        center: V2f {
            x: available_space.x * 0.5,
            y: available_space.y * 0.5,
        },
    }
}

pub fn default_bounds<C>(available_space: V2f, c_middle_attractive: f32) -> Bounds
where
    C: Canvas,
{
    let vertex_radius = C::vertex_radius();
    Bounds {
        lower: V2f {
            x: vertex_radius,
            y: vertex_radius,
        },
        upper: V2f {
            x: f32::max(vertex_radius, available_space.x - vertex_radius),
            y: f32::max(vertex_radius, available_space.y - vertex_radius),
        },
        c_middle_attractive: Some(c_middle_attractive),
    }
}

pub fn default_spring<C>(vertices: usize, available_space: V2f) -> SpringEmbedder
where
    C: Canvas,
{
    let iterations = usize::clamp(
        max_spring_iterations(vertices),
        *SPRING_ITERATIONS.start(),
        *SPRING_ITERATIONS.end(),
    );

    let area = available_space.x * available_space.y;
    let ideal_spring_length = f32::max(
        f32::sqrt(area / f32::max(vertices as f32, 1.0)) * 0.5,
        C::vertex_radius() * 3.0,
    );

    SpringEmbedder {
        cooling_rate: f32::powf(0.01, 1.0 / iterations as f32),
        c_attractive: 1.0,
        c_repulsive: (250.0 / (40.0 * 40.0)) * ideal_spring_length * ideal_spring_length,
        ideal_spring_length,
        iterations,
        bounds: Some(default_bounds::<C>(available_space, PULL_TO_CENTER)),
    }
}

pub fn arrange<C, G, V>(graph: &mut G, available_space: V2f)
where
    C: Canvas,
    G: Graph<V>,
    V: Has<V2f>,
{
    let vertices = graph.size();
    default_circle::<C>(available_space).layout(graph);
    default_spring::<C>(vertices, available_space).layout(graph);
}
