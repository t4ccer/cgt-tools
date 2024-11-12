use cgt::{
    graph::{
        adjacency_matrix::undirected::UndirectedGraph, layout::SpringEmbedder, Graph, VertexIndex,
    },
    numeric::v2f::V2f,
    short::partizan::games::snort::{self, Snort},
};
use imgui::{ComboBoxFlags, Condition, ImColor32, MouseButton, StyleColor};
use std::{f32::consts::PI, fmt::Write};

use crate::{
    imgui_enum, impl_game_window, impl_titled_window,
    widgets::{self, canonical_form::CanonicalFormWindow},
    Context, DetailOptions, Details, EvalTask, IsCgtWindow, RawOf, Task, TitledWindow, UpdateKind,
};

const SNORT_NODE_RADIUS: f32 = 16.0;

imgui_enum! {
    GraphEditingMode {
        DragNode, "Drag vertex",
        TintNodeBlue, "Tint vertex blue (left)",
        TintNodeRed, "Tint vertex red (right)",
        TintNodeNone, "Untint vertex",
        MoveLeft, "Blue move (left)",
        MoveRight, "Red move (right)",
        AddNode, "Add vertex",
        DeleteNode, "Remove vertex",
        AddEdge, "Add/Remove edge",
    }
}

imgui_enum! {
    RepositionMode {
        SpringEmbedder, "Spring Embedder",
        Circle, "Circle",
    }
}

#[derive(Debug, Clone)]
pub struct SnortWindow {
    game: Snort,
    reposition_option_selected: RawOf<RepositionMode>,
    node_positions: Vec<V2f>,
    editing_mode: RawOf<GraphEditingMode>,
    new_edge_starting_node: Option<VertexIndex>,
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
                14,
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
            )),
            node_positions: Vec::new(),
            reposition_option_selected: RawOf::new(RepositionMode::SpringEmbedder),
            editing_mode: RawOf::new(GraphEditingMode::DragNode),
            new_edge_starting_node: None,
            alternating_moves: true,
            edge_creates_vertex: true,
            details_options: DetailOptions::new(),
            details: None,
        }
    }

    pub fn reposition_circle(&mut self) {
        let n = self.game.graph.size();
        let packing_circle_radius = SNORT_NODE_RADIUS * (self.game.graph.size() as f32 + 4.0) * 0.5;
        self.node_positions.clear();
        self.node_positions.reserve(self.game.graph.size());
        for i in self.game.graph.vertices() {
            let angle = (2.0 * PI * i.index as f32) / n as f32;
            let node_pos = V2f {
                x: (packing_circle_radius - SNORT_NODE_RADIUS) * f32::cos(angle)
                    + packing_circle_radius,
                y: (packing_circle_radius - SNORT_NODE_RADIUS) * f32::sin(angle)
                    + packing_circle_radius,
            };
            self.node_positions.push(node_pos);
        }
    }

    pub fn reposition(&mut self, graph_panel_size: V2f) {
        match self.reposition_option_selected.as_enum() {
            RepositionMode::Circle => {
                self.reposition_circle();
            }
            RepositionMode::SpringEmbedder => {
                let spring_embedder = SpringEmbedder {
                    cooling_rate: 0.999,
                    c_attractive: 1.0,
                    c_repulsive: 250.0,
                    ideal_spring_length: 40.0,
                    iterations: 1024,
                    bounds: Some((
                        V2f {
                            x: SNORT_NODE_RADIUS,
                            y: SNORT_NODE_RADIUS,
                        },
                        V2f {
                            x: f32::max(
                                SNORT_NODE_RADIUS,
                                graph_panel_size.x - SNORT_NODE_RADIUS * 2.0,
                            ),
                            y: f32::max(
                                SNORT_NODE_RADIUS,
                                graph_panel_size.y - SNORT_NODE_RADIUS * 2.0,
                            ),
                        },
                    )),
                };
                spring_embedder.layout(&self.game.graph, &mut self.node_positions);
            }
        }
    }
}

impl IsCgtWindow for TitledWindow<SnortWindow> {
    impl_titled_window!("Snort");
    impl_game_window!(EvalSnort, SnortDetails);

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
                let node_color = ui.style_color(StyleColor::Text);
                for this_vertex_idx in self.content.game.graph.vertices() {
                    let absolute_node_pos = self.content.node_positions[this_vertex_idx.index];
                    let _node_id = ui.push_id_usize(this_vertex_idx.index);
                    let node_pos @ [node_pos_x, node_pos_y] =
                        [pos_x + absolute_node_pos.x, pos_y + absolute_node_pos.y];
                    max_y = max_y.max(node_pos_y);
                    let button_pos @ [button_pos_x, button_pos_y] = [
                        node_pos_x - SNORT_NODE_RADIUS,
                        node_pos_y - SNORT_NODE_RADIUS,
                    ];
                    let button_size @ [button_size_width, button_size_height] =
                        [SNORT_NODE_RADIUS * 2.0, SNORT_NODE_RADIUS * 2.0];
                    ui.set_cursor_screen_pos(button_pos);

                    if ui.invisible_button("node", button_size) {
                        match self.content.editing_mode.as_enum() {
                            GraphEditingMode::DragNode => { /* NOOP */ }
                            GraphEditingMode::TintNodeNone => {
                                *self.content.game.vertices[this_vertex_idx].color_mut() =
                                    snort::VertexColor::Empty;
                                is_dirty = true;
                            }
                            GraphEditingMode::TintNodeBlue => {
                                *self.content.game.vertices[this_vertex_idx].color_mut() =
                                    snort::VertexColor::TintLeft;
                                is_dirty = true;
                            }
                            GraphEditingMode::TintNodeRed => {
                                *self.content.game.vertices[this_vertex_idx].color_mut() =
                                    snort::VertexColor::TintRight;
                                is_dirty = true;
                            }
                            GraphEditingMode::DeleteNode => {
                                // We don't remove it immediately because we're just iterating over
                                // vertices
                                *self.content.game.vertices[this_vertex_idx].color_mut() =
                                    snort::VertexColor::Taken;
                                is_dirty = true;
                            }
                            GraphEditingMode::AddEdge => { /* NOOP */ }
                            GraphEditingMode::AddNode => { /* NOOP */ }
                            GraphEditingMode::MoveLeft => {
                                if matches!(
                                    self.content.game.vertices[this_vertex_idx].color(),
                                    snort::VertexColor::TintLeft | snort::VertexColor::Empty
                                ) {
                                    for adjacent in
                                        self.content.game.graph.adjacent_to(this_vertex_idx)
                                    {
                                        if matches!(
                                            self.content.game.vertices[adjacent].color(),
                                            snort::VertexColor::Taken
                                                | snort::VertexColor::TintRight
                                        ) {
                                            *self.content.game.vertices[adjacent].color_mut() =
                                                snort::VertexColor::Taken;
                                        } else {
                                            *self.content.game.vertices[adjacent].color_mut() =
                                                snort::VertexColor::TintLeft;
                                        }
                                    }
                                    *self.content.game.vertices[this_vertex_idx].color_mut() =
                                        snort::VertexColor::Taken;
                                    if self.content.alternating_moves {
                                        self.content.editing_mode =
                                            RawOf::new(GraphEditingMode::MoveRight);
                                    }
                                    is_dirty = true;
                                }
                            }
                            GraphEditingMode::MoveRight => {
                                if matches!(
                                    self.content.game.vertices[this_vertex_idx].color(),
                                    snort::VertexColor::TintRight | snort::VertexColor::Empty
                                ) {
                                    for adjacent in
                                        self.content.game.graph.adjacent_to(this_vertex_idx)
                                    {
                                        if matches!(
                                            self.content.game.vertices[adjacent].color(),
                                            snort::VertexColor::Taken
                                                | snort::VertexColor::TintLeft
                                        ) {
                                            *self.content.game.vertices[adjacent].color_mut() =
                                                snort::VertexColor::Taken;
                                        } else {
                                            *self.content.game.vertices[adjacent].color_mut() =
                                                snort::VertexColor::TintRight;
                                        }
                                    }
                                    *self.content.game.vertices[this_vertex_idx].color_mut() =
                                        snort::VertexColor::Taken;
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
                        self.content.new_edge_starting_node = Some(this_vertex_idx);
                    }

                    let [mouse_pos_x, mouse_pos_y] = ui.io().mouse_pos;
                    if !ui.io()[MouseButton::Left]
                        && mouse_pos_x >= button_pos_x
                        && mouse_pos_x <= (button_pos_x + button_size_width)
                        && mouse_pos_y >= button_pos_y
                        && mouse_pos_y <= (button_pos_y + button_size_height)
                    {
                        if let Some(starting_node) = self.content.new_edge_starting_node.take() {
                            if starting_node != this_vertex_idx {
                                self.content.game.graph.connect(
                                    starting_node,
                                    this_vertex_idx,
                                    !self
                                        .content
                                        .game
                                        .graph
                                        .are_adjacent(starting_node, this_vertex_idx),
                                );
                                is_dirty = true;
                            }
                        }
                    }

                    if ui.is_item_active()
                        && matches!(
                            self.content.editing_mode.as_enum(),
                            GraphEditingMode::DragNode
                        )
                    {
                        let [mouse_delta_x, mouse_delta_y] = ui.io().mouse_delta;
                        self.content.node_positions[this_vertex_idx.index] = V2f {
                            x: f32::max(SNORT_NODE_RADIUS, absolute_node_pos.x + mouse_delta_x),
                            y: f32::max(SNORT_NODE_RADIUS, absolute_node_pos.y + mouse_delta_y),
                        };
                    }

                    let (node_fill_color, should_fill) =
                        match self.content.game.vertices[this_vertex_idx].color() {
                            snort::VertexColor::Empty => (node_color, false),
                            snort::VertexColor::TintLeft => {
                                (ImColor32::from_bits(0xfffb4a4e).to_rgba_f32s(), true)
                            }
                            snort::VertexColor::TintRight => {
                                (ImColor32::from_bits(0xff7226f9).to_rgba_f32s(), true)
                            }
                            snort::VertexColor::Taken => {
                                (ImColor32::from_bits(0xff333333).to_rgba_f32s(), true)
                            }
                        };

                    draw_list
                        .add_circle(node_pos, SNORT_NODE_RADIUS, node_color)
                        .build();
                    if should_fill {
                        draw_list
                            .add_circle(node_pos, SNORT_NODE_RADIUS - 0.5, node_fill_color)
                            .filled(true)
                            .build();
                    }

                    self.scratch_buffer.clear();
                    self.scratch_buffer
                        .write_fmt(format_args!("{}", this_vertex_idx.index + 1))
                        .unwrap();
                    let off_x = ui.calc_text_size(&self.scratch_buffer)[0];
                    draw_list.add_text(
                        [node_pos_x - off_x * 0.5, node_pos_y + SNORT_NODE_RADIUS],
                        node_color,
                        &self.scratch_buffer,
                    );

                    for adjacent_vertex_idx in self.content.game.graph.adjacent_to(this_vertex_idx)
                    {
                        if adjacent_vertex_idx < this_vertex_idx {
                            let adjacent_pos =
                                self.content.node_positions[adjacent_vertex_idx.index];
                            let adjacent_pos = [pos_x + adjacent_pos.x, pos_y + adjacent_pos.y];
                            draw_list
                                .add_line(node_pos, adjacent_pos, node_color)
                                .thickness(1.0)
                                .build();
                        }
                    }
                }

                if let Some(starting_node) = self.content.new_edge_starting_node {
                    let held_node_pos = self.content.node_positions[starting_node.index];
                    let held_node_pos = [pos_x + held_node_pos.x, pos_y + held_node_pos.y];
                    draw_list
                        .add_line(held_node_pos, ui.io().mouse_pos, ImColor32::BLACK)
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
                    GraphEditingMode::AddNode
                ) && ui.invisible_button("Add node", graph_panel_size)
                {
                    self.content.game.graph.add_vertex();
                    self.content
                        .game
                        .vertices
                        .inner
                        .push(snort::VertexKind::Single(snort::VertexColor::Empty));

                    let [mouse_x, mouse_y] = ui.io().mouse_pos;
                    self.content.node_positions.push(V2f {
                        x: mouse_x - pos_x,
                        y: mouse_y - pos_y,
                    });
                    is_dirty = true;
                }

                ui.set_cursor_screen_pos([pos_x, max_y + SNORT_NODE_RADIUS]);
                ui.next_column();

                'outer: loop {
                    for (to_remove, color) in
                        self.content.game.vertices.inner.iter().copied().enumerate()
                    {
                        if color.color() == snort::VertexColor::Taken {
                            self.content
                                .game
                                .graph
                                .remove_vertex(VertexIndex { index: to_remove });
                            self.content.game.vertices.inner.remove(to_remove);
                            self.content.node_positions.remove(to_remove);
                            is_dirty = true;
                            continue 'outer;
                        }
                    }
                    break;
                }

                if !ui.io()[MouseButton::Left] {
                    if let Some(edge_start) = self.content.new_edge_starting_node.take() {
                        if self.content.edge_creates_vertex {
                            self.content.game.graph.add_vertex();
                            self.content
                                .game
                                .vertices
                                .inner
                                .push(snort::VertexKind::Single(snort::VertexColor::Empty));

                            let [mouse_x, mouse_y] = ui.io().mouse_pos;
                            self.content.node_positions.push(V2f {
                                x: f32::max(SNORT_NODE_RADIUS, mouse_x - pos_x),
                                y: f32::max(SNORT_NODE_RADIUS, mouse_y - pos_y),
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
                    ctx.schedule_task(Task::EvalSnort(EvalTask {
                        window: self.window_id,
                        game: self.content.game.clone(),
                    }));
                }
            });

        if should_reposition {
            self.content.reposition(graph_panel_size);
        }
    }
}
