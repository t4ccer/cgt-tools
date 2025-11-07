use crate::{
    AccessTracker, GuiContext, IsCgtWindow, TitledWindow, UpdateKind, imgui_enum,
    impl_titled_window,
    widgets::{AddEdgeMode, RepositionMode},
};
use ::imgui::{Condition, Ui};
use cgt::{
    drawing::{Canvas, imgui},
    graph::{
        Graph, VertexIndex,
        adjacency_matrix::undirected::UndirectedGraph,
        layout::{Bounds, CircleEdge, SpringEmbedder},
        resolving_set::{self, Tower},
    },
    has::Has,
    impl_has,
    numeric::v2f::V2f,
};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Vertex {
    position: V2f,
    inner: resolving_set::Vertex,
}

impl_has!(Vertex -> position -> V2f);
impl_has!(Vertex -> inner -> resolving_set::Vertex);

impl Vertex {
    pub const fn new(tower: Option<Tower>) -> Vertex {
        Vertex {
            position: V2f::ZERO,
            inner: resolving_set::Vertex::new(tower),
        }
    }
}

imgui_enum! {
    #[derive(Debug, Clone, Copy)]
    GraphEditingMode {
        DragVertex, "Drag vertex",
        AddRemoveTower, "Add/Remove tower",
        AddVertex, "Add vertex",
        DeleteVertex, "Remove vertex",
        AddEdge, "Add/Remove edge",
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct CodeVertex {
    position: V2f,
    inner: resolving_set::CodeVertex,
}

impl_has!(CodeVertex -> position -> V2f);
impl_has!(CodeVertex -> inner -> resolving_set::CodeVertex);

#[derive(Debug, Clone)]
struct CodeGraphPanel {
    graph: UndirectedGraph<CodeVertex>,
    reposition_mode: RepositionMode,
    collisions: String,
}

impl CodeGraphPanel {
    fn new<G, V>(graph: &G, infinite_distances: bool) -> CodeGraphPanel
    where
        G: Graph<V>,
        V: Has<resolving_set::Vertex> + Clone,
    {
        let aux: UndirectedGraph<_> =
            resolving_set::one_bit_error_auxiliary_graph(graph, infinite_distances);

        let mut collisions = String::from("Collisions: ");
        for (idx, v) in aux
            .vertices()
            .filter(|v| v.is_colliding() && v.is_original())
            .enumerate()
        {
            use std::fmt::Write;

            if idx != 0 {
                collisions.push_str(", ");
            }
            write!(collisions, "{}", v.distances()).unwrap();
        }

        CodeGraphPanel {
            graph: aux.map(|v| CodeVertex {
                inner: v.clone(),
                position: V2f::ZERO,
            }),
            reposition_mode: RepositionMode::SpringEmbedder,
            collisions,
        }
    }

    fn reposition_circle(&mut self) {
        let circle = CircleEdge {
            circle_radius: imgui::Canvas::vertex_radius() * (self.graph.size() as f32 + 4.0) * 0.5,
            vertex_radius: imgui::Canvas::vertex_radius(),
        };
        circle.layout(&mut self.graph);
    }

    fn reposition(&mut self, graph_panel_size: V2f) {
        match self.reposition_mode {
            RepositionMode::Circle => {
                self.reposition_circle();
            }
            RepositionMode::SpringEmbedder => {
                let spring_embedder = SpringEmbedder {
                    cooling_rate: 0.999,
                    c_attractive: 1.0,
                    c_repulsive: 350.0,
                    ideal_spring_length: 75.0,
                    iterations: 1 << 12,
                    bounds: Some(Bounds {
                        lower: V2f {
                            x: imgui::Canvas::vertex_radius(),
                            y: imgui::Canvas::vertex_radius(),
                        },
                        upper: V2f {
                            x: f32::max(
                                imgui::Canvas::vertex_radius(),
                                imgui::Canvas::vertex_radius().mul_add(-2.0, graph_panel_size.x),
                            ),
                            y: f32::max(
                                imgui::Canvas::vertex_radius(),
                                imgui::Canvas::vertex_radius().mul_add(-2.0, graph_panel_size.y),
                            ),
                        },
                        c_middle_attractive: Some(0.003),
                    }),
                };
                spring_embedder.layout(&mut self.graph);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvingSetWindow {
    graph: AccessTracker<UndirectedGraph<Vertex>>,
    reposition_mode: RepositionMode,
    editing_mode: GraphEditingMode,
    add_edge_mode: AddEdgeMode,
    code_graph: CodeGraphPanel,
    infinite_distances: bool,
    duplicates: String,
}

impl ResolvingSetWindow {
    pub fn new() -> ResolvingSetWindow {
        let graph = Graph::from_edges(
            &[
                (VertexIndex { index: 0 }, VertexIndex { index: 1 }),
                (VertexIndex { index: 0 }, VertexIndex { index: 2 }),
                (VertexIndex { index: 0 }, VertexIndex { index: 3 }),
                (VertexIndex { index: 1 }, VertexIndex { index: 2 }),
            ],
            &[
                Vertex::new(None),
                Vertex::new(Some(Tower::Unrestricted)),
                Vertex::new(None),
                Vertex::new(None),
            ],
        );

        let infinite_distances = false;
        let code_graph = CodeGraphPanel::new(&graph, infinite_distances);

        let mut this = ResolvingSetWindow {
            graph: AccessTracker::new(graph),
            reposition_mode: RepositionMode::SpringEmbedder,
            editing_mode: GraphEditingMode::DragVertex,
            add_edge_mode: AddEdgeMode::new(),
            code_graph,
            infinite_distances,
            duplicates: String::new(),
        };

        this.recompute();
        this.code_graph.reposition(V2f { x: 350.0, y: 350.0 });

        this
    }

    pub fn reposition_circle(&mut self) {
        let circle = CircleEdge {
            circle_radius: imgui::Canvas::vertex_radius() * (self.graph.size() as f32 + 4.0) * 0.5,
            vertex_radius: imgui::Canvas::vertex_radius(),
        };
        circle.layout(self.graph.get_mut_untracked());
    }

    pub fn reposition(&mut self, graph_panel_size: V2f) {
        match self.reposition_mode {
            RepositionMode::Circle => {
                self.reposition_circle();
            }
            RepositionMode::SpringEmbedder => {
                let spring_embedder = SpringEmbedder {
                    cooling_rate: 0.99999,
                    c_attractive: 1.0,
                    c_repulsive: 250.0,
                    ideal_spring_length: 75.0,
                    iterations: 1 << 14,
                    bounds: Some(Bounds {
                        lower: V2f {
                            x: imgui::Canvas::vertex_radius(),
                            y: imgui::Canvas::vertex_radius(),
                        },
                        upper: V2f {
                            x: f32::max(
                                imgui::Canvas::vertex_radius(),
                                imgui::Canvas::vertex_radius().mul_add(-2.0, graph_panel_size.x),
                            ),
                            y: f32::max(
                                imgui::Canvas::vertex_radius(),
                                imgui::Canvas::vertex_radius().mul_add(-2.0, graph_panel_size.y),
                            ),
                        },
                        c_middle_attractive: None,
                    }),
                };
                spring_embedder.layout(self.graph.get_mut_untracked());
            }
        }
    }

    fn recompute(&mut self) {
        resolving_set::label_distances(self.graph.get_mut_untracked());

        let mut seen = HashSet::new();
        self.duplicates.clear();
        self.duplicates.push_str("Duplicates: ");
        for (idx, v) in self
            .graph
            .vertices()
            .filter(|v| !v.inner.is_unique() && seen.insert(v.inner.distances()))
            .enumerate()
        {
            use std::fmt::Write;

            if idx != 0 {
                self.duplicates.push_str(", ");
            }
            write!(self.duplicates, "{}", v.inner.distances()).unwrap();
        }

        self.code_graph = CodeGraphPanel::new(&*self.graph, self.infinite_distances);
        self.code_graph.reposition_circle();
    }
}
impl IsCgtWindow for TitledWindow<ResolvingSetWindow> {
    impl_titled_window!("Resolving Set");

    fn initialize(&mut self, _ctx: &GuiContext) {
        self.content.reposition_circle();
        self.content.reposition(V2f { x: 350.0, y: 350.0 });
    }

    fn draw(&mut self, ui: &Ui, ctx: &mut GuiContext) {
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
                            ctx.new_windows
                                .push(Box::new(TitledWindow::without_title(self.content.clone())));
                        }
                    }
                }

                ui.columns(2, "columns", true);

                let short_inputs = ui.push_item_width(200.0);
                self.content.reposition_mode.combo(ui, "##Reposition Mode");
                ui.same_line();
                let mut should_reposition_main = ui.button("Reposition");
                ui.same_line();
                if ui.button("Clear") {
                    *self.content.graph = Graph::empty(&[Vertex {
                        position: V2f::ZERO,
                        inner: resolving_set::Vertex::new(None),
                    }]);
                    should_reposition_main = true;
                }

                self.content.editing_mode.combo(ui, "Edit Mode");

                if matches!(self.content.editing_mode, GraphEditingMode::AddEdge) {
                    ui.same_line();
                    ui.checkbox(
                        "Add vertex",
                        &mut self.content.add_edge_mode.edge_creates_vertex,
                    );
                }

                if ui.checkbox("Infinite distances", &mut self.content.infinite_distances) {
                    let _ = &mut *self.content.graph;
                }

                short_inputs.end();

                ui.text_wrapped(&self.content.duplicates);

                let graph_area_position = V2f::from(ui.cursor_screen_pos());
                let graph_area_size = V2f {
                    x: ui.current_column_width(),
                    y: unsafe { ui.style().item_spacing[1] }
                        .mul_add(-2.0, ui.window_size()[1] - ui.cursor_pos()[1]),
                };
                let new_vertex_position =
                    (matches!(self.content.editing_mode, GraphEditingMode::AddVertex)
                        && ui.invisible_button("Add vertex", graph_area_size))
                    .then(|| V2f::from(ui.io().mouse_pos) - graph_area_position);
                ui.set_cursor_screen_pos(graph_area_position);

                let mut canvas =
                    imgui::Canvas::new(ui, &draw_list, ctx.large_font_id, &mut self.scratch_buffer);
                resolving_set::draw_graph(&mut canvas, &*self.content.graph);
                let pressed = canvas.pressed_vertex();
                let clicked = canvas.clicked_vertex(&*self.content.graph);
                match self.content.editing_mode {
                    GraphEditingMode::DragVertex => {
                        if let Some(pressed) = pressed {
                            let delta = V2f::from(ui.io().mouse_delta);
                            let current_pos: &mut V2f = self
                                .content
                                .graph
                                .get_mut_untracked()
                                .get_vertex_mut(pressed)
                                .get_inner_mut();
                            *current_pos += delta;
                        }
                    }
                    GraphEditingMode::AddRemoveTower => {
                        if let Some(clicked) = clicked {
                            let vertex: &mut Vertex =
                                self.content.graph.get_vertex_mut(clicked).get_inner_mut();
                            vertex.inner.set_tower(if vertex.inner.tower().is_some() {
                                None
                            } else {
                                Some(Tower::Unrestricted)
                            });
                        }
                    }
                    GraphEditingMode::AddVertex => {
                        if let Some(new_vertex_position) = new_vertex_position {
                            self.content.graph.add_vertex(Vertex {
                                inner: resolving_set::Vertex::new(None),
                                position: new_vertex_position,
                            });
                        }
                    }
                    GraphEditingMode::DeleteVertex => {
                        if let Some(clicked) = clicked {
                            self.content.graph.remove_vertex(clicked);
                        }
                    }
                    GraphEditingMode::AddEdge => {
                        self.content.add_edge_mode.handle_update(
                            V2f::from(ui.io().mouse_pos),
                            graph_area_position,
                            &mut canvas,
                            &mut self.content.graph,
                            |position| Vertex {
                                inner: resolving_set::Vertex::new(None),
                                position,
                            },
                        );
                    }
                }

                if should_reposition_main {
                    self.content.reposition(graph_area_size);
                }

                ui.next_column();
                let aux_id = ui.push_id("aux");

                let short_inputs = ui.push_item_width(200.0);
                self.content
                    .code_graph
                    .reposition_mode
                    .combo(ui, "##Reposition Mode");
                ui.same_line();
                let should_reposition_aux = ui.button("Reposition");
                short_inputs.end();

                ui.text_wrapped(&self.content.code_graph.collisions);

                let graph_area_size = V2f {
                    x: ui.current_column_width(),
                    y: unsafe { ui.style().item_spacing[1] }
                        .mul_add(-2.0, ui.window_size()[1] - ui.cursor_pos()[1]),
                };

                let mut canvas =
                    imgui::Canvas::new(ui, &draw_list, ctx.large_font_id, &mut self.scratch_buffer);
                resolving_set::draw_code_graph(&mut canvas, &self.content.code_graph.graph);
                let pressed = canvas.pressed_vertex();
                if let Some(pressed) = pressed {
                    let delta = V2f::from(ui.io().mouse_delta);
                    let current_pos: &mut V2f = self
                        .content
                        .code_graph
                        .graph
                        .get_vertex_mut(pressed)
                        .get_inner_mut();
                    *current_pos += delta;
                }

                if should_reposition_aux {
                    self.content.code_graph.reposition(graph_area_size);
                }
                drop(aux_id);

                if self.content.graph.clear_flag() {
                    self.content.recompute();
                    self.content.code_graph.reposition(graph_area_size);
                }
            });
    }

    fn update(&mut self, _update: UpdateKind) {}
}
