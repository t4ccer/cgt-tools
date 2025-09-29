use crate::{
    GuiContext, IsCgtWindow, RawOf, TitledWindow, UpdateKind, imgui_enum, impl_titled_window,
    widgets::AccessTracker,
};
use ::imgui::{ComboBoxFlags, Condition, Ui};
use cgt::{
    drawing::{Canvas, Color, imgui},
    graph::{
        Graph, VertexIndex,
        adjacency_matrix::undirected::UndirectedGraph,
        layout::{CircleEdge, SpringEmbedder},
        resolving_set::{self, Tower},
    },
    has::Has,
    impl_has,
    numeric::v2f::V2f,
};
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Vertex {
    position: V2f,
    inner: resolving_set::Vertex,
}

impl Vertex {
    pub fn new(tower: Option<Tower>) -> Vertex {
        Vertex {
            position: V2f::ZERO,
            inner: resolving_set::Vertex::new(tower),
        }
    }
}

impl_has!(Vertex -> position -> V2f);
impl_has!(Vertex -> inner -> resolving_set::Vertex);

imgui_enum! {
    GraphEditingMode {
        DragVertex, "Drag vertex",
        AddRemoveTower, "Add/Remove tower",
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

pub struct ResolvingSetWindow {
    graph: AccessTracker<UndirectedGraph<Vertex>>,
    reposition_option_selected: RawOf<RepositionMode>,
    editing_mode: RawOf<GraphEditingMode>,
    edge_creates_vertex: bool,
    edge_start_vertex: Option<VertexIndex>,
}

impl ResolvingSetWindow {
    pub fn new() -> ResolvingSetWindow {
        ResolvingSetWindow {
            graph: AccessTracker::new(Graph::from_edges(
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
            )),
            reposition_option_selected: RawOf::new(RepositionMode::SpringEmbedder),
            editing_mode: RawOf::new(GraphEditingMode::DragVertex),
            edge_creates_vertex: true,
            edge_start_vertex: None,
        }
    }

    pub fn reposition_circle(&mut self) {
        let circle = CircleEdge {
            circle_radius: imgui::Canvas::vertex_radius() * (self.graph.size() as f32 + 4.0) * 0.5,
            vertex_radius: imgui::Canvas::vertex_radius(),
        };
        circle.layout(self.graph.get_mut_untracked());
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
                    ideal_spring_length: 75.0,
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
                spring_embedder.layout(self.graph.get_mut_untracked());
            }
        }
    }
}
impl IsCgtWindow for TitledWindow<ResolvingSetWindow> {
    impl_titled_window!("Resolving Set");

    fn initialize(&mut self, _ctx: &GuiContext) {
        self.content.reposition_circle();
        self.content.reposition(V2f { x: 350.0, y: 400.0 });
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

                let short_inputs = ui.push_item_width(200.0);
                self.content.reposition_option_selected.combo(
                    ui,
                    "##Reposition Mode",
                    ComboBoxFlags::empty(),
                );
                ui.same_line();
                let should_reposition = ui.button("Reposition");
                ui.same_line();
                if ui.button("Clear") {
                    *self.content.graph = Graph::empty(&[]);
                }

                self.content
                    .editing_mode
                    .combo(ui, "Edit Mode", ComboBoxFlags::HEIGHT_LARGE);
                short_inputs.end();

                if matches!(self.content.editing_mode.get(), GraphEditingMode::AddEdge) {
                    ui.same_line();
                    ui.checkbox("Add vertex", &mut self.content.edge_creates_vertex);
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
                resolving_set::draw_graph(&mut canvas, self.content.graph.deref());
                let pressed = canvas.pressed_vertex();
                let clicked = canvas.clicked_vertex(self.content.graph.deref());
                match self.content.editing_mode.get() {
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
                            let vertex: &mut Vertex = self
                                .content
                                .graph
                                .get_mut_untracked()
                                .get_vertex_mut(clicked)
                                .get_inner_mut();
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
                        let mouse_position = V2f::from(ui.io().mouse_pos) - graph_area_position;
                        if let Some(pressed) = pressed {
                            self.content.edge_start_vertex = Some(pressed);
                            let pressed_position: V2f =
                                self.content.graph.get_vertex_mut(pressed).position;
                            canvas.line(
                                pressed_position,
                                mouse_position,
                                imgui::Canvas::thin_line_weight(),
                                Color::BLACK,
                            );
                        } else if let Some(start) = self.content.edge_start_vertex.take() {
                            if let Some(end) = canvas
                                .vertex_at_position(mouse_position, self.content.graph.deref())
                                && start != end
                            {
                                let should_connect = !self.content.graph.are_adjacent(start, end);
                                self.content.graph.connect(start, end, should_connect);
                            } else if self.content.edge_creates_vertex {
                                let end = self.content.graph.add_vertex(Vertex {
                                    inner: resolving_set::Vertex::new(None),
                                    position: mouse_position,
                                });
                                self.content.graph.connect(start, end, true);
                            }
                        }
                    }
                }

                if self.content.graph.clear_flag() {
                    resolving_set::label_distances(self.content.graph.deref_mut());
                }

                if should_reposition {
                    self.content.reposition(graph_area_size);
                }
            });
    }

    fn update(&mut self, _update: UpdateKind) {}
}
