use crate::{
    Details, EvalTask, GuiContext, IsCgtWindow, RawOf, Task, TitledWindow, UpdateKind, imgui_enum,
    impl_titled_window,
    widgets::{self, canonical_form::CanonicalFormWindow, save_button},
};
use ::imgui::{ComboBoxFlags, Condition, Ui};
use cgt::{
    drawing::{Canvas, Color, Draw, imgui},
    graph::{
        Graph, VertexIndex,
        adjacency_matrix::directed::DirectedGraph,
        layout::{CircleEdge, SpringEmbedder},
    },
    has::Has,
    impl_has,
    numeric::v2f::V2f,
    short::partizan::{
        Player,
        games::digraph_placement::{DigraphPlacement, VertexColor},
    },
};

imgui_enum! {
    NewVertexColor {
        Left, "Blue",
        Right, "Red",
    }
}

impl From<RawOf<NewVertexColor>> for VertexColor {
    fn from(new_vertex_color: RawOf<NewVertexColor>) -> VertexColor {
        match new_vertex_color.get() {
            NewVertexColor::Left => VertexColor::Left,
            NewVertexColor::Right => VertexColor::Right,
        }
    }
}

imgui_enum! {
    GraphEditingMode {
        DragVertex, "Drag vertex",
        ColorVertexBlue, "Color vertex blue (left)",
        ColorVertexRed, "Color vertex red (right)",
        MoveLeft, "Blue move (left)",
        MoveRight, "Red move (right)",
        AddVertex, "Add vertex",
        DeleteVertex, "Remove vertex",
        AddEdge, "Add/Remove edge",
    }
}

enum Edit {
    DragVertex,
    Color(Player),
    Move(Player),
    AddVertex,
    DeleteVertex,
    AddEdge,
}

impl From<GraphEditingMode> for Edit {
    fn from(mode: GraphEditingMode) -> Edit {
        match mode {
            GraphEditingMode::DragVertex => Edit::DragVertex,
            GraphEditingMode::ColorVertexBlue => Edit::Color(Player::Left),
            GraphEditingMode::ColorVertexRed => Edit::Color(Player::Right),
            GraphEditingMode::MoveLeft => Edit::Move(Player::Left),
            GraphEditingMode::MoveRight => Edit::Move(Player::Right),
            GraphEditingMode::AddVertex => Edit::AddVertex,
            GraphEditingMode::DeleteVertex => Edit::DeleteVertex,
            GraphEditingMode::AddEdge => Edit::AddEdge,
        }
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

impl_has!(PositionedVertex -> color -> VertexColor);
impl_has!(PositionedVertex -> position -> V2f);

#[derive(Debug, Clone)]
pub struct DigraphPlacementWindow {
    game: DigraphPlacement<PositionedVertex, DirectedGraph<PositionedVertex>>,
    reposition_option_selected: RawOf<RepositionMode>,
    new_vertex_color: RawOf<NewVertexColor>,
    editing_mode: RawOf<GraphEditingMode>,
    alternating_moves: bool,
    edge_creates_vertex: bool,
    details: Option<Details>,
    edge_start_vertex: Option<VertexIndex>,
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
            alternating_moves: true,
            edge_creates_vertex: true,
            details: None,
            edge_start_vertex: None,
        }
    }

    pub fn reposition_circle(&mut self) {
        let circle = CircleEdge {
            circle_radius: imgui::Canvas::vertex_radius()
                * (self.game.graph.size() as f32 + 4.0)
                * 0.5,
            vertex_radius: imgui::Canvas::vertex_radius(),
        };
        circle.layout(&mut self.game.graph);
    }

    pub fn reposition(&mut self, graph_panel_size: V2f) {
        match self.reposition_option_selected.get() {
            RepositionMode::Circle => {
                self.reposition_circle();
            }
            RepositionMode::SpringEmbedder => {
                let radius = imgui::Canvas::vertex_radius();
                let spring_embedder = SpringEmbedder {
                    cooling_rate: 0.99999,
                    c_attractive: 1.0,
                    c_repulsive: 250.0,
                    ideal_spring_length: 140.0,
                    iterations: 1 << 14,
                    bounds: Some((
                        V2f {
                            x: radius,
                            y: radius,
                        },
                        V2f {
                            x: f32::max(radius, radius.mul_add(-2.0, graph_panel_size.x)),
                            y: f32::max(radius, radius.mul_add(-2.0, graph_panel_size.y)),
                        },
                    )),
                };
                spring_embedder.layout(&mut self.game.graph);
            }
        }
    }
}

impl IsCgtWindow for TitledWindow<DigraphPlacementWindow> {
    impl_titled_window!("Digraph Placement");

    fn initialize(&mut self, ctx: &GuiContext) {
        self.content.reposition_circle();
        self.content.reposition(V2f { x: 350.0, y: 400.0 });
        let graph = self.content.game.graph.map(|v| v.color);
        ctx.schedule_task(
            "Digraph Placement",
            crate::Task::EvalDigraphPlacement(crate::EvalTask {
                window: self.window_id,
                game: DigraphPlacement::new(graph),
            }),
        );
    }
    fn update(&mut self, update: crate::UpdateKind) {
        let graph = self.content.game.graph.map(|v| v.color);
        if let UpdateKind::DigraphPlacementDetails(game, details) = update {
            if graph == game.graph {
                self.content.details = Some(details);
            }
        }
    }

    fn draw(&mut self, ui: &Ui, ctx: &mut GuiContext) {
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
                        "digraph_placement",
                        &self.content.game,
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
                    self.content.game = DigraphPlacement::new(DirectedGraph::empty(&[]));
                    is_dirty = true;
                }

                self.content
                    .editing_mode
                    .combo(ui, "Edit Mode", ComboBoxFlags::HEIGHT_LARGE);

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

                if matches!(
                    self.content.editing_mode.get(),
                    GraphEditingMode::AddVertex
                        | GraphEditingMode::AddEdge if self.content.edge_creates_vertex
                ) {
                    self.content.new_vertex_color.combo(
                        ui,
                        "New Vertex Color",
                        ComboBoxFlags::HEIGHT_LARGE,
                    );
                }

                short_inputs.end();

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

                let pressed = canvas.pressed_vertex(&self.content.game.graph);
                let clicked = canvas.clicked_vertex(&self.content.game.graph);

                match Edit::from(self.content.editing_mode.get()) {
                    Edit::DragVertex => {
                        if let Some(pressed) = pressed {
                            let delta = V2f::from(ui.io().mouse_delta);
                            let current_pos: &mut V2f = self
                                .content
                                .game
                                .graph
                                .get_vertex_mut(pressed)
                                .get_inner_mut();
                            *current_pos += delta;
                        }
                    }
                    Edit::Color(player) => {
                        if let Some(clicked) = clicked {
                            let current_color: &mut VertexColor = self
                                .content
                                .game
                                .graph
                                .get_vertex_mut(clicked)
                                .get_inner_mut();
                            *current_color = VertexColor::from(player);
                            is_dirty = true;
                        }
                    }
                    Edit::Move(player) => {
                        if let Some(clicked) = clicked {
                            let clicked_color: &VertexColor =
                                self.content.game.graph.get_vertex(clicked).get_inner();
                            if *clicked_color == VertexColor::from(player) {
                                self.content.game = self.content.game.move_in_vertex(clicked);
                                if self.content.alternating_moves {
                                    self.content.editing_mode = RawOf::new(match player {
                                        Player::Left => GraphEditingMode::MoveRight,
                                        Player::Right => GraphEditingMode::MoveLeft,
                                    });
                                }
                                is_dirty = true;
                            }
                        }
                    }
                    Edit::AddVertex => {
                        if let Some(new_vertex_position) = new_vertex_position {
                            self.content.game.graph.add_vertex(PositionedVertex {
                                color: VertexColor::from(self.content.new_vertex_color),
                                position: new_vertex_position,
                            });
                            is_dirty = true;
                        }
                    }
                    Edit::DeleteVertex => {
                        if let Some(clicked) = clicked {
                            self.content.game.graph.remove_vertex(clicked);
                            is_dirty = true;
                        }
                    }
                    Edit::AddEdge => {
                        let mouse_position = V2f::from(ui.io().mouse_pos) - graph_area_position;
                        if let Some(pressed) = pressed {
                            self.content.edge_start_vertex = Some(pressed);
                            let pressed_position: V2f =
                                self.content.game.graph.get_vertex_mut(pressed).position;
                            canvas.line(
                                pressed_position,
                                mouse_position,
                                imgui::Canvas::thin_line_weight(),
                                Color::BLACK,
                            );
                        } else if let Some(start) = self.content.edge_start_vertex.take() {
                            if let Some(end) =
                                canvas.vertex_at_position(mouse_position, &self.content.game.graph)
                                && start != end
                            {
                                self.content.game.graph.connect(
                                    start,
                                    end,
                                    !self.content.game.graph.are_adjacent(start, end),
                                );
                                is_dirty = true;
                            } else if self.content.edge_creates_vertex {
                                let end = self.content.game.graph.add_vertex(PositionedVertex {
                                    color: VertexColor::from(self.content.new_vertex_color),
                                    position: mouse_position,
                                });
                                self.content.game.graph.connect(start, end, true);
                                is_dirty = true;
                            }
                        }
                    }
                }

                ui.next_column();
                widgets::game_details(
                    self.content.details.as_ref(),
                    &mut self.scratch_buffer,
                    ui,
                    &draw_list,
                    ctx.large_font_id,
                );

                if is_dirty {
                    self.content.details = None;
                    let graph = self.content.game.graph.map(|v| v.color);
                    ctx.schedule_task(
                        "Digraph Placement",
                        Task::EvalDigraphPlacement(EvalTask {
                            window: self.window_id,
                            game: DigraphPlacement::new(graph),
                        }),
                    );
                }

                if should_reposition {
                    self.content.reposition(graph_area_size);
                }
            });
    }
}
