use cgt::{
    graph::{
        adjacency_matrix::directed::DirectedGraph, layout::SpringEmbedder, Graph, VertexIndex,
    },
    has::Has,
    numeric::v2f::V2f,
    short::partizan::games::digraph_placement::{DigraphPlacement, VertexColor},
};
use imgui::{ComboBoxFlags, Condition, MouseButton, StyleColor};
use std::{f32::consts::PI, fmt::Write};

use crate::{
    imgui_enum, impl_titled_window,
    widgets::{self, canonical_form::CanonicalFormWindow, COLOR_BLUE, COLOR_RED},
    Context, DetailOptions, Details, EvalTask, IsCgtWindow, RawOf, Task, TitledWindow, UpdateKind,
};

const VERTEX_RADIUS: f32 = 16.0;
const ARROW_HEAD_SIZE: f32 = 4.0;

imgui_enum! {
    NewVertexColor {
        Left, "Blue",
        Right, "Red",
    }
}

imgui_enum! {
    GraphEditingMode {
        DragVertex, "Drag vertex",
        TintVertexBlue, "Color vertex blue (left)",
        TintVertexRed, "Color vertex red (right)",
        MoveLeft, "Blue move (left)",
        MoveRight, "Red move (right)",
        AddVertex, "Add vertex",
        DeleteVertex, "Remove vertex",
        AddEdge, "Add/Remove edge",
    }
}

imgui_enum! {
    RepositionMode {
        SpringEmbedder, "Spring Embedder",
        Circle, "Circle",
    }
}

#[derive(Debug, Clone, Copy)]
struct PositionedVertex {
    color: VertexColor,
    position: V2f,
}

impl Has<VertexColor> for PositionedVertex {
    fn get_inner(&self) -> &VertexColor {
        &self.color
    }

    fn get_inner_mut(&mut self) -> &mut VertexColor {
        &mut self.color
    }
}

enum Action {
    None,
    Remove(VertexIndex),
    Move(VertexIndex),
}

#[derive(Debug, Clone)]
pub struct DigraphPlacementWindow {
    game: DigraphPlacement<PositionedVertex, DirectedGraph<PositionedVertex>>,
    reposition_option_selected: RawOf<RepositionMode>,
    new_vertex_color: RawOf<NewVertexColor>,
    editing_mode: RawOf<GraphEditingMode>,
    new_edge_starting_vertex: Option<VertexIndex>,
    alternating_moves: bool,
    edge_creates_vertex: bool,
    details_options: DetailOptions,
    details: Option<Details>,
}

impl DigraphPlacementWindow {
    pub fn new() -> DigraphPlacementWindow {
        DigraphPlacementWindow {
            game: DigraphPlacement::new(DirectedGraph::from_edges(
                &[
                    (VertexIndex { index: 1 }, VertexIndex { index: 0 }),
                    (VertexIndex { index: 2 }, VertexIndex { index: 0 }),
                    (VertexIndex { index: 3 }, VertexIndex { index: 0 }),
                    (VertexIndex { index: 4 }, VertexIndex { index: 0 }),
                    //
                    (VertexIndex { index: 2 }, VertexIndex { index: 1 }),
                    (VertexIndex { index: 3 }, VertexIndex { index: 1 }),
                    (VertexIndex { index: 4 }, VertexIndex { index: 1 }),
                    //
                    (VertexIndex { index: 1 }, VertexIndex { index: 4 }),
                    (VertexIndex { index: 2 }, VertexIndex { index: 4 }),
                    (VertexIndex { index: 3 }, VertexIndex { index: 4 }),
                ],
                &[
                    PositionedVertex {
                        color: VertexColor::Right,
                        position: V2f::ZERO,
                    },
                    PositionedVertex {
                        color: VertexColor::Right,
                        position: V2f::ZERO,
                    },
                    PositionedVertex {
                        color: VertexColor::Left,
                        position: V2f::ZERO,
                    },
                    PositionedVertex {
                        color: VertexColor::Left,
                        position: V2f::ZERO,
                    },
                    PositionedVertex {
                        color: VertexColor::Left,
                        position: V2f::ZERO,
                    },
                ],
            )),
            new_vertex_color: RawOf::new(NewVertexColor::Left),
            reposition_option_selected: RawOf::new(RepositionMode::SpringEmbedder),
            editing_mode: RawOf::new(GraphEditingMode::DragVertex),
            new_edge_starting_vertex: None,
            alternating_moves: true,
            edge_creates_vertex: true,
            details_options: DetailOptions::new(),
            details: None,
        }
    }

    pub fn reposition_circle(&mut self) {
        let n = self.game.graph.size();
        let packing_circle_radius = VERTEX_RADIUS * (self.game.graph.size() as f32 + 4.0) * 0.5;
        for i in self.game.graph.vertex_indices() {
            let angle = (2.0 * PI * i.index as f32) / n as f32;
            let vertex_pos = V2f {
                x: (packing_circle_radius - VERTEX_RADIUS) * f32::cos(angle)
                    + packing_circle_radius,
                y: (packing_circle_radius - VERTEX_RADIUS) * f32::sin(angle)
                    + packing_circle_radius,
            };
            self.game.graph.get_vertex_mut(i).position = vertex_pos;
        }
    }

    pub fn reposition(&mut self, graph_panel_size: V2f) {
        match self.reposition_option_selected.as_enum() {
            RepositionMode::Circle => {
                self.reposition_circle();
            }
            RepositionMode::SpringEmbedder => {
                let spring_embedder = SpringEmbedder {
                    cooling_rate: 0.99999,
                    c_attractive: 1.0,
                    c_repulsive: 250.0,
                    ideal_spring_length: 140.0,
                    iterations: 1 << 14,
                    bounds: Some((
                        V2f {
                            x: VERTEX_RADIUS,
                            y: VERTEX_RADIUS,
                        },
                        V2f {
                            x: f32::max(VERTEX_RADIUS, graph_panel_size.x - VERTEX_RADIUS * 2.0),
                            y: f32::max(VERTEX_RADIUS, graph_panel_size.y - VERTEX_RADIUS * 2.0),
                        },
                    )),
                };
                // TODO: Make spring_embedder generic over Has<V2f>
                let mut vertex_positions = self
                    .game
                    .graph
                    .vertex_indices()
                    .map(|i| self.game.graph.get_vertex(i).position)
                    .collect::<Vec<_>>();
                spring_embedder.layout(&self.game.graph, &mut vertex_positions);
                for i in self.game.graph.vertex_indices() {
                    self.game.graph.get_vertex_mut(i).position = vertex_positions[i.index];
                }
            }
        }
    }
}

impl IsCgtWindow for TitledWindow<DigraphPlacementWindow> {
    impl_titled_window!("Digraph Placement");

    fn init(&self, ctx: &Context) {
        let graph = self.content.game.graph.map(|v| v.color);
        ctx.schedule_task(crate::Task::EvalDigraphPlacement(crate::EvalTask {
            window: self.window_id,
            game: DigraphPlacement::new(graph),
        }));
    }
    fn update(&mut self, update: crate::UpdateKind) {
        let graph = self.content.game.graph.map(|v| v.color);
        match update {
            UpdateKind::DigraphPlacementDetails(game, details) => {
                if graph == game.graph {
                    self.content.details = Some(details);
                }
            }
            _ => {}
        }
    }

    fn draw(&mut self, ui: &imgui::Ui, ctx: &mut Context) {
        let mut should_reposition = false;
        let mut is_dirty = false;

        let mut graph_panel_size = V2f::ZERO;

        ui.window(&self.title)
            .position(ui.io().mouse_pos, Condition::Appearing)
            .size([800.0, 450.0], Condition::Appearing)
            .bring_to_front_on_focus(true)
            .menu_bar(true)
            .opened(&mut self.is_open)
            .build(|| {
                let draw_list = ui.get_window_draw_list();

                if let Some(_menu_bar) = ui.begin_menu_bar() {
                    if let Some(_new_menu) = ui.begin_menu("New") {
                        if ui.menu_item("Duplicate") {
                            let w = self.content.clone();
                            ctx.new_windows
                                .push(Box::new(TitledWindow::without_title(w)));
                        };
                        if ui.menu_item("Canonical Form") {
                            if let Some(details) = self.content.details.clone() {
                                let w = CanonicalFormWindow::with_details(details);
                                ctx.new_windows
                                    .push(Box::new(TitledWindow::without_title(w)));
                            }
                        }
                    }
                }

                ui.columns(2, "columns", true);

                let short_inputs = ui.push_item_width(200.0);
                self.content.reposition_option_selected.combo(
                    ui,
                    "##Reposition Mode",
                    ComboBoxFlags::empty(),
                );
                ui.same_line();
                should_reposition = ui.button("Reposition");
                ui.same_line();
                if ui.button("Clear") {
                    self.content.game = DigraphPlacement::new(DirectedGraph::empty(&[]));
                    is_dirty = true;
                }

                if matches!(
                    self.content.editing_mode.as_enum(),
                    GraphEditingMode::AddVertex
                        | GraphEditingMode::AddEdge if self.content.edge_creates_vertex
                ) {
                    self.content.new_vertex_color.combo(
                        ui,
                        "New Vertex Color",
                        ComboBoxFlags::HEIGHT_LARGE,
                    );
                }

                self.content
                    .editing_mode
                    .combo(ui, "Edit Mode", ComboBoxFlags::HEIGHT_LARGE);
                short_inputs.end();

                if matches!(
                    self.content.editing_mode.as_enum(),
                    GraphEditingMode::MoveLeft | GraphEditingMode::MoveRight
                ) {
                    ui.same_line();
                    ui.checkbox("Alternating", &mut self.content.alternating_moves);
                } else if matches!(
                    self.content.editing_mode.as_enum(),
                    GraphEditingMode::AddEdge
                ) {
                    ui.same_line();
                    ui.checkbox("Add vertex", &mut self.content.edge_creates_vertex);
                }

                let graph_region_start = V2f::from(ui.cursor_screen_pos());
                let control_panel_height = ui.cursor_pos()[1];

                let mut max_y = f32::NEG_INFINITY;
                let vertex_border_color = ui.style_color(StyleColor::Text);
                let mut action = Action::None;
                for this_vertex_idx in self.content.game.graph.vertex_indices() {
                    let absolute_vertex_pos =
                        self.content.game.graph.get_vertex(this_vertex_idx).position;
                    let _vertex_id = ui.push_id_usize(this_vertex_idx.index);
                    let this_vertex_pos = graph_region_start + absolute_vertex_pos;
                    max_y = max_y.max(this_vertex_pos.y);
                    let button_pos = this_vertex_pos - VERTEX_RADIUS;
                    let button_size = V2f {
                        x: VERTEX_RADIUS * 2.0,
                        y: VERTEX_RADIUS * 2.0,
                    };
                    ui.set_cursor_screen_pos(button_pos);

                    if ui.invisible_button("vertex", button_size) {
                        match self.content.editing_mode.as_enum() {
                            GraphEditingMode::DragVertex => { /* NOOP */ }
                            GraphEditingMode::TintVertexBlue => {
                                self.content
                                    .game
                                    .graph
                                    .get_vertex_mut(this_vertex_idx)
                                    .color = VertexColor::Left;
                                is_dirty = true;
                            }
                            GraphEditingMode::TintVertexRed => {
                                self.content
                                    .game
                                    .graph
                                    .get_vertex_mut(this_vertex_idx)
                                    .color = VertexColor::Right;
                                is_dirty = true;
                            }
                            GraphEditingMode::DeleteVertex => {
                                // We don't remove it immediately because we're just iterating over
                                // vertices
                                action = Action::Remove(this_vertex_idx);
                                is_dirty = true;
                            }
                            GraphEditingMode::AddEdge => { /* NOOP */ }
                            GraphEditingMode::AddVertex => { /* NOOP */ }
                            GraphEditingMode::MoveLeft => {
                                if self.content.game.graph.get_vertex(this_vertex_idx).color
                                    == VertexColor::Left
                                {
                                    action = Action::Move(this_vertex_idx);
                                    if self.content.alternating_moves {
                                        self.content.editing_mode =
                                            RawOf::new(GraphEditingMode::MoveRight);
                                    }
                                    is_dirty = true;
                                }
                            }
                            GraphEditingMode::MoveRight => {
                                if self.content.game.graph.get_vertex(this_vertex_idx).color
                                    == VertexColor::Right
                                {
                                    action = Action::Move(this_vertex_idx);
                                    if self.content.alternating_moves {
                                        self.content.editing_mode =
                                            RawOf::new(GraphEditingMode::MoveLeft);
                                    }
                                    is_dirty = true;
                                }
                            }
                        }
                    };

                    if ui.is_item_activated()
                        && matches!(
                            self.content.editing_mode.as_enum(),
                            GraphEditingMode::AddEdge
                        )
                    {
                        self.content.new_edge_starting_vertex = Some(this_vertex_idx);
                    }

                    let mouse_pos = V2f::from(ui.io().mouse_pos);
                    if !ui.io()[MouseButton::Left]
                        && mouse_pos.x >= button_pos.x
                        && mouse_pos.x <= (button_pos.x + button_size.x)
                        && mouse_pos.y >= button_pos.y
                        && mouse_pos.y <= (button_pos.y + button_size.y)
                    {
                        if let Some(starting_vertex) = self.content.new_edge_starting_vertex.take()
                        {
                            if starting_vertex != this_vertex_idx {
                                self.content.game.graph.connect(
                                    starting_vertex,
                                    this_vertex_idx,
                                    !self
                                        .content
                                        .game
                                        .graph
                                        .are_adjacent(starting_vertex, this_vertex_idx),
                                );
                                is_dirty = true;
                            }
                        }
                    }

                    if ui.is_item_active()
                        && matches!(
                            self.content.editing_mode.as_enum(),
                            GraphEditingMode::DragVertex
                        )
                    {
                        let mouse_delta = V2f::from(ui.io().mouse_delta);
                        self.content
                            .game
                            .graph
                            .get_vertex_mut(this_vertex_idx)
                            .position = V2f {
                            x: f32::max(VERTEX_RADIUS, absolute_vertex_pos.x + mouse_delta.x),
                            y: f32::max(VERTEX_RADIUS, absolute_vertex_pos.y + mouse_delta.y),
                        };
                    }

                    let vertex_fill_color = match self
                        .content
                        .game
                        .graph
                        .get_vertex_mut(this_vertex_idx)
                        .color
                    {
                        VertexColor::Left => COLOR_BLUE,
                        VertexColor::Right => COLOR_RED,
                    };

                    draw_list
                        .add_circle(this_vertex_pos, VERTEX_RADIUS, vertex_border_color)
                        .filled(false)
                        .build();
                    draw_list
                        .add_circle(this_vertex_pos, VERTEX_RADIUS - 0.5, vertex_fill_color)
                        .filled(true)
                        .build();

                    self.scratch_buffer.clear();
                    self.scratch_buffer
                        .write_fmt(format_args!("{}", this_vertex_idx.index + 1))
                        .unwrap();
                    let off_x = ui.calc_text_size(&self.scratch_buffer)[0];
                    draw_list.add_text(
                        [
                            this_vertex_pos.x - off_x * 0.5,
                            this_vertex_pos.y + VERTEX_RADIUS,
                        ],
                        vertex_border_color,
                        &self.scratch_buffer,
                    );

                    for adjacent_vertex_idx in self.content.game.graph.adjacent_to(this_vertex_idx)
                    {
                        let adjacent_relative_pos = self
                            .content
                            .game
                            .graph
                            .get_vertex(adjacent_vertex_idx)
                            .position;

                        let adjacent_vertex_pos = graph_region_start + adjacent_relative_pos;
                        let direction = V2f::direction(this_vertex_pos, adjacent_vertex_pos);
                        let edge_start_pos = this_vertex_pos + direction * VERTEX_RADIUS;
                        let edge_end_pos = adjacent_vertex_pos - direction * VERTEX_RADIUS;
                        let distance_between_vertices = adjacent_vertex_pos - this_vertex_pos;

                        if distance_between_vertices.x.abs() < 2.0 * VERTEX_RADIUS
                            && distance_between_vertices.y.abs() < 2.0 * VERTEX_RADIUS
                        {
                            continue;
                        }

                        let both_ways = self
                            .content
                            .game
                            .graph
                            .are_adjacent(adjacent_vertex_idx, this_vertex_idx);

                        if !both_ways || this_vertex_idx < adjacent_vertex_idx {
                            draw_list
                                .add_line(edge_start_pos, edge_end_pos, vertex_border_color)
                                .thickness(1.0)
                                .build();
                        }

                        // If connection is both ways then we do not draw arrow heads
                        if !both_ways {
                            draw_list
                                .add_triangle(
                                    edge_end_pos,
                                    V2f {
                                        x: edge_end_pos.x - direction.x * ARROW_HEAD_SIZE
                                            + direction.y * ARROW_HEAD_SIZE,
                                        y: edge_end_pos.y
                                            - direction.y * ARROW_HEAD_SIZE
                                            - direction.x * ARROW_HEAD_SIZE,
                                    },
                                    V2f {
                                        x: edge_end_pos.x
                                            - direction.x * ARROW_HEAD_SIZE
                                            - direction.y * ARROW_HEAD_SIZE,
                                        y: edge_end_pos.y - direction.y * ARROW_HEAD_SIZE
                                            + direction.x * ARROW_HEAD_SIZE,
                                    },
                                    vertex_border_color,
                                )
                                .filled(true)
                                .build();
                        }
                    }
                }

                if let Some(starting_vertex) = self.content.new_edge_starting_vertex {
                    let held_vertex_relative_pos =
                        self.content.game.graph.get_vertex(starting_vertex).position;
                    let held_vertex_pos = graph_region_start + held_vertex_relative_pos;
                    draw_list
                        .add_line(held_vertex_pos, ui.io().mouse_pos, vertex_border_color)
                        .thickness(2.0)
                        .build();
                    // TODO: Draw arrow head
                }

                ui.set_cursor_screen_pos(graph_region_start);
                let style = unsafe { ui.style() };
                graph_panel_size = V2f {
                    x: ui.current_column_width(),
                    y: ui.window_size()[1] - control_panel_height - style.item_spacing[1] * 2.0,
                };
                if matches!(
                    self.content.editing_mode.as_enum(),
                    GraphEditingMode::AddVertex
                ) && ui.invisible_button("Add vertex", graph_panel_size)
                {
                    let mouse_pos = V2f::from(ui.io().mouse_pos);
                    let color = match self.content.new_vertex_color.as_enum() {
                        NewVertexColor::Left => VertexColor::Left,
                        NewVertexColor::Right => VertexColor::Right,
                    };
                    self.content.game.graph.add_vertex(PositionedVertex {
                        color,
                        position: mouse_pos - graph_region_start,
                    });

                    is_dirty = true;
                }

                ui.set_cursor_screen_pos([graph_region_start.x, max_y + VERTEX_RADIUS]);
                ui.next_column();

                match action {
                    Action::None => {}
                    Action::Remove(idx) => {
                        self.content.game.graph.remove_vertex(idx);
                    }
                    Action::Move(idx) => {
                        self.content.game = self.content.game.move_in_vertex(idx);
                    }
                }

                if !ui.io()[MouseButton::Left] {
                    if let Some(edge_start) = self.content.new_edge_starting_vertex.take() {
                        if self.content.edge_creates_vertex {
                            let mouse_pos = V2f::from(ui.io().mouse_pos);
                            let color = match self.content.new_vertex_color.as_enum() {
                                NewVertexColor::Left => VertexColor::Left,
                                NewVertexColor::Right => VertexColor::Right,
                            };
                            self.content.game.graph.add_vertex(PositionedVertex {
                                color,
                                position: V2f {
                                    x: f32::max(VERTEX_RADIUS, mouse_pos.x - graph_region_start.x),
                                    y: f32::max(VERTEX_RADIUS, mouse_pos.y - graph_region_start.y),
                                },
                            });

                            let edge_end = VertexIndex {
                                index: self.content.game.graph.size() - 1,
                            };
                            self.content.game.graph.connect(edge_start, edge_end, true);
                            is_dirty = true;
                        }
                    }
                }

                widgets::game_details!(self, ui, draw_list);

                if is_dirty {
                    self.content.details = None;
                    let graph = self.content.game.graph.map(|v| v.color);
                    ctx.schedule_task(Task::EvalDigraphPlacement(EvalTask {
                        window: self.window_id,
                        game: DigraphPlacement::new(graph),
                    }));
                }
            });

        if should_reposition {
            self.content.reposition(graph_panel_size);
        }
    }
}
