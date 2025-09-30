use crate::{
    AccessTracker, Details, EvalTask, GuiContext, IsCgtWindow, RawOf, Task, TitledWindow,
    UpdateKind, imgui_enum, impl_titled_window,
    widgets::{self, AddEdgeMode, canonical_form::CanonicalFormWindow, save_button},
};
use ::imgui::{ComboBoxFlags, Condition, Ui};
use cgt::{
    drawing::{Canvas, Draw, imgui},
    graph::{
        Graph, VertexIndex,
        adjacency_matrix::undirected::UndirectedGraph,
        layout::{CircleEdge, SpringEmbedder},
    },
    has::Has,
    impl_has,
    numeric::v2f::V2f,
    short::partizan::games::snort::{Snort, VertexColor, VertexKind},
};
use std::{fmt::Write, ops::Deref};

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

impl_has!(PositionedVertex -> kind -> VertexKind);
impl_has!(PositionedVertex -> position -> V2f);

#[derive(Debug, Clone)]
pub struct SnortWindow {
    game: AccessTracker<Snort<PositionedVertex, UndirectedGraph<PositionedVertex>>>,
    reposition_option_selected: RawOf<RepositionMode>,
    editing_mode: RawOf<GraphEditingMode>,
    alternating_moves: bool,
    add_edge_mode: AddEdgeMode,
    details: Option<Details>,
}

impl SnortWindow {
    pub fn new() -> SnortWindow {
        SnortWindow {
            // caterpillar C(4, 3, 4)
            game: AccessTracker::new(Snort::new(UndirectedGraph::from_edges(
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
            ))),
            reposition_option_selected: RawOf::new(RepositionMode::SpringEmbedder),
            editing_mode: RawOf::new(GraphEditingMode::DragVertex),
            alternating_moves: true,
            add_edge_mode: AddEdgeMode::new(),
            details: None,
        }
    }

    pub fn reposition_circle(&mut self) {
        let circle = CircleEdge {
            circle_radius: imgui::Canvas::vertex_radius()
                * (self.game.graph.size() as f32 + 4.0)
                * 0.5,
            vertex_radius: imgui::Canvas::vertex_radius(),
        };
        circle.layout(&mut self.game.get_mut_untracked().graph);
    }

    pub fn reposition(&mut self, graph_panel_size: V2f) {
        match self.reposition_option_selected.get() {
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
                            x: imgui::Canvas::vertex_radius(),
                            y: imgui::Canvas::vertex_radius(),
                        },
                        V2f {
                            x: f32::max(
                                imgui::Canvas::vertex_radius(),
                                imgui::Canvas::vertex_radius().mul_add(-2.0, graph_panel_size.x),
                            ),
                            y: f32::max(
                                imgui::Canvas::vertex_radius(),
                                imgui::Canvas::vertex_radius().mul_add(-2.0, graph_panel_size.y),
                            ),
                        },
                    )),
                };
                spring_embedder.layout(&mut self.game.get_mut_untracked().graph);
            }
        }
    }
}

impl IsCgtWindow for TitledWindow<SnortWindow> {
    impl_titled_window!("Snort");

    fn initialize(&mut self, ctx: &GuiContext) {
        self.content.reposition_circle();
        self.content.reposition(V2f { x: 350.0, y: 400.0 });
        let graph = self.content.game.graph.map(|v| v.kind);
        ctx.schedule_task(
            "Snort",
            crate::Task::EvalSnort(crate::EvalTask {
                window: self.window_id,
                game: Snort::new(graph),
            }),
        );
    }
    fn update(&mut self, update: crate::UpdateKind) {
        let graph = self.content.game.graph.map(|v| v.kind);
        if let UpdateKind::SnortDetails(game, details) = update {
            if graph == game.graph {
                self.content.details = Some(details);
            }
        }
    }

    fn draw(&mut self, ui: &Ui, ctx: &mut GuiContext) {
        let mut should_reposition = false;

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
                        }
                        if ui.menu_item("Canonical Form") {
                            if let Some(details) = self.content.details.clone() {
                                let w = CanonicalFormWindow::with_details(details);
                                ctx.new_windows
                                    .push(Box::new(TitledWindow::without_title(w)));
                            }
                        }
                    }
                    save_button(
                        ui,
                        "snort",
                        self.content.game.deref(),
                        self.content.details.as_ref().map(|d| &d.thermograph),
                    );
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
                    *self.content.game = Snort::new(UndirectedGraph::empty(&[]));
                }

                self.content
                    .editing_mode
                    .combo(ui, "Edit Mode", ComboBoxFlags::HEIGHT_LARGE);
                short_inputs.end();

                if matches!(
                    self.content.editing_mode.get(),
                    GraphEditingMode::MoveLeft | GraphEditingMode::MoveRight
                ) {
                    ui.same_line();
                    ui.checkbox("Alternating", &mut self.content.alternating_moves);
                } else if matches!(self.content.editing_mode.get(), GraphEditingMode::AddEdge) {
                    ui.same_line();
                    ui.checkbox(
                        "Add vertex",
                        &mut self.content.add_edge_mode.edge_creates_vertex,
                    );
                }

                let graph_area_position = V2f::from(ui.cursor_screen_pos());
                let graph_area_size = V2f {
                    x: ui.current_column_width(),
                    y: unsafe { ui.style().item_spacing[1] }
                        .mul_add(-2.0, ui.window_size()[1] - ui.cursor_pos()[1]),
                };
                let new_vertex_position =
                    (matches!(self.content.editing_mode.get(), GraphEditingMode::AddVertex)
                        && ui.invisible_button("Add vertex", graph_area_size))
                    .then(|| V2f::from(ui.io().mouse_pos) - graph_area_position);
                ui.set_cursor_screen_pos(graph_area_position);

                let mut canvas =
                    imgui::Canvas::new(ui, &draw_list, ctx.large_font_id, &mut self.scratch_buffer);
                self.content.game.draw(&mut canvas);

                let pressed = canvas.pressed_vertex();
                let clicked = canvas.clicked_vertex(&self.content.game.graph);
                match self.content.editing_mode.get() {
                    GraphEditingMode::DragVertex => {
                        if let Some(pressed) = pressed {
                            let delta = V2f::from(ui.io().mouse_delta);
                            let current_pos: &mut V2f = self
                                .content
                                .game
                                .get_mut_untracked()
                                .graph
                                .get_vertex_mut(pressed)
                                .get_inner_mut();
                            *current_pos += delta;
                        }
                    }
                    GraphEditingMode::TintVertexBlue => {
                        if let Some(clicked) = clicked {
                            let clicked_vertex: &mut VertexKind = self
                                .content
                                .game
                                .graph
                                .get_vertex_mut(clicked)
                                .get_inner_mut();
                            *clicked_vertex.color_mut() = VertexColor::TintLeft;
                        }
                    }
                    GraphEditingMode::TintVertexRed => {
                        if let Some(clicked) = clicked {
                            let clicked_vertex: &mut VertexKind = self
                                .content
                                .game
                                .graph
                                .get_vertex_mut(clicked)
                                .get_inner_mut();
                            *clicked_vertex.color_mut() = VertexColor::TintRight;
                        }
                    }
                    GraphEditingMode::TintVertexNone => {
                        if let Some(clicked) = clicked {
                            let clicked_vertex: &mut VertexKind = self
                                .content
                                .game
                                .graph
                                .get_vertex_mut(clicked)
                                .get_inner_mut();
                            *clicked_vertex.color_mut() = VertexColor::Empty;
                        }
                    }
                    GraphEditingMode::MoveLeft => {
                        if let Some(clicked) = clicked {
                            if self
                                .content
                                .game
                                .available_moves_for::<{ VertexColor::TintLeft as u8 }>()
                                .any(|available_vertex| available_vertex == clicked)
                            {
                                *self.content.game =
                                    self.content
                                        .game
                                        .move_in_vertex::<{ VertexColor::TintLeft as u8 }>(clicked);
                                self.content.editing_mode = RawOf::new(GraphEditingMode::MoveRight);
                            }
                        }
                    }
                    GraphEditingMode::MoveRight => {
                        if let Some(clicked) = clicked {
                            if self
                                .content
                                .game
                                .available_moves_for::<{ VertexColor::TintRight as u8 }>()
                                .any(|available_vertex| available_vertex == clicked)
                            {
                                *self.content.game =
                                    self.content
                                        .game
                                        .move_in_vertex::<{ VertexColor::TintRight as u8 }>(
                                            clicked,
                                        );
                                self.content.editing_mode = RawOf::new(GraphEditingMode::MoveLeft);
                            }
                        }
                    }
                    GraphEditingMode::AddVertex => {
                        if let Some(new_vertex_position) = new_vertex_position {
                            self.content.game.graph.add_vertex(PositionedVertex {
                                kind: VertexKind::Single(VertexColor::Empty),
                                position: new_vertex_position,
                            });
                        }
                    }
                    GraphEditingMode::DeleteVertex => {
                        if let Some(clicked) = clicked {
                            self.content.game.graph.remove_vertex(clicked);
                        }
                    }
                    GraphEditingMode::AddEdge => {
                        self.content.add_edge_mode.handle_update(
                            V2f::from(ui.io().mouse_pos),
                            graph_area_position,
                            &mut canvas,
                            &mut self.content.game.map(|game| &mut game.graph),
                            |position| PositionedVertex {
                                kind: VertexKind::Single(VertexColor::Empty),
                                position,
                            },
                        );
                    }
                }

                ui.next_column();
                self.scratch_buffer.clear();
                self.scratch_buffer
                    .write_fmt(format_args!("Degree: {}", self.content.game.degree()))
                    .unwrap();
                ui.text(&self.scratch_buffer);

                widgets::game_details(
                    self.content.details.as_ref(),
                    &mut self.scratch_buffer,
                    ui,
                    &draw_list,
                    ctx.large_font_id,
                );

                if self.content.game.clear_flag() {
                    self.content.details = None;
                    let graph = self.content.game.graph.map(|v| v.kind);
                    ctx.schedule_task(
                        "Snort",
                        Task::EvalSnort(EvalTask {
                            window: self.window_id,
                            game: Snort::new(graph),
                        }),
                    );
                }

                if should_reposition {
                    self.content.reposition(graph_area_size);
                }
            });
    }
}
