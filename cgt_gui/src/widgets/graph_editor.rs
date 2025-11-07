use crate::{
    GuiContext, IsCgtWindow, TitledWindow, imgui_enum, impl_titled_window,
    widgets::{AddEdgeMode, RepositionMode, save_button},
};
use ::imgui::{Condition, Ui};
use cgt::{
    drawing::{Canvas, Color, Draw, imgui},
    graph::{
        Graph, VertexIndex,
        adjacency_matrix::{directed::DirectedGraph, undirected::UndirectedGraph},
        layout::{Bounds, CircleEdge, SpringEmbedder},
    },
    has::Has,
    impl_has,
    numeric::v2f::V2f,
};

imgui_enum! {
    #[derive(Debug, Clone, Copy)]
    GraphEditingMode {
        DragVertex, "Drag vertex",
        ColorVertexBlue, "Color vertex blue (left)",
        ColorVertexRed, "Color vertex red (right)",
        ColorVertexNone, "Clear color",
        AddVertex, "Add vertex",
        DeleteVertex, "Remove vertex",
        AddEdge, "Add/Remove edge",
    }
}

#[derive(Debug, Clone, Copy)]
pub enum VertexColor {
    Blue,
    Red,
    None,
}

impl From<NewVertexColor> for VertexColor {
    fn from(color: NewVertexColor) -> VertexColor {
        match color {
            NewVertexColor::Blue => VertexColor::Blue,
            NewVertexColor::Red => VertexColor::Red,
            NewVertexColor::None => VertexColor::None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PositionedVertex {
    color: VertexColor,
    position: V2f,
}

impl_has!(PositionedVertex -> color -> VertexColor);
impl_has!(PositionedVertex -> position -> V2f);

imgui_enum! {
    #[derive(Debug, Clone, Copy)]
    NewVertexColor {
        Blue, "Blue",
        Red, "Red",
        None, "None",
    }
}

#[derive(Debug, Clone)]
pub struct GraphWindow<G> {
    widget: GraphWidget<G>,
    reposition_mode: RepositionMode,
    new_vertex_color: NewVertexColor,
    editing_mode: GraphEditingMode,
    add_edge_mode: AddEdgeMode,
}

impl<G> GraphWindow<G>
where
    G: Graph<PositionedVertex> + Clone + 'static,
    TitledWindow<GraphWindow<G>>: IsCgtWindow,
    GraphWidget<G>: FilePrefix,
{
    pub fn new() -> GraphWindow<G> {
        let mut this = GraphWindow {
            // caterpillar C(4, 3, 4)
            widget: GraphWidget {
                graph: G::from_edges(
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
                    &[PositionedVertex {
                        color: VertexColor::None,
                        position: V2f::ZERO,
                    }; 14],
                ),
            },
            new_vertex_color: NewVertexColor::None,
            reposition_mode: RepositionMode::SpringEmbedder,
            editing_mode: GraphEditingMode::DragVertex,
            add_edge_mode: AddEdgeMode::new(),
        };
        this.reposition_circle();
        this.reposition(V2f { x: 350.0, y: 400.0 });
        this
    }

    pub fn reposition_circle(&mut self) {
        let circle = CircleEdge {
            circle_radius: imgui::Canvas::vertex_radius()
                * (self.widget.graph.size() as f32 + 4.0)
                * 0.5,
            vertex_radius: imgui::Canvas::vertex_radius(),
        };
        circle.layout(&mut self.widget.graph);
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
                    ideal_spring_length: 40.0,
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
                spring_embedder.layout(&mut self.widget.graph);
            }
        }
    }

    fn draw_impl(&mut self, ui: &Ui, ctx: &mut GuiContext, scratch_buffer: &mut String) {
        let draw_list = ui.get_window_draw_list();

        if let Some(_menu_bar) = ui.begin_menu_bar() {
            if let Some(_new_menu) = ui.begin_menu("New") {
                if ui.menu_item("Duplicate") {
                    let w = self.clone();
                    ctx.new_windows
                        .push(Box::new(TitledWindow::without_title(w)));
                }
            }
            save_button(
                ui,
                <GraphWidget<G> as FilePrefix>::FILE_PREFIX,
                &self.widget,
                None,
            );
        }

        let short_inputs = ui.push_item_width(200.0);
        self.reposition_mode.combo(ui, "##Reposition Mode");
        ui.same_line();
        let should_reposition = ui.button("Reposition");
        ui.same_line();
        if ui.button("Clear") {
            self.widget.graph = G::empty(&[PositionedVertex {
                color: VertexColor::None,
                position: V2f { x: 32.0, y: 32.0 },
            }]);
        }

        self.editing_mode.combo(ui, "Edit Mode");

        if matches!(self.editing_mode, GraphEditingMode::AddEdge) {
            ui.same_line();
            ui.checkbox("Add vertex", &mut self.add_edge_mode.edge_creates_vertex);
        }

        if matches!(self.editing_mode, GraphEditingMode::AddVertex)
            || matches!(
                self.editing_mode,
                GraphEditingMode::AddEdge if self.add_edge_mode.edge_creates_vertex
            )
        {
            self.new_vertex_color.combo(ui, "New Vertex Color");
        }

        short_inputs.end();

        let graph_area_position = V2f::from(ui.cursor_screen_pos());
        let graph_area_size = V2f {
            x: ui.current_column_width(),
            y: unsafe { ui.style().item_spacing[1] }
                .mul_add(-2.0, ui.window_size()[1] - ui.cursor_pos()[1]),
        };
        let new_vertex_position = (matches!(self.editing_mode, GraphEditingMode::AddVertex)
            && ui.invisible_button("Add vertex area", graph_area_size))
        .then(|| V2f::from(ui.io().mouse_pos) - graph_area_position);
        ui.set_cursor_screen_pos(graph_area_position);

        let mut canvas = imgui::Canvas::new(ui, &draw_list, ctx.large_font_id, scratch_buffer);
        self.widget.draw(&mut canvas);

        let pressed = canvas.pressed_vertex();
        let clicked = canvas.clicked_vertex(&self.widget.graph);
        match self.editing_mode {
            GraphEditingMode::DragVertex => {
                if let Some(pressed) = pressed {
                    let delta = V2f::from(ui.io().mouse_delta);
                    let current_pos: &mut V2f =
                        self.widget.graph.get_vertex_mut(pressed).get_inner_mut();
                    *current_pos += delta;
                }
            }
            GraphEditingMode::ColorVertexBlue => {
                if let Some(clicked) = clicked {
                    let clicked_vertex: &mut VertexColor =
                        self.widget.graph.get_vertex_mut(clicked).get_inner_mut();
                    *clicked_vertex = VertexColor::Blue;
                }
            }
            GraphEditingMode::ColorVertexRed => {
                if let Some(clicked) = clicked {
                    let clicked_vertex: &mut VertexColor =
                        self.widget.graph.get_vertex_mut(clicked).get_inner_mut();
                    *clicked_vertex = VertexColor::Red;
                }
            }
            GraphEditingMode::ColorVertexNone => {
                if let Some(clicked) = clicked {
                    let clicked_vertex: &mut VertexColor =
                        self.widget.graph.get_vertex_mut(clicked).get_inner_mut();
                    *clicked_vertex = VertexColor::None;
                }
            }
            GraphEditingMode::AddVertex => {
                if let Some(new_vertex_position) = new_vertex_position {
                    self.widget.graph.add_vertex(PositionedVertex {
                        color: VertexColor::from(self.new_vertex_color),
                        position: new_vertex_position,
                    });
                }
            }
            GraphEditingMode::DeleteVertex => {
                if let Some(clicked) = clicked {
                    self.widget.graph.remove_vertex(clicked);
                }
            }
            GraphEditingMode::AddEdge => {
                self.add_edge_mode.handle_update(
                    V2f::from(ui.io().mouse_pos),
                    graph_area_position,
                    &mut canvas,
                    &mut &mut self.widget.graph,
                    |position| PositionedVertex {
                        color: VertexColor::from(self.new_vertex_color),
                        position,
                    },
                );
            }
        }

        if should_reposition {
            self.reposition(graph_area_size);
        }
    }
}

#[derive(Debug, Clone)]
pub struct GraphWidget<G> {
    graph: G,
}

impl<G> Draw for GraphWidget<G>
where
    G: Graph<PositionedVertex>,
{
    fn draw<C>(&self, canvas: &mut C)
    where
        C: Canvas,
    {
        self.graph.draw(canvas, |canvas, vertex_index| {
            let position: V2f = *self.graph.get_vertex(vertex_index).get_inner();
            let color: VertexColor = *self.graph.get_vertex(vertex_index).get_inner();
            canvas.vertex(
                position,
                match color {
                    VertexColor::None => Color::LIGHT_GRAY,
                    VertexColor::Blue => Color::BLUE,
                    VertexColor::Red => Color::RED,
                },
                vertex_index,
            );
        });
    }

    fn required_canvas<C>(&self) -> cgt::drawing::BoundingBox
    where
        C: Canvas,
    {
        self.graph.required_canvas::<C>()
    }
}

pub trait FilePrefix {
    const FILE_PREFIX: &'static str;
}

impl FilePrefix for GraphWidget<DirectedGraph<PositionedVertex>> {
    const FILE_PREFIX: &'static str = "directed_graph";
}

impl FilePrefix for GraphWidget<UndirectedGraph<PositionedVertex>> {
    const FILE_PREFIX: &'static str = "undirected_graph";
}

impl IsCgtWindow for TitledWindow<GraphWindow<UndirectedGraph<PositionedVertex>>> {
    impl_titled_window!("Undirected Graph");

    fn initialize(&mut self, _ctx: &GuiContext) {}

    fn update(&mut self, _update: crate::UpdateKind) {}

    fn draw(&mut self, ui: &Ui, ctx: &mut GuiContext) {
        ui.window(&self.title)
            .position(ui.io().mouse_pos, Condition::Appearing)
            .size([800.0, 450.0], Condition::Appearing)
            .bring_to_front_on_focus(true)
            .menu_bar(true)
            .opened(&mut self.is_open)
            .build(|| {
                self.content.draw_impl(ui, ctx, &mut self.scratch_buffer);
            });
    }
}

impl IsCgtWindow for TitledWindow<GraphWindow<DirectedGraph<PositionedVertex>>> {
    impl_titled_window!("Directed Graph");

    fn initialize(&mut self, _ctx: &GuiContext) {}

    fn update(&mut self, _update: crate::UpdateKind) {}

    fn draw(&mut self, ui: &Ui, ctx: &mut GuiContext) {
        ui.window(&self.title)
            .position(ui.io().mouse_pos, Condition::Appearing)
            .size([800.0, 450.0], Condition::Appearing)
            .bring_to_front_on_focus(true)
            .menu_bar(true)
            .opened(&mut self.is_open)
            .build(|| {
                self.content.draw_impl(ui, ctx, &mut self.scratch_buffer);
            });
    }
}
