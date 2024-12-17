use cgt::{
    graph::{
        adjacency_matrix::undirected::UndirectedGraph, layout::SpringEmbedder, Graph, VertexIndex,
    },
    has::Has,
    numeric::v2f::V2f,
    short::partizan::games::snort::{Snort, VertexColor, VertexKind},
};
use imgui::{ComboBoxFlags, Condition, ImColor32, MouseButton, StyleColor};
use std::{f32::consts::PI, fmt::Write};

use crate::{
    imgui_enum, impl_titled_window,
    widgets::{self, canonical_form::CanonicalFormWindow},
    Context, DetailOptions, Details, EvalTask, IsCgtWindow, RawOf, Task, TitledWindow, UpdateKind,
};

const VERTEX_RADIUS: f32 = 16.0;

imgui_enum! {
    GraphEditingMode {
        DragVertex, "Drag vertex",
        TintVertexBlue, "Tint vertex blue (left)",
        TintVertexRed, "Tint vertex red (right)",
        TintVertexNone, "Untint vertex",
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
pub struct PositionedVertex {
    kind: VertexKind,
    position: V2f,
}

impl Has<VertexKind> for PositionedVertex {
    fn get_inner(&self) -> &VertexKind {
        &self.kind
    }

    fn get_inner_mut(&mut self) -> &mut VertexKind {
        &mut self.kind
    }
}

#[derive(Debug, Clone)]
pub struct SnortWindow {
    game: Snort<PositionedVertex, UndirectedGraph<PositionedVertex>>,
    reposition_option_selected: RawOf<RepositionMode>,
    editing_mode: RawOf<GraphEditingMode>,
    new_edge_starting_vertex: Option<VertexIndex>,
    alternating_moves: bool,
    edge_creates_vertex: bool,
    details_options: DetailOptions,
    details: Option<Details>,
}

impl SnortWindow {
    pub fn new() -> SnortWindow {
        SnortWindow {
            // caterpillar C(4, 3, 4)
            game: Snort::new(UndirectedGraph::from_edges(
                &[
                    // left
                    (VertexIndex { index: 0 }, VertexIndex { index: 4 }),
                    (VertexIndex { index: 1 }, VertexIndex { index: 4 }),
                    (VertexIndex { index: 2 }, VertexIndex { index: 4 }),
                    (VertexIndex { index: 3 }, VertexIndex { index: 4 }),
                    // center
                    (VertexIndex { index: 6 }, VertexIndex { index: 5 }),
                    (VertexIndex { index: 7 }, VertexIndex { index: 5 }),
                    (VertexIndex { index: 8 }, VertexIndex { index: 5 }),
                    // right
                    (VertexIndex { index: 10 }, VertexIndex { index: 9 }),
                    (VertexIndex { index: 11 }, VertexIndex { index: 9 }),
                    (VertexIndex { index: 12 }, VertexIndex { index: 9 }),
                    (VertexIndex { index: 13 }, VertexIndex { index: 9 }),
                    // main path
                    (VertexIndex { index: 4 }, VertexIndex { index: 5 }),
                    (VertexIndex { index: 5 }, VertexIndex { index: 9 }),
                ],
                &vec![
                    PositionedVertex {
                        kind: VertexKind::Single(VertexColor::Empty),
                        position: V2f::ZERO,
                    };
                    14
                ],
            )),
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
                    ideal_spring_length: 40.0,
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

impl IsCgtWindow for TitledWindow<SnortWindow> {
    impl_titled_window!("Snort");

    fn init(&self, ctx: &Context) {
        let graph = self.content.game.graph.map(|v| v.kind);
        ctx.schedule_task(crate::Task::EvalSnort(crate::EvalTask {
            window: self.window_id,
            game: Snort::new(graph),
        }));
    }
    fn update(&mut self, update: crate::UpdateKind) {
        let graph = self.content.game.graph.map(|v| v.kind);
        match update {
            UpdateKind::SnortDetails(game, details) => {
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

        let mut graph_panel_size = V2f { x: 0.0, y: 0.0 };

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
                    self.content.game = Snort::new(UndirectedGraph::empty(&[]));
                    is_dirty = true;
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

                let [pos_x, pos_y] = ui.cursor_screen_pos();
                let control_panel_height = ui.cursor_pos()[1];

                let mut max_y = f32::NEG_INFINITY;
                let vertex_border_color = ui.style_color(StyleColor::Text);
                for this_vertex_idx in self.content.game.graph.vertex_indices() {
                    let absolute_vertex_pos =
                        self.content.game.graph.get_vertex(this_vertex_idx).position;
                    let _vertex_id = ui.push_id_usize(this_vertex_idx.index);
                    let vertex_pos @ [vertex_pos_x, vertex_pos_y] =
                        [pos_x + absolute_vertex_pos.x, pos_y + absolute_vertex_pos.y];
                    max_y = max_y.max(vertex_pos_y);
                    let button_pos @ [button_pos_x, button_pos_y] =
                        [vertex_pos_x - VERTEX_RADIUS, vertex_pos_y - VERTEX_RADIUS];
                    let button_size @ [button_size_width, button_size_height] =
                        [VERTEX_RADIUS * 2.0, VERTEX_RADIUS * 2.0];
                    ui.set_cursor_screen_pos(button_pos);

                    if ui.invisible_button("vertex", button_size) {
                        match self.content.editing_mode.as_enum() {
                            GraphEditingMode::DragVertex => { /* NOOP */ }
                            GraphEditingMode::TintVertexNone => {
                                *self
                                    .content
                                    .game
                                    .graph
                                    .get_vertex_mut(this_vertex_idx)
                                    .kind
                                    .color_mut() = VertexColor::Empty;
                                is_dirty = true;
                            }
                            GraphEditingMode::TintVertexBlue => {
                                *self
                                    .content
                                    .game
                                    .graph
                                    .get_vertex_mut(this_vertex_idx)
                                    .kind
                                    .color_mut() = VertexColor::TintLeft;
                                is_dirty = true;
                            }
                            GraphEditingMode::TintVertexRed => {
                                *self
                                    .content
                                    .game
                                    .graph
                                    .get_vertex_mut(this_vertex_idx)
                                    .kind
                                    .color_mut() = VertexColor::TintRight;
                                is_dirty = true;
                            }
                            GraphEditingMode::DeleteVertex => {
                                // We don't remove it immediately because we're just iterating over
                                // vertices
                                *self
                                    .content
                                    .game
                                    .graph
                                    .get_vertex_mut(this_vertex_idx)
                                    .kind
                                    .color_mut() = VertexColor::Taken;
                                is_dirty = true;
                            }
                            GraphEditingMode::AddEdge => { /* NOOP */ }
                            GraphEditingMode::AddVertex => { /* NOOP */ }
                            GraphEditingMode::MoveLeft => {
                                if self
                                    .content
                                    .game
                                    .available_moves_for::<{ VertexColor::TintLeft as u8 }>()
                                    .any(|v| v == this_vertex_idx)
                                {
                                    self.content.game =
                                        self.content
                                            .game
                                            .move_in_vertex::<{ VertexColor::TintLeft as u8 }>(
                                                this_vertex_idx,
                                            );
                                    if self.content.alternating_moves {
                                        self.content.editing_mode =
                                            RawOf::new(GraphEditingMode::MoveRight);
                                    }
                                }
                            }
                            GraphEditingMode::MoveRight => {
                                if self
                                    .content
                                    .game
                                    .available_moves_for::<{ VertexColor::TintRight as u8 }>()
                                    .any(|v| v == this_vertex_idx)
                                {
                                    self.content.game =
                                        self.content
                                            .game
                                            .move_in_vertex::<{ VertexColor::TintRight as u8 }>(
                                                this_vertex_idx,
                                            );
                                    if self.content.alternating_moves {
                                        self.content.editing_mode =
                                            RawOf::new(GraphEditingMode::MoveLeft);
                                    }
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

                    let [mouse_pos_x, mouse_pos_y] = ui.io().mouse_pos;
                    if !ui.io()[MouseButton::Left]
                        && mouse_pos_x >= button_pos_x
                        && mouse_pos_x <= (button_pos_x + button_size_width)
                        && mouse_pos_y >= button_pos_y
                        && mouse_pos_y <= (button_pos_y + button_size_height)
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
                        let [mouse_delta_x, mouse_delta_y] = ui.io().mouse_delta;
                        self.content
                            .game
                            .graph
                            .get_vertex_mut(this_vertex_idx)
                            .position = V2f {
                            x: f32::max(VERTEX_RADIUS, absolute_vertex_pos.x + mouse_delta_x),
                            y: f32::max(VERTEX_RADIUS, absolute_vertex_pos.y + mouse_delta_y),
                        };
                    }

                    let (vertex_fill_color, should_fill) = match self
                        .content
                        .game
                        .graph
                        .get_vertex_mut(this_vertex_idx)
                        .kind
                        .color()
                    {
                        VertexColor::Empty => (vertex_color, false),
                        VertexColor::TintLeft => {
                            (ImColor32::from_bits(0xfffb4a4e).to_rgba_f32s(), true)
                        }
                        VertexColor::TintRight => {
                            (ImColor32::from_bits(0xff7226f9).to_rgba_f32s(), true)
                        }
                        VertexColor::Taken => {
                            (ImColor32::from_bits(0xff333333).to_rgba_f32s(), true)
                        }
                    };

                    draw_list
                        .add_circle(vertex_pos, VERTEX_RADIUS, vertex_border_color)
                        .build();
                    if should_fill {
                        draw_list
                            .add_circle(vertex_pos, VERTEX_RADIUS - 0.5, vertex_fill_color)
                            .filled(true)
                            .build();
                    }

                    self.scratch_buffer.clear();
                    self.scratch_buffer
                        .write_fmt(format_args!("{}", this_vertex_idx.index + 1))
                        .unwrap();
                    let off_x = ui.calc_text_size(&self.scratch_buffer)[0];
                    draw_list.add_text(
                        [vertex_pos_x - off_x * 0.5, vertex_pos_y + VERTEX_RADIUS],
                        vertex_border_color,
                        &self.scratch_buffer,
                    );

                    for adjacent_vertex_idx in self.content.game.graph.adjacent_to(this_vertex_idx)
                    {
                        if adjacent_vertex_idx < this_vertex_idx {
                            let adjacent_pos = self
                                .content
                                .game
                                .graph
                                .get_vertex(adjacent_vertex_idx)
                                .position;

                            let adjacent_vertex_pos =
                                [pos_x + adjacent_pos.x, pos_y + adjacent_pos.y];

                            let direction = [
                                adjacent_vertex_pos[0] - vertex_pos[0],
                                adjacent_vertex_pos[1] - vertex_pos[1],
                            ];
                            let direction_len =
                                (direction[0].powi(2) + direction[1].powi(2)).sqrt();
                            let direction =
                                [direction[0] / direction_len, direction[1] / direction_len];

                            let edge_start_pos = [
                                vertex_pos[0] + direction[0] * VERTEX_RADIUS,
                                vertex_pos[1] + direction[1] * VERTEX_RADIUS,
                            ];
                            let edge_end_pos = [
                                adjacent_vertex_pos[0] - direction[0] * VERTEX_RADIUS,
                                adjacent_vertex_pos[1] - direction[1] * VERTEX_RADIUS,
                            ];

                            let distance_between_vertices = [
                                adjacent_vertex_pos[0] - vertex_pos[0],
                                adjacent_vertex_pos[1] - vertex_pos[1],
                            ];

                            if distance_between_vertices[0].abs() < 2.0 * VERTEX_RADIUS
                                && distance_between_vertices[1].abs() < 2.0 * VERTEX_RADIUS
                            {
                                continue;
                            }

                            draw_list
                                .add_line(edge_start_pos, edge_end_pos, vertex_border_color)
                                .thickness(1.0)
                                .build();
                        }
                    }
                }

                if let Some(starting_vertex) = self.content.new_edge_starting_vertex {
                    let held_vertex_pos =
                        self.content.game.graph.get_vertex(starting_vertex).position;
                    let held_vertex_pos = [pos_x + held_vertex_pos.x, pos_y + held_vertex_pos.y];
                    draw_list
                        .add_line(held_vertex_pos, ui.io().mouse_pos, vertex_border_color)
                        .thickness(2.0)
                        .build();
                }

                ui.set_cursor_screen_pos([pos_x, pos_y]);
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
                    let [mouse_x, mouse_y] = ui.io().mouse_pos;
                    self.content.game.graph.add_vertex(PositionedVertex {
                        kind: VertexKind::Single(VertexColor::Empty),
                        position: V2f {
                            x: mouse_x - pos_x,
                            y: mouse_y - pos_y,
                        },
                    });

                    is_dirty = true;
                }

                ui.set_cursor_screen_pos([pos_x, max_y + VERTEX_RADIUS]);
                ui.next_column();

                'outer: loop {
                    for to_remove_idx in self.content.game.graph.vertex_indices() {
                        let vertex = self.content.game.graph.get_vertex(to_remove_idx);
                        if vertex.kind.color() == VertexColor::Taken {
                            self.content.game.graph.remove_vertex(to_remove_idx);
                            is_dirty = true;
                            continue 'outer;
                        }
                    }
                    break;
                }

                if !ui.io()[MouseButton::Left] {
                    if let Some(edge_start) = self.content.new_edge_starting_vertex.take() {
                        if self.content.edge_creates_vertex {
                            let [mouse_x, mouse_y] = ui.io().mouse_pos;
                            self.content.game.graph.add_vertex(PositionedVertex {
                                kind: VertexKind::Single(VertexColor::Empty),
                                position: V2f {
                                    x: f32::max(VERTEX_RADIUS, mouse_x - pos_x),
                                    y: f32::max(VERTEX_RADIUS, mouse_y - pos_y),
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

                self.scratch_buffer.clear();
                self.scratch_buffer
                    .write_fmt(format_args!("Degree: {}", self.content.game.degree()))
                    .unwrap();
                ui.text(&self.scratch_buffer);

                widgets::game_details!(self, ui, draw_list);

                if is_dirty {
                    self.content.details = None;
                    let graph = self.content.game.graph.map(|v| v.kind);
                    ctx.schedule_task(Task::EvalSnort(EvalTask {
                        window: self.window_id,
                        game: Snort::new(graph),
                    }));
                }
            });

        if should_reposition {
            self.content.reposition(graph_panel_size);
        }
    }
}
