use cgt::{
    graph,
    short::partizan::{
        games::snort::{self, Snort},
        partizan_game::PartizanGame,
        transposition_table::ParallelTranspositionTable,
    },
};
use imgui::{Condition, ImColor32, MouseButton, StyleColor};
use std::{borrow::Cow, f32::consts::PI, fmt::Write};

use crate::{imgui_enum, widgets, Details, IsEnum, RawOf, WindowId};

const SNORT_NODE_RADIUS: f32 = 16.0;

imgui_enum! {
    GraphEditingMode {
        DragNode, 0, "Drag vertex",
        TintNodeBlue, 1, "Tint vertex blue (left)",
        TintNodeRed, 2, "Tint vertex red (right)",
        TintNodeNone, 3, "Untint vertex",
        DeleteNode, 4, "Remove vertex",
        AddEdge, 5, "Add/Remove edge",
        AddNode, 6, "Add vertex",
    }
}

imgui_enum! {
    RepositionMode {
        Circle, 0, "Circle",
        FDP, 1, "FDP (Not Implemented Yet)",
    }
}

pub struct SnortWindow<'tt> {
    title: String,
    is_open: bool,
    game: Snort,
    reposition_option_selected: RawOf<RepositionMode>,

    #[allow(dead_code)]
    transposition_table: &'tt ParallelTranspositionTable<Snort>,
    node_positions: Vec<[f32; 2]>,
    editing_mode: RawOf<GraphEditingMode>,
    new_edge_starting_node: Option<usize>,
    details: Option<Details>,
    show_thermograph: bool,
    thermograph_scale: f32,
}

impl<'tt> SnortWindow<'tt> {
    pub fn new(id: WindowId, snort_tt: &'tt ParallelTranspositionTable<Snort>) -> SnortWindow<'tt> {
        SnortWindow {
            title: format!("Snort##{}", id.0),
            is_open: true,
            // caterpillar C(4, 3, 4)
            game: Snort::new(graph::undirected::Graph::from_edges(
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
            transposition_table: &snort_tt,
            node_positions: Vec::new(),
            reposition_option_selected: RawOf::new(RepositionMode::Circle),
            editing_mode: RawOf::new(GraphEditingMode::DragNode),
            new_edge_starting_node: None,
            details: None,
            show_thermograph: true,
            thermograph_scale: 20.0,
        }
    }

    pub fn reposition_circle(&mut self) {
        let n = self.game.graph.size();
        let packing_circle_radius = SNORT_NODE_RADIUS * (self.game.graph.size() as f32 + 4.0) * 0.5;
        self.node_positions.clear();
        self.node_positions.reserve(self.game.graph.size());
        for i in 0..n {
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

    pub fn draw(&mut self, ui: &imgui::Ui) {
        if !self.is_open {
            return;
        }

        let mut should_reposition = false;
        let mut to_remove: Option<usize> = None;
        let mut is_dirty = false;
        let mut label_buf = String::new();

        ui.window(&self.title)
            .position([50.0, 50.0], Condition::Appearing)
            .size([750.0, 450.0], Condition::Appearing)
            .bring_to_front_on_focus(true)
            .opened(&mut self.is_open)
            .build(|| {
                let draw_list = ui.get_window_draw_list();

                ui.columns(2, "columns", true);

                let short_inputs = ui.push_item_width(200.0);
                ui.combo(
                    "##Reposition Mode",
                    &mut self.reposition_option_selected.value,
                    RepositionMode::LABELS,
                    |i| Cow::Borrowed(i),
                );
                ui.same_line();
                should_reposition = ui.button("Reposition");

                ui.combo(
                    "Edit Mode",
                    &mut self.editing_mode.value,
                    GraphEditingMode::LABELS,
                    |i| Cow::Borrowed(i),
                );
                short_inputs.end();

                let [pos_x, pos_y] = ui.cursor_screen_pos();
                let off_y = ui.cursor_pos()[1];

                let mut max_y = f32::NEG_INFINITY;
                let node_color = ui.style_color(StyleColor::Text);
                for this_vertex_idx in 0..self.game.graph.size() {
                    let [absolute_node_pos_x, absolute_node_pos_y] =
                        self.node_positions[this_vertex_idx];
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
                        match self.editing_mode.as_enum() {
                            GraphEditingMode::DragNode => { /* NOOP */ }
                            GraphEditingMode::TintNodeNone => {
                                *self.game.vertices[this_vertex_idx].color_mut() =
                                    snort::VertexColor::Empty;
                                is_dirty = true;
                            }
                            GraphEditingMode::TintNodeBlue => {
                                *self.game.vertices[this_vertex_idx].color_mut() =
                                    snort::VertexColor::TintLeft;
                                is_dirty = true;
                            }
                            GraphEditingMode::TintNodeRed => {
                                *self.game.vertices[this_vertex_idx].color_mut() =
                                    snort::VertexColor::TintRight;
                                is_dirty = true;
                            }
                            GraphEditingMode::DeleteNode => {
                                // We don't remove it immediately because we're just iterating over
                                // vertices
                                to_remove = Some(this_vertex_idx);
                                is_dirty = true;
                            }
                            GraphEditingMode::AddEdge => { /* NOOP */ }
                            GraphEditingMode::AddNode => { /* NOOP */ }
                        }
                    };

                    if ui.is_item_activated()
                        && matches!(self.editing_mode.as_enum(), GraphEditingMode::AddEdge)
                    {
                        self.new_edge_starting_node = Some(this_vertex_idx);
                    }

                    let [mouse_pos_x, mouse_pos_y] = ui.io().mouse_pos;
                    if !ui.io()[MouseButton::Left]
                        && mouse_pos_x >= button_pos_x
                        && mouse_pos_x <= (button_pos_x + button_size_width)
                        && mouse_pos_y >= button_pos_y
                        && mouse_pos_y <= (button_pos_y + button_size_height)
                    {
                        if let Some(starting_node) = self.new_edge_starting_node.take() {
                            if starting_node != this_vertex_idx {
                                self.game.graph.connect(
                                    starting_node,
                                    this_vertex_idx,
                                    !self.game.graph.are_adjacent(starting_node, this_vertex_idx),
                                );
                                is_dirty = true;
                            }
                        }
                    }

                    if ui.is_item_active()
                        && matches!(self.editing_mode.as_enum(), GraphEditingMode::DragNode)
                    {
                        let [mouse_delta_x, mouse_delta_y] = ui.io().mouse_delta;
                        self.node_positions[this_vertex_idx] = [
                            f32::max(SNORT_NODE_RADIUS, absolute_node_pos_x + mouse_delta_x),
                            f32::max(SNORT_NODE_RADIUS, absolute_node_pos_y + mouse_delta_y),
                        ];
                    }

                    let (node_fill_color, should_fill) =
                        match self.game.vertices[this_vertex_idx].color() {
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

                    label_buf.clear();
                    label_buf
                        .write_fmt(format_args!("{}", this_vertex_idx + 1))
                        .unwrap();
                    let off_x = ui.calc_text_size(&label_buf)[0];
                    draw_list.add_text(
                        [node_pos_x - off_x * 0.5, node_pos_y + SNORT_NODE_RADIUS],
                        node_color,
                        &label_buf,
                    );

                    for adjacent_vertex_idx in self.game.graph.adjacent_to(this_vertex_idx) {
                        if adjacent_vertex_idx < this_vertex_idx {
                            let [adjacent_pos_x, adjacent_pos_y] =
                                self.node_positions[adjacent_vertex_idx];
                            let adjacent_pos = [pos_x + adjacent_pos_x, pos_y + adjacent_pos_y];
                            draw_list
                                .add_line(node_pos, adjacent_pos, node_color)
                                .thickness(1.0)
                                .build();
                        }
                    }
                }

                if let Some(starting_node) = self.new_edge_starting_node {
                    let [held_node_pos_x, held_node_pos_y] = self.node_positions[starting_node];
                    let held_node_pos = [pos_x + held_node_pos_x, pos_y + held_node_pos_y];
                    draw_list
                        .add_line(held_node_pos, ui.io().mouse_pos, ImColor32::BLACK)
                        .thickness(2.0)
                        .build();
                }

                ui.set_cursor_screen_pos([pos_x, pos_y]);
                if matches!(self.editing_mode.as_enum(), GraphEditingMode::AddNode)
                    && ui.invisible_button(
                        "Graph background",
                        [ui.current_column_width(), ui.window_size()[1] - off_y],
                    )
                {
                    self.game.graph.add_vertex();
                    self.game
                        .vertices
                        .push(snort::VertexKind::Single(snort::VertexColor::Empty));

                    let [mouse_x, mouse_y] = ui.io().mouse_pos;
                    self.node_positions.push([mouse_x - pos_x, mouse_y - pos_y]);
                    is_dirty = true;
                }

                ui.set_cursor_screen_pos([pos_x, max_y + SNORT_NODE_RADIUS]);
                ui.next_column();

                if let Some(to_remove) = to_remove.take() {
                    self.game.graph.remove_vertex(to_remove);
                    self.game.vertices.remove(to_remove);
                    self.node_positions.remove(to_remove);
                    is_dirty = true;
                }

                if is_dirty {
                    self.details = None;
                }

                // TODO: Worker thread
                if self.details.is_none() {
                    let canonical_form = self.game.canonical_form(self.transposition_table);
                    self.details = Some(Details::from_canonical_form(canonical_form));
                }

                if let Some(details) = self.details.as_ref() {
                    ui.text_wrapped(&details.canonical_form_rendered);
                    ui.text_wrapped(&details.temperature_rendered);

                    ui.checkbox("Thermograph:", &mut self.show_thermograph);
                    if self.show_thermograph {
                        ui.align_text_to_frame_padding();
                        ui.text("Scale: ");
                        ui.same_line();
                        let short_slider = ui.push_item_width(200.0);
                        ui.slider("##1", 5.0, 100.0, &mut self.thermograph_scale);
                        short_slider.end();
                        widgets::thermograph(
                            ui,
                            &draw_list,
                            self.thermograph_scale,
                            &details.thermograph,
                        );
                    }
                }
            });

        if should_reposition {
            match self.reposition_option_selected.as_enum() {
                RepositionMode::Circle => self.reposition_circle(),
                RepositionMode::FDP => { /* TODO */ }
            }
        }

        if !ui.io()[MouseButton::Left] {
            self.new_edge_starting_node = None;
        }
    }
}
