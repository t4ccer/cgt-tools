use crate::{
    imgui_enum, impl_titled_window,
    widgets::{
        self, canonical_form::CanonicalFormWindow, GraphEditor, VertexFillColor, COLOR_BLUE,
        COLOR_RED, VERTEX_RADIUS,
    },
    DetailOptions, Details, EvalTask, GuiContext, IsCgtWindow, RawOf, Task, TitledWindow,
    UpdateKind,
};
use cgt::{
    graph::{
        adjacency_matrix::undirected::UndirectedGraph,
        layout::{CircleEdge, SpringEmbedder},
        Graph, VertexIndex,
    },
    has::Has,
    numeric::v2f::V2f,
    short::partizan::games::snort::{Snort, VertexColor, VertexKind},
};
use imgui::{ComboBoxFlags, Condition, ImColor32};
use std::fmt::Write;

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

impl Has<V2f> for PositionedVertex {
    fn get_inner(&self) -> &V2f {
        &self.position
    }

    fn get_inner_mut(&mut self) -> &mut V2f {
        &mut self.position
    }
}

impl VertexFillColor for PositionedVertex {
    fn fill_color(&self) -> ImColor32 {
        match self.kind.color() {
            VertexColor::TintLeft => COLOR_BLUE,
            VertexColor::TintRight => COLOR_RED,
            VertexColor::Empty => ImColor32::TRANSPARENT,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SnortWindow {
    game: Snort<PositionedVertex, UndirectedGraph<PositionedVertex>>,
    reposition_option_selected: RawOf<RepositionMode>,
    editing_mode: RawOf<GraphEditingMode>,
    alternating_moves: bool,
    edge_creates_vertex: bool,
    details_options: DetailOptions,
    details: Option<Details>,
    graph_editor: GraphEditor,
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
            alternating_moves: true,
            edge_creates_vertex: true,
            details_options: DetailOptions::new(),
            details: None,
            graph_editor: GraphEditor::new(),
        }
    }

    pub fn reposition_circle(&mut self) {
        let circle = CircleEdge {
            circle_radius: VERTEX_RADIUS * (self.game.graph.size() as f32 + 4.0) * 0.5,
            vertex_radius: VERTEX_RADIUS,
        };
        circle.layout(&mut self.game.graph);
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
                            x: VERTEX_RADIUS,
                            y: VERTEX_RADIUS,
                        },
                        V2f {
                            x: f32::max(
                                VERTEX_RADIUS,
                                VERTEX_RADIUS.mul_add(-2.0, graph_panel_size.x),
                            ),
                            y: f32::max(
                                VERTEX_RADIUS,
                                VERTEX_RADIUS.mul_add(-2.0, graph_panel_size.y),
                            ),
                        },
                    )),
                };
                spring_embedder.layout(&mut self.game.graph);
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

    fn draw(&mut self, ui: &imgui::Ui, ctx: &mut GuiContext) {
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
                    self.content.editing_mode.get(),
                    GraphEditingMode::MoveLeft | GraphEditingMode::MoveRight
                ) {
                    ui.same_line();
                    ui.checkbox("Alternating", &mut self.content.alternating_moves);
                } else if matches!(self.content.editing_mode.get(), GraphEditingMode::AddEdge) {
                    ui.same_line();
                    ui.checkbox("Add vertex", &mut self.content.edge_creates_vertex);
                }

                let action = self.content.graph_editor.draw(
                    ui,
                    &draw_list,
                    matches!(self.content.editing_mode.get(), GraphEditingMode::AddEdge),
                    matches!(self.content.editing_mode.get(), GraphEditingMode::AddVertex),
                    matches!(
                        self.content.editing_mode.get(),
                        GraphEditingMode::DragVertex
                    ),
                    self.content.edge_creates_vertex,
                    &mut self.scratch_buffer,
                    &mut self.content.game.graph,
                );
                match action {
                    widgets::GraphEditorAction::None => {}
                    widgets::GraphEditorAction::VertexClick(clicked_vertex) => {
                        match self.content.editing_mode.get() {
                            GraphEditingMode::TintVertexBlue => {
                                *self
                                    .content
                                    .game
                                    .graph
                                    .get_vertex_mut(clicked_vertex)
                                    .kind
                                    .color_mut() = VertexColor::TintLeft;
                                is_dirty = true;
                            }
                            GraphEditingMode::TintVertexRed => {
                                *self
                                    .content
                                    .game
                                    .graph
                                    .get_vertex_mut(clicked_vertex)
                                    .kind
                                    .color_mut() = VertexColor::TintRight;
                                is_dirty = true;
                            }
                            GraphEditingMode::TintVertexNone => {
                                *self
                                    .content
                                    .game
                                    .graph
                                    .get_vertex_mut(clicked_vertex)
                                    .kind
                                    .color_mut() = VertexColor::Empty;
                                is_dirty = true;
                            }
                            GraphEditingMode::MoveLeft => {
                                if self
                                    .content
                                    .game
                                    .available_moves_for::<{ VertexColor::TintLeft as u8 }>()
                                    .any(|available_vertex| available_vertex == clicked_vertex)
                                {
                                    self.content.game =
                                        self.content
                                            .game
                                            .move_in_vertex::<{ VertexColor::TintLeft as u8 }>(
                                                clicked_vertex,
                                            );
                                    self.content.editing_mode =
                                        RawOf::new(GraphEditingMode::MoveRight);
                                    is_dirty = true;
                                }
                            }
                            GraphEditingMode::MoveRight => {
                                if self
                                    .content
                                    .game
                                    .available_moves_for::<{ VertexColor::TintRight as u8 }>()
                                    .any(|available_vertex| available_vertex == clicked_vertex)
                                {
                                    self.content.game =
                                        self.content
                                            .game
                                            .move_in_vertex::<{ VertexColor::TintRight as u8 }>(
                                                clicked_vertex,
                                            );
                                    self.content.editing_mode =
                                        RawOf::new(GraphEditingMode::MoveLeft);
                                    is_dirty = true;
                                }
                            }
                            GraphEditingMode::DeleteVertex => {
                                self.content.game.graph.remove_vertex(clicked_vertex);
                                is_dirty = true;
                            }
                            GraphEditingMode::AddEdge
                            | GraphEditingMode::AddVertex
                            | GraphEditingMode::DragVertex => {}
                        }
                    }
                    widgets::GraphEditorAction::NewVertex(position, connection) => {
                        match self.content.editing_mode.get() {
                            GraphEditingMode::AddEdge | GraphEditingMode::AddVertex => {
                                let kind = VertexKind::Single(VertexColor::Empty);
                                self.content
                                    .game
                                    .graph
                                    .add_vertex(PositionedVertex { kind, position });
                                if let Some(from) = connection {
                                    self.content.game.graph.connect(
                                        from,
                                        VertexIndex {
                                            index: self.content.game.graph.size() - 1,
                                        },
                                        true,
                                    );
                                }
                                is_dirty = true;
                            }
                            _ => {}
                        }
                    }
                }

                ui.next_column();
                self.scratch_buffer.clear();
                self.scratch_buffer
                    .write_fmt(format_args!("Degree: {}", self.content.game.degree()))
                    .unwrap();
                ui.text(&self.scratch_buffer);

                widgets::game_details!(self, ui, draw_list);

                if is_dirty {
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
            });

        if should_reposition {
            self.content
                .reposition(self.content.graph_editor.graph_panel_size);
        }
    }
}
