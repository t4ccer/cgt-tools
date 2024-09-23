use cgt::{
    graph,
    short::partizan::games::snort::{self, Snort},
};
use imgui::{Condition, ImColor32, MouseButton, StyleColor};
use std::{borrow::Cow, f32::consts::PI, fmt::Write};

use crate::{
    imgui_enum, impl_game_window, impl_titled_window,
    widgets::{self, canonical_form::CanonicalFormWindow},
    Context, Details, EvalTask, IsCgtWindow, IsEnum, RawOf, Task, TitledWindow, UpdateKind,
};

const SNORT_NODE_RADIUS: f32 = 16.0;

imgui_enum! {
    GraphEditingMode {
        DragNode, 0, "Drag vertex",
        TintNodeBlue, 1, "Tint vertex blue (left)",
        TintNodeRed, 2, "Tint vertex red (right)",
        TintNodeNone, 3, "Untint vertex",
        MoveLeft, 4, "Blue move (left)",
        MoveRight, 5, "Red move (right)",
        AddNode, 6, "Add vertex",
        DeleteNode, 7, "Remove vertex",
        AddEdge, 8, "Add/Remove edge",
    }
}

imgui_enum! {
    RepositionMode {
        Circle, 0, "Circle",
        FDP, 1, "FDP (Not Implemented Yet)",
    }
}

#[derive(Debug, Clone)]
pub struct SnortWindow {
    game: Snort,
    reposition_option_selected: RawOf<RepositionMode>,
    node_positions: Vec<[f32; 2]>,
    editing_mode: RawOf<GraphEditingMode>,
    new_edge_starting_node: Option<usize>,
    pub details: Option<Details>,
    show_thermograph: bool,
    thermograph_scale: f32,
    alternating_moves: bool,
    edge_creates_vertex: bool,
}

impl SnortWindow {
    pub fn new() -> SnortWindow {
        SnortWindow {
            // caterpillar C(4, 3, 4)
            game: Snort::new(graph::undirected::UndirectedGraph::from_edges(
                14,
                &[
                    // left
                    (0, 4),
                    (1, 4),
                    (2, 4),
                    (3, 4),
                    // center
                    (6, 5),
                    (7, 5),
                    (8, 5),
                    // right
                    (10, 9),
                    (11, 9),
                    (12, 9),
                    (13, 9),
                    // main path
                    (4, 5),
                    (5, 9),
                ],
            )),
            node_positions: Vec::new(),
            reposition_option_selected: RawOf::new(RepositionMode::Circle),
            editing_mode: RawOf::new(GraphEditingMode::DragNode),
            new_edge_starting_node: None,
            details: None,
            show_thermograph: true,
            thermograph_scale: 20.0,
            alternating_moves: true,
            edge_creates_vertex: true,
        }
    }

    pub fn reposition_circle(&mut self) {
        let n = self.game.graph.size();
        let packing_circle_radius = SNORT_NODE_RADIUS * (self.game.graph.size() as f32 + 4.0) * 0.5;
        self.node_positions.clear();
        self.node_positions.reserve(self.game.graph.size());
        for i in self.game.graph.vertices() {
            let angle = (2.0 * PI * i as f32) / n as f32;
            let node_pos = [
                (packing_circle_radius - SNORT_NODE_RADIUS) * f32::cos(angle)
                    + packing_circle_radius,
                (packing_circle_radius - SNORT_NODE_RADIUS) * f32::sin(angle)
                    + packing_circle_radius,
            ];
            self.node_positions.push(node_pos);
        }
    }
}

impl IsCgtWindow for TitledWindow<SnortWindow> {
    impl_titled_window!("Snort");
    impl_game_window!(EvalSnort, SnortDetails);

    fn draw(&mut self, ui: &imgui::Ui, ctx: &mut Context) {
        let mut should_reposition = false;
        let mut is_dirty = false;

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
                ui.combo(
                    "##Reposition Mode",
                    &mut self.content.reposition_option_selected.value,
                    RepositionMode::LABELS,
                    |i| Cow::Borrowed(i),
                );
                ui.same_line();
                should_reposition = ui.button("Reposition");

                ui.combo(
                    "Edit Mode",
                    &mut self.content.editing_mode.value,
                    GraphEditingMode::LABELS,
                    |i| Cow::Borrowed(i),
                );
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
                    let [absolute_node_pos_x, absolute_node_pos_y] =
                        self.content.node_positions[this_vertex_idx];
                    let _node_id = ui.push_id_usize(this_vertex_idx as usize);
                    let node_pos @ [node_pos_x, node_pos_y] =
                        [pos_x + absolute_node_pos_x, pos_y + absolute_node_pos_y];
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
                        self.content.node_positions[this_vertex_idx] = [
                            f32::max(SNORT_NODE_RADIUS, absolute_node_pos_x + mouse_delta_x),
                            f32::max(SNORT_NODE_RADIUS, absolute_node_pos_y + mouse_delta_y),
                        ];
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
                        .write_fmt(format_args!("{}", this_vertex_idx + 1))
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
                            let [adjacent_pos_x, adjacent_pos_y] =
                                self.content.node_positions[adjacent_vertex_idx];
                            let adjacent_pos = [pos_x + adjacent_pos_x, pos_y + adjacent_pos_y];
                            draw_list
                                .add_line(node_pos, adjacent_pos, node_color)
                                .thickness(1.0)
                                .build();
                        }
                    }
                }

                if let Some(starting_node) = self.content.new_edge_starting_node {
                    let [held_node_pos_x, held_node_pos_y] =
                        self.content.node_positions[starting_node];
                    let held_node_pos = [pos_x + held_node_pos_x, pos_y + held_node_pos_y];
                    draw_list
                        .add_line(held_node_pos, ui.io().mouse_pos, ImColor32::BLACK)
                        .thickness(2.0)
                        .build();
                }

                ui.set_cursor_screen_pos([pos_x, pos_y]);
                if matches!(
                    self.content.editing_mode.as_enum(),
                    GraphEditingMode::AddNode
                ) && ui.invisible_button(
                    "Add node",
                    [
                        ui.current_column_width(),
                        ui.window_size()[1] - control_panel_height,
                    ],
                ) {
                    self.content.game.graph.add_vertex();
                    self.content
                        .game
                        .vertices
                        .push(snort::VertexKind::Single(snort::VertexColor::Empty));

                    let [mouse_x, mouse_y] = ui.io().mouse_pos;
                    self.content
                        .node_positions
                        .push([mouse_x - pos_x, mouse_y - pos_y]);
                    is_dirty = true;
                }

                ui.set_cursor_screen_pos([pos_x, max_y + SNORT_NODE_RADIUS]);
                ui.next_column();

                'outer: loop {
                    for (to_remove, color) in self.content.game.vertices.iter().copied().enumerate()
                    {
                        if color.color() == snort::VertexColor::Taken {
                            self.content.game.graph.remove_vertex(to_remove);
                            self.content.game.vertices.remove(to_remove);
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
                                .push(snort::VertexKind::Single(snort::VertexColor::Empty));

                            let [mouse_x, mouse_y] = ui.io().mouse_pos;
                            self.content.node_positions.push([
                                f32::max(SNORT_NODE_RADIUS, mouse_x - pos_x),
                                f32::max(SNORT_NODE_RADIUS, mouse_y - pos_y),
                            ]);
                            let edge_end = self.content.game.graph.size() - 1;
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
            match self.content.reposition_option_selected.as_enum() {
                RepositionMode::Circle => self.content.reposition_circle(),
                RepositionMode::FDP => { /* TODO */ }
            }
        }
    }
}
