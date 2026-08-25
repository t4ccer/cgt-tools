use crate::{
    SyncState,
    canvas::HtmlCanvas,
    reactive::{self, SelectOption, SelectOptionElement},
    report_edits_to_python, set_edited,
};
use cgt::{
    drawing::{Area, Canvas, Color, Hits, Interaction, Interactions},
    graph::{
        Graph, VertexIndex,
        adjacency_matrix::{directed::DirectedGraph, undirected::UndirectedGraph},
        layout::{CircleEdge, SpringEmbedder},
    },
    has::Has,
    impl_has,
    numeric::v2f::V2f,
    short::partizan::{
        Player,
        games::{
            bipartite_snort::{self, BipartiteSnort},
            col::{self, Col},
            digraph_placement::{self, DigraphPlacement},
            snort::{self, Snort},
        },
    },
};
use cgt_py_messages::{
    GraphBackendMessage, GraphFrontendMessage, GraphPreset, GraphPresetFlag, Vertex, VertexColor,
    WidgetGraph,
    layout::{default_bounds, default_circle, default_spring, max_spring_iterations},
};
use futures_signals::{
    map_ref,
    signal::{Mutable, SignalExt},
};
use jupyter_rust_widget_frontend::{AnyWidgetModel, Context, WasmWidget};
use wasm_bindgen::{
    JsCast, JsValue,
    prelude::{ScopedClosure, wasm_bindgen},
};
use web_sys::{
    CanvasRenderingContext2d, Document, Element, HtmlButtonElement, HtmlCanvasElement,
    HtmlDivElement, HtmlElement, HtmlInputElement, HtmlLabelElement,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditMode {
    MoveVertex,
    ToggleEdge,
    RemoveVertex,
    /// This is 3 in 1
    /// - Clicking on canvas adds a vertex of this color
    /// - Clicking on vertex changes the color
    /// - Dragging a vertex moves it without changing the color
    AddColorVertex(VertexColor),
    /// Play a move of the preset's game as a given player
    GameMove(Player),
}

impl EditMode {
    /// Color of the vertex that clicking on empty canvas adds, if the mode adds one at all.
    /// [`EditMode::ToggleEdge`] adds whichever vertex its own dropdown is set to
    const fn new_vertex_color(self, edge_vertex: Option<VertexColor>) -> Option<VertexColor> {
        match self {
            EditMode::ToggleEdge => edge_vertex,
            EditMode::AddColorVertex(color) => Some(color),
            EditMode::MoveVertex | EditMode::RemoveVertex | EditMode::GameMove(_) => None,
        }
    }

    /// Whether vertices follow the pointer that drags them
    const fn moves_vertices(self) -> bool {
        match self {
            EditMode::MoveVertex | EditMode::AddColorVertex(_) => true,
            EditMode::ToggleEdge | EditMode::RemoveVertex | EditMode::GameMove(_) => false,
        }
    }

    /// The same mode but played by the other player, if the mode plays moves at all
    const fn opposite_player(self) -> Option<EditMode> {
        match self {
            EditMode::MoveVertex
            | EditMode::ToggleEdge
            | EditMode::RemoveVertex
            | EditMode::AddColorVertex(_) => None,
            EditMode::GameMove(player) => Some(EditMode::GameMove(player.opposite())),
        }
    }
}

#[derive(Clone, Copy)]
struct Frame {
    vertices: Hits<VertexIndex>,
    background: Interaction,
}

const UNCOLORED_VERTEX: GraphPresetFlag =
    GraphPresetFlag::from_slice(&[GraphPresetFlag::Snort, GraphPresetFlag::Col]);

const PLAYER_VERTEX: GraphPresetFlag = GraphPresetFlag::from_slice(&[
    GraphPresetFlag::Snort,
    GraphPresetFlag::Col,
    GraphPresetFlag::DigraphPlacement,
    GraphPresetFlag::BipartiteSnort,
]);

const GREEN_VERTEX: GraphPresetFlag = GraphPresetFlag::from_slice(&[]);

const PLAYABLE: GraphPresetFlag = GraphPresetFlag::from_slice(&[
    GraphPresetFlag::Snort,
    GraphPresetFlag::Col,
    GraphPresetFlag::DigraphPlacement,
    GraphPresetFlag::BipartiteSnort,
]);

const EDIT_OPTIONS: &[EditOption] = &[
    EditOption {
        text: "Add/Remove Edge",
        mode: EditMode::ToggleEdge,
        visible_preset: GraphPresetFlag::all(),
    },
    EditOption {
        text: "Add White Vertex",
        mode: EditMode::AddColorVertex(VertexColor::White),
        visible_preset: UNCOLORED_VERTEX,
    },
    EditOption {
        text: "Add Blue Vertex",
        mode: EditMode::AddColorVertex(VertexColor::Blue),
        visible_preset: PLAYER_VERTEX,
    },
    EditOption {
        text: "Add Red Vertex",
        mode: EditMode::AddColorVertex(VertexColor::Red),
        visible_preset: PLAYER_VERTEX,
    },
    EditOption {
        text: "Add Green Vertex",
        mode: EditMode::AddColorVertex(VertexColor::Green),
        visible_preset: GREEN_VERTEX,
    },
    EditOption {
        text: "Move Vertex",
        mode: EditMode::MoveVertex,
        visible_preset: GraphPresetFlag::all(),
    },
    EditOption {
        text: "Remove Vertex",
        mode: EditMode::RemoveVertex,
        visible_preset: GraphPresetFlag::all(),
    },
    EditOption {
        text: "Left Move",
        mode: EditMode::GameMove(Player::Left),
        visible_preset: PLAYABLE,
    },
    EditOption {
        text: "Right Move",
        mode: EditMode::GameMove(Player::Right),
        visible_preset: PLAYABLE,
    },
];

/// Vertex that [`EditMode::ToggleEdge`] leaves behind when an edge is dropped on empty
/// canvas. The first option visible for a preset is the one that preset starts on
const EDGE_VERTEX_OPTIONS: &[EdgeVertexOption] = &[
    EdgeVertexOption {
        text: "White",
        color: Some(VertexColor::White),
        visible_preset: UNCOLORED_VERTEX,
    },
    EdgeVertexOption {
        text: "Blue",
        color: Some(VertexColor::Blue),
        visible_preset: PLAYER_VERTEX,
    },
    EdgeVertexOption {
        text: "Red",
        color: Some(VertexColor::Red),
        visible_preset: PLAYER_VERTEX,
    },
    EdgeVertexOption {
        text: "Green",
        color: Some(VertexColor::Green),
        visible_preset: GREEN_VERTEX,
    },
    EdgeVertexOption {
        text: "None",
        color: None,
        visible_preset: GraphPresetFlag::all(),
    },
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum LayoutAlgorithm {
    SpringEmbedder,
    Circle,
}

/// Layouts do not care which game is being played, so every preset offers both
const LAYOUT_OPTIONS: &[LayoutOption] = &[
    LayoutOption {
        text: "Spring Embedder",
        algorithm: LayoutAlgorithm::SpringEmbedder,
        visible_preset: GraphPresetFlag::all(),
    },
    LayoutOption {
        text: "Circle",
        algorithm: LayoutAlgorithm::Circle,
        visible_preset: GraphPresetFlag::all(),
    },
];

const DEFAULT_CANVAS_SIZE: V2f = V2f { x: 640.0, y: 400.0 };
const MIN_CANVAS_SIZE: V2f = V2f { x: 240.0, y: 160.0 };

#[derive(Clone)]
struct EditModeInputs {
    option: Mutable<EditOption>,
    edge_vertex: Mutable<EdgeVertexOption>,
    alternating_moves: Mutable<bool>,
}

impl EditModeInputs {
    fn new(preset: GraphPreset) -> EditModeInputs {
        EditModeInputs {
            option: Mutable::new(
                SelectOption::selected_value_idx(0, &preset, EDIT_OPTIONS).unwrap(),
            ),
            edge_vertex: Mutable::new(
                SelectOption::selected_value_idx(0, &preset, EDGE_VERTEX_OPTIONS).unwrap(),
            ),
            alternating_moves: Mutable::new(true),
        }
    }

    fn pass_turn(&self, preset: GraphPreset) {
        if self.alternating_moves.get()
            && let Some(new_mode) = self.option.get().mode.opposite_player()
            && let Some(new_option) =
                SelectOption::find(|o| o.mode == new_mode, &preset, EDIT_OPTIONS)
        {
            self.option.set(new_option);
        }
    }
}

struct GraphWidget {
    preset: GraphPreset,
    edit: EditModeInputs,
    layout: LayoutInputs,
    /// Size of the canvas, which the user is free to resize
    canvas_size: Mutable<V2f>,
    graph: Mutable<SyncState<WidgetGraph>>,
    interactions: Mutable<Interactions>,
}

impl GraphWidget {
    fn new(preset: GraphPreset) -> GraphWidget {
        GraphWidget {
            preset,
            edit: EditModeInputs::new(preset),
            layout: LayoutInputs::new(preset),
            canvas_size: Mutable::new(DEFAULT_CANVAS_SIZE),
            graph: Mutable::new(SyncState::uninitialized(Graph::empty(&[]))),
            interactions: Mutable::new(Interactions::new()),
        }
    }
}

fn clamp_to_canvas(position: V2f, canvas_size: V2f) -> V2f {
    let radius = HtmlCanvas::vertex_radius();
    V2f {
        x: f32::clamp(position.x, radius, f32::max(radius, canvas_size.x - radius)),
        y: f32::clamp(position.y, radius, f32::max(radius, canvas_size.y - radius)),
    }
}

fn vertex_position(graph: &WidgetGraph, vertex: VertexIndex) -> V2f {
    *graph.get_vertex(vertex).get_inner()
}

#[derive(Clone, Copy)]
struct SnortVertex {
    kind: snort::VertexKind,
    position: V2f,
}

impl_has!(SnortVertex -> kind -> snort::VertexKind);

fn snort_move_in<const COLOR: u8>(
    position: &Snort<SnortVertex, UndirectedGraph<SnortVertex>>,
    vertex: VertexIndex,
) -> Option<Snort<SnortVertex, UndirectedGraph<SnortVertex>>> {
    position
        .available_moves_for::<COLOR>()
        .any(|legal| legal == vertex)
        .then(|| position.move_in_vertex::<COLOR>(vertex))
}

#[derive(Clone, Copy)]
struct ColVertex {
    color: col::VertexColor,
    position: V2f,
}

impl_has!(ColVertex -> color -> col::VertexColor);

fn col_move_in<const COLOR: u8>(
    position: &Col<ColVertex, UndirectedGraph<ColVertex>>,
    vertex: VertexIndex,
) -> Option<Col<ColVertex, UndirectedGraph<ColVertex>>> {
    position
        .available_moves_for::<COLOR>()
        .any(|legal| legal == vertex)
        .then(|| position.move_in_vertex::<COLOR>(vertex))
}

#[derive(Clone, Copy)]
struct BipartiteSnortVertex {
    color: bipartite_snort::VertexColor,
    position: V2f,
}

impl_has!(BipartiteSnortVertex -> color -> bipartite_snort::VertexColor);

type BipartiteSnortPosition =
    BipartiteSnort<BipartiteSnortVertex, UndirectedGraph<BipartiteSnortVertex>>;

fn bipartite_snort_move_in<const COLOR: u8>(
    position: &BipartiteSnortPosition,
    vertex: VertexIndex,
) -> Option<BipartiteSnortPosition> {
    position
        .available_moves_for::<COLOR>()
        .any(|legal| legal == vertex)
        .then(|| position.move_in_vertex::<COLOR>(vertex))
}

#[derive(Clone, Copy)]
struct DigraphPlacementVertex {
    color: digraph_placement::VertexColor,
    position: V2f,
}

impl_has!(DigraphPlacementVertex -> color -> digraph_placement::VertexColor);

type DigraphPlacementPosition =
    DigraphPlacement<DigraphPlacementVertex, DirectedGraph<DigraphPlacementVertex>>;

fn digraph_placement_move_in<const COLOR: u8>(
    position: &DigraphPlacementPosition,
    vertex: VertexIndex,
) -> Option<DigraphPlacementPosition> {
    position
        .available_moves_for::<COLOR>()
        .any(|legal| legal == vertex)
        .then(|| position.move_in_vertex(vertex))
}

impl GraphWidget {
    /// Paint the graph and report what the mouse did to it
    fn draw(
        canvas: &HtmlCanvasElement,
        graph: &WidgetGraph,
        canvas_size: V2f,
        interactions: &mut Interactions,
        edit_mode: EditMode,
        preset: GraphPreset,
    ) -> Result<Frame, JsValue> {
        // Resizing a canvas wipes it, so it is only resized when it is the wrong size
        if canvas.width() != canvas_size.x as u32 {
            canvas.set_width(canvas_size.x as u32);
        }
        if canvas.height() != canvas_size.y as u32 {
            canvas.set_height(canvas_size.y as u32);
        }

        let context = canvas
            .get_context("2d")?
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()?;

        let mut canvas = HtmlCanvas::new(context, interactions);
        Ok(canvas.frame(|canvas| {
            // The whole canvas is behind the graph, so that clicking where there is no
            // vertex is an interaction of its own
            let background = canvas.interact(Area::Rect {
                position: V2f::ZERO,
                size: canvas_size,
            });
            canvas.rect(V2f::ZERO, canvas_size, Color::LIGHT_GRAY);

            let vertices = graph.draw(canvas, |canvas, vertex| {
                let position: V2f = *graph.get_vertex(vertex).get_inner();
                let color: VertexColor = *graph.get_vertex(vertex).get_inner();
                canvas.vertex(position, color.color(), vertex)
            });

            if matches!(edit_mode, EditMode::ToggleEdge) {
                GraphWidget::draw_new_edge(canvas, graph, preset, &vertices);
            }

            Frame {
                vertices,
                background,
            }
        }))
    }

    /// Paint the edge that is being dragged out of a vertex. It is painted on top of the
    /// graph, but starts and ends at the rim of the vertices it connects rather than at
    /// their centers, the same way that the edges of the graph itself do
    fn draw_new_edge(
        canvas: &mut HtmlCanvas<'_>,
        graph: &WidgetGraph,
        preset: GraphPreset,
        hits: &Hits<VertexIndex>,
    ) {
        let Some((from, drag)) = hits.dragged else {
            return;
        };

        let target = hits.hovered.filter(|&hovered| hovered != from);
        let from_position = vertex_position(graph, from);
        let to_position = target.map_or(drag.cursor, |target| vertex_position(graph, target));

        let radius = HtmlCanvas::vertex_radius();
        if V2f::distance(from_position, to_position) <= radius {
            return;
        }

        let direction = V2f::direction(from_position, to_position);
        let start = from_position + direction * radius;
        let end = match target {
            Some(_) => to_position - direction * radius,
            None => to_position,
        };

        // Show whether dropping the edge here would add it, remove it, or do nothing.
        // Dropping on empty canvas always lands, so only an existing vertex can refuse
        let color = match target {
            Some(target) if graph.are_adjacent(from, target) => Color::RED,
            Some(target) if !GraphWidget::may_connect(graph, preset, from, target) => {
                Color::DARK_GRAY
            }
            Some(_) | None => Color::BLUE,
        };

        // Arrow head so that it is clear which way around the edge is about to go
        if preset.directed_edges() {
            canvas.arrow(start, end, HtmlCanvas::thin_line_weight(), color);
        } else {
            canvas.line(start, end, HtmlCanvas::thin_line_weight(), color);
        }
    }

    fn add_vertex_at(
        graph: &mut WidgetGraph,
        canvas_size: V2f,
        position: V2f,
        color: VertexColor,
    ) -> VertexIndex {
        graph.add_vertex(Vertex {
            position: clamp_to_canvas(position, canvas_size),
            color,
        })
    }

    fn move_dragged_vertex(
        graph: &Mutable<SyncState<WidgetGraph>>,
        canvas_size: V2f,
        edit_mode: EditMode,
        frame: &Frame,
    ) {
        let Some((vertex, drag)) = frame
            .vertices
            .dragged
            .filter(|_| edit_mode.moves_vertices())
        else {
            return;
        };

        let new_position = clamp_to_canvas(drag.position(), canvas_size);
        let mut graph = graph.lock_mut();
        let moved = vertex_position(&graph.state, vertex) != new_position;

        // Every frame that the pointer holds a vertex down for drags it again, so a vertex
        // that is already where the pointer left it must not be written to: that would
        // report a change, which would paint the frame that reports the drag once more,
        // and so on forever
        if drag.dropped {
            // A drag that took the vertex nowhere is not worth reporting to python
            if moved || graph.is_in_progress() {
                let position: &mut V2f = graph.edit().get_vertex_mut(vertex).get_inner_mut();
                *position = new_position;
            }
        } else if moved {
            let position: &mut V2f = graph
                .edit_in_progress()
                .get_vertex_mut(vertex)
                .get_inner_mut();
            *position = new_position;
        }
    }

    fn apply(
        graph: &Mutable<SyncState<WidgetGraph>>,
        canvas_size: V2f,
        edit: &EditModeInputs,
        preset: GraphPreset,
        frame: &Frame,
    ) {
        let edit_mode = edit.option.get().mode;
        GraphWidget::move_dragged_vertex(graph, canvas_size, edit_mode, frame);

        if let Some(position) = frame.background.clicked
            && let Some(color) = edit_mode.new_vertex_color(edit.edge_vertex.get().color)
        {
            let mut state = graph.lock_mut();
            GraphWidget::add_vertex_at(state.edit(), canvas_size, position, color);
            return;
        }

        match edit_mode {
            EditMode::MoveVertex => {}

            EditMode::AddColorVertex(new_color) => {
                if let Some(vertex) = frame.vertices.clicked {
                    GraphWidget::recolor_vertex(graph, preset, vertex, new_color);
                }
            }

            EditMode::RemoveVertex => {
                if let Some(vertex) = frame.vertices.clicked {
                    graph.lock_mut().edit().remove_vertex(vertex);
                }
            }

            EditMode::ToggleEdge => {
                GraphWidget::drop_edge(
                    graph,
                    canvas_size,
                    preset,
                    frame,
                    edit.edge_vertex.get().color,
                );
            }

            EditMode::GameMove(player) => match preset {
                GraphPreset::Snort => {
                    GraphWidget::snort_move(graph, edit, preset, frame, player);
                }
                GraphPreset::Col => {
                    GraphWidget::col_move(graph, edit, preset, frame, player);
                }
                GraphPreset::DigraphPlacement => {
                    GraphWidget::digraph_placement_move(graph, edit, preset, frame, player)
                }
                GraphPreset::BipartiteSnort => {
                    GraphWidget::bipartite_snort_move(graph, edit, preset, frame, player);
                }
            },
        }
    }

    /// Whether an edge is allowed to join two vertices
    fn may_connect(
        graph: &WidgetGraph,
        preset: GraphPreset,
        from: VertexIndex,
        to: VertexIndex,
    ) -> bool {
        if !preset.bipartite() {
            return true;
        }

        let from: &VertexColor = graph.get_vertex(from).get_inner();
        let to: &VertexColor = graph.get_vertex(to).get_inner();
        from != to
    }

    fn dropped_vertex_color(
        graph: &WidgetGraph,
        preset: GraphPreset,
        from: VertexIndex,
        edge_vertex: Option<VertexColor>,
    ) -> Option<VertexColor> {
        let chosen = edge_vertex?;
        if !preset.bipartite() {
            return Some(chosen);
        }

        let from: &VertexColor = graph.get_vertex(from).get_inner();
        match from {
            VertexColor::Blue => Some(VertexColor::Red),
            VertexColor::Red => Some(VertexColor::Blue),
            VertexColor::White | VertexColor::Green => None,
        }
    }

    fn may_recolor(
        graph: &WidgetGraph,
        preset: GraphPreset,
        vertex: VertexIndex,
        new_color: VertexColor,
    ) -> bool {
        if !preset.bipartite() {
            return true;
        }

        graph.adjacent_to(vertex).all(|adjacent| {
            let color: &VertexColor = graph.get_vertex(adjacent).get_inner();
            adjacent == vertex || *color != new_color
        })
    }

    /// Connect or disconnect two vertices. Games played on undirected graphs store an edge
    /// as a pair of opposite arcs, so for them the edge is dragged out in both directions
    /// at once
    fn connect(
        graph: &mut WidgetGraph,
        preset: GraphPreset,
        from: VertexIndex,
        to: VertexIndex,
        connect: bool,
    ) {
        graph.connect(from, to, connect);
        if !preset.directed_edges() {
            graph.connect(to, from, connect);
        }
    }

    /// Repaint a vertex, which is nothing worth reporting if it already had that color and
    /// nothing the preset allows if the new color is one a neighbour is already in
    fn recolor_vertex(
        graph: &Mutable<SyncState<WidgetGraph>>,
        preset: GraphPreset,
        vertex: VertexIndex,
        new_color: VertexColor,
    ) {
        let mut graph = graph.lock_mut();
        if !GraphWidget::may_recolor(&graph.state, preset, vertex, new_color) {
            return;
        }

        let color: &VertexColor = graph.state.get_vertex(vertex).get_inner();
        if *color != new_color {
            let color: &mut VertexColor = graph.edit().get_vertex_mut(vertex).get_inner_mut();
            *color = new_color;
        }
    }

    fn drop_edge(
        graph: &Mutable<SyncState<WidgetGraph>>,
        canvas_size: V2f,
        preset: GraphPreset,
        frame: &Frame,
        edge_vertex: Option<VertexColor>,
    ) {
        // Clicking a vertex rather than dragging an edge out of it paints it the color
        // that dropping an edge on empty canvas would have created, and a dropdown set to
        // create nothing paints nothing
        if let Some(vertex) = frame.vertices.clicked {
            if let Some(new_color) = edge_vertex {
                GraphWidget::recolor_vertex(graph, preset, vertex, new_color);
            }

            return;
        }

        let Some((from, drag)) = frame.vertices.dragged.filter(|(_, drag)| drag.dropped) else {
            return;
        };

        match frame.vertices.hovered {
            Some(target) if target == from => {}
            Some(target) => {
                let mut state = graph.lock_mut();
                let adjacent = state.state.are_adjacent(from, target);

                if !adjacent && !GraphWidget::may_connect(&state.state, preset, from, target) {
                    return;
                }

                let graph = state.edit();
                GraphWidget::connect(graph, preset, from, target, !adjacent);
            }
            // Dropped on empty canvas, so the edge only lands somewhere if the dropdown
            // is set to leave a vertex behind
            None => {
                let color = GraphWidget::dropped_vertex_color(
                    &graph.lock_ref().state,
                    preset,
                    from,
                    edge_vertex,
                );
                let Some(color) = color else {
                    return;
                };

                let mut state = graph.lock_mut();
                let graph = state.edit();
                let target = GraphWidget::add_vertex_at(graph, canvas_size, drag.cursor, color);
                GraphWidget::connect(graph, preset, from, target, true);
            }
        }
    }

    /// Rearrange the whole graph with the chosen algorithm. Parameters that were never
    /// touched by hand are recomputed first, since the graph they were last filled in for
    /// is not the graph being laid out now
    fn apply_layout(
        graph: &Mutable<SyncState<WidgetGraph>>,
        canvas_size: V2f,
        inputs: &LayoutInputs,
    ) {
        let vertices = graph.lock_ref().state.size();

        if !inputs.customized.get() {
            inputs.show_defaults(vertices, canvas_size);
        }

        let mut state = graph.lock_mut();
        let graph = state.edit();

        match inputs.algorithm.get().algorithm {
            LayoutAlgorithm::Circle => inputs.circle().layout(graph),
            LayoutAlgorithm::SpringEmbedder => inputs.spring(vertices, canvas_size).layout(graph),
        }

        // Parameters of one's own choosing are left to put vertices wherever they put
        // them, but a layout that diverged has to be caught: a position that is not a
        // number would not even survive being sent back to python
        for vertex in graph.vertex_indices() {
            let position: &mut V2f = graph.get_vertex_mut(vertex).get_inner_mut();
            if !position.x.is_finite() {
                position.x = canvas_size.x * 0.5;
            }
            if !position.y.is_finite() {
                position.y = canvas_size.y * 0.5;
            }
        }
    }

    fn snort_move(
        graph: &Mutable<SyncState<WidgetGraph>>,
        edit: &EditModeInputs,
        preset: GraphPreset,
        frame: &Frame,
        player: Player,
    ) {
        let Some(clicked) = frame.vertices.clicked else {
            return;
        };

        let Ok(snort_graph) = graph.lock_ref().state.try_map(|vertex| {
            snort::VertexColor::try_from(vertex.color).map(|color| SnortVertex {
                kind: snort::VertexKind::Single(color),
                position: vertex.position,
            })
        }) else {
            // Should be unreachable
            return;
        };

        let position = Snort::new(UndirectedGraph::from_directed(&snort_graph));
        let new_position = match player {
            Player::Left => {
                snort_move_in::<{ snort::VertexColor::TintLeft as u8 }>(&position, clicked)
            }
            Player::Right => {
                snort_move_in::<{ snort::VertexColor::TintRight as u8 }>(&position, clicked)
            }
        };

        let Some(new_position) = new_position else {
            return;
        };

        set_edited(
            graph,
            new_position.graph.as_directed().map(|vertex| Vertex {
                position: vertex.position,
                color: VertexColor::from(vertex.kind.color()),
            }),
        );
        edit.pass_turn(preset);
    }

    fn col_move(
        graph: &Mutable<SyncState<WidgetGraph>>,
        edit: &EditModeInputs,
        preset: GraphPreset,
        frame: &Frame,
        player: Player,
    ) {
        let Some(clicked) = frame.vertices.clicked else {
            return;
        };

        let Ok(col_graph) = graph.lock_ref().state.try_map(|vertex| {
            col::VertexColor::try_from(vertex.color).map(|color| ColVertex {
                color,
                position: vertex.position,
            })
        }) else {
            // Reachable only if the graph got a vertex of a color that the game has no
            // move for, e.g. by editing it as another game first
            return;
        };

        let position = Col::new(UndirectedGraph::from_directed(&col_graph));
        let new_position = match player {
            Player::Left => col_move_in::<{ col::VertexColor::TintLeft as u8 }>(&position, clicked),
            Player::Right => {
                col_move_in::<{ col::VertexColor::TintRight as u8 }>(&position, clicked)
            }
        };

        let Some(new_position) = new_position else {
            return;
        };

        set_edited(
            graph,
            new_position.graph.as_directed().map(|vertex| Vertex {
                position: vertex.position,
                color: VertexColor::from(vertex.color),
            }),
        );
        edit.pass_turn(preset);
    }

    fn bipartite_snort_move(
        graph: &Mutable<SyncState<WidgetGraph>>,
        edit: &EditModeInputs,
        preset: GraphPreset,
        frame: &Frame,
        player: Player,
    ) {
        let Some(clicked) = frame.vertices.clicked else {
            return;
        };

        let Ok(snort_graph) = graph.lock_ref().state.try_map(|vertex| {
            bipartite_snort::VertexColor::try_from(vertex.color).map(|color| BipartiteSnortVertex {
                color,
                position: vertex.position,
            })
        }) else {
            return;
        };

        let position = BipartiteSnort::new(UndirectedGraph::from_directed(&snort_graph));
        let new_position = match player {
            Player::Left => bipartite_snort_move_in::<
                { bipartite_snort::VertexColor::TintLeft as u8 },
            >(&position, clicked),
            Player::Right => bipartite_snort_move_in::<
                { bipartite_snort::VertexColor::TintRight as u8 },
            >(&position, clicked),
        };

        let Some(new_position) = new_position else {
            return;
        };

        set_edited(
            graph,
            new_position.graph.as_directed().map(|vertex| Vertex {
                position: vertex.position,
                color: VertexColor::from(vertex.color),
            }),
        );
        edit.pass_turn(preset);
    }

    fn digraph_placement_move(
        graph: &Mutable<SyncState<WidgetGraph>>,
        edit: &EditModeInputs,
        preset: GraphPreset,
        frame: &Frame,
        player: Player,
    ) {
        let Some(clicked) = frame.vertices.clicked else {
            return;
        };

        let Ok(placement_graph) = graph.lock_ref().state.try_map(|vertex| {
            digraph_placement::VertexColor::try_from(vertex.color).map(|color| {
                DigraphPlacementVertex {
                    color,
                    position: vertex.position,
                }
            })
        }) else {
            // Reachable only if the graph got a vertex of a color that the game has no
            // move for, e.g. by editing it as another game first
            return;
        };

        let position = DigraphPlacement::new(placement_graph);
        let new_position = match player {
            Player::Left => digraph_placement_move_in::<
                { digraph_placement::VertexColor::Left as u8 },
            >(&position, clicked),
            Player::Right => digraph_placement_move_in::<
                { digraph_placement::VertexColor::Right as u8 },
            >(&position, clicked),
        };

        let Some(new_position) = new_position else {
            return;
        };

        set_edited(
            graph,
            new_position.graph.map(|vertex| Vertex {
                position: vertex.position,
                color: VertexColor::from(vertex.color),
            }),
        );
        edit.pass_turn(preset);
    }

    fn update(
        canvas: &HtmlCanvasElement,
        graph: &Mutable<SyncState<WidgetGraph>>,
        canvas_size: &Mutable<V2f>,
        interactions: &Mutable<Interactions>,
        edit: &EditModeInputs,
        preset: GraphPreset,
    ) -> Result<(), JsValue> {
        let frame = GraphWidget::draw(
            canvas,
            &graph.lock_ref().state,
            canvas_size.get(),
            &mut interactions.lock_mut(),
            edit.option.get().mode,
            preset,
        )?;
        GraphWidget::apply(graph, canvas_size.get(), edit, preset, &frame);

        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct EditOption {
    text: &'static str,
    mode: EditMode,
    visible_preset: GraphPresetFlag,
}

impl SelectOptionElement for EditOption {
    type Preset = GraphPreset;

    fn text(&self) -> &str {
        self.text
    }

    fn is_visible(&self, preset: &Self::Preset) -> bool {
        preset.intersects(self.visible_preset)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct EdgeVertexOption {
    text: &'static str,
    color: Option<VertexColor>,
    visible_preset: GraphPresetFlag,
}

impl SelectOptionElement for EdgeVertexOption {
    type Preset = GraphPreset;

    fn text(&self) -> &str {
        self.text
    }

    fn is_visible(&self, preset: &Self::Preset) -> bool {
        preset.intersects(self.visible_preset)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct LayoutOption {
    text: &'static str,
    algorithm: LayoutAlgorithm,
    visible_preset: GraphPresetFlag,
}

impl SelectOptionElement for LayoutOption {
    type Preset = GraphPreset;

    fn text(&self) -> &str {
        self.text
    }

    fn is_visible(&self, preset: &Self::Preset) -> bool {
        preset.intersects(self.visible_preset)
    }
}

/// A parameter row of the layout panel, the label naming it and the input holding it
fn parameter_row(
    document: &Document,
    label: &str,
    input_type: &str,
) -> Result<(HtmlLabelElement, HtmlInputElement), JsValue> {
    let row = document
        .create_element("label")?
        .dyn_into::<HtmlLabelElement>()?;
    row.style().set_property("display", "flex")?;
    row.style().set_property("align-items", "center")?;
    row.style()
        .set_property("justify-content", "space-between")?;
    row.style().set_property("gap", "8px")?;

    let text = document.create_element("span")?;
    text.set_text_content(Some(label));

    let input = document
        .create_element("input")?
        .dyn_into::<HtmlInputElement>()?;
    input.set_type(input_type);

    row.append_child(&text)?;
    row.append_child(&input)?;
    Ok((row, input))
}

/// A parameter row holding a number, shown with `decimals` digits after the point
fn number_row(
    document: &Document,
    label: &str,
    step: &str,
    decimals: usize,
    value: &Mutable<f32>,
) -> Result<HtmlLabelElement, JsValue> {
    let (row, input) = parameter_row(document, label, "number")?;
    input.set_step(step);
    input.set_min("0");
    input.style().set_property("width", "7em")?;
    reactive::input_f32(&input, value, decimals)?;

    Ok(row)
}

/// A parameter row holding a flag
fn checkbox_row(
    document: &Document,
    label: &str,
    value: &Mutable<bool>,
) -> Result<HtmlLabelElement, JsValue> {
    let (row, input) = parameter_row(document, label, "checkbox")?;
    reactive::checkbox(&input, value)?;

    Ok(row)
}

/// The controls that pick a layout and run it, along with the collapsed panel holding the
/// parameters of both algorithms
fn layout_controls(
    document: &Document,
    preset: GraphPreset,
    inputs: &LayoutInputs,
    graph: &Mutable<SyncState<WidgetGraph>>,
    canvas_size: &Mutable<V2f>,
) -> Result<HtmlElement, JsValue> {
    let root = document.create_element("div")?.dyn_into::<HtmlElement>()?;
    root.style().set_property("margin-bottom", "4px")?;

    let row = document
        .create_element("div")?
        .dyn_into::<HtmlDivElement>()?;
    row.style().set_property("display", "flex")?;
    row.style().set_property("align-items", "center")?;
    row.style().set_property("width", "fit-content")?;
    row.style().set_property("gap", "8px")?;

    let label = document.create_element("span")?;
    label.set_text_content(Some("Layout"));
    let algorithm = SelectOption::create_element_reactive(
        document,
        "Layout",
        preset,
        LAYOUT_OPTIONS,
        inputs.algorithm.clone(),
    )?;

    let apply = document
        .create_element("button")?
        .dyn_into::<HtmlButtonElement>()?;
    apply.set_type("button");
    apply.set_text_content(Some("Apply"));

    let apply_handler = ScopedClosure::<dyn FnMut()>::new({
        let inputs = inputs.clone();
        let graph = graph.clone();
        let canvas_size = canvas_size.clone();
        move || GraphWidget::apply_layout(&graph, canvas_size.get(), &inputs)
    });
    apply.add_event_listener_with_callback("click", apply_handler.as_ref().unchecked_ref())?;
    apply_handler.forget();

    row.append_child(&label)?;
    row.append_child(&algorithm)?;
    row.append_child(&apply)?;

    let parameters = document
        .create_element("div")?
        .dyn_into::<HtmlDivElement>()?;
    parameters.style().set_property("display", "flex")?;
    parameters
        .style()
        .set_property("flex-direction", "column")?;
    parameters.style().set_property("width", "fit-content")?;
    parameters.style().set_property("gap", "4px")?;
    parameters.style().set_property("padding", "4px 0 0 8px")?;

    let circle_rows = [
        number_row(document, "Radius", "1", 1, &inputs.circle_radius)?,
        number_row(document, "Center X", "1", 1, &inputs.center_x)?,
        number_row(document, "Center Y", "1", 1, &inputs.center_y)?,
    ];
    let spring_rows = [
        number_row(document, "Iterations", "128", 0, &inputs.iterations)?,
        number_row(document, "Cooling rate", "0.0001", 6, &inputs.cooling_rate)?,
        number_row(document, "Attraction", "0.1", 2, &inputs.c_attractive)?,
        number_row(document, "Repulsion", "10", 1, &inputs.c_repulsive)?,
        number_row(
            document,
            "Ideal edge length",
            "1",
            1,
            &inputs.ideal_spring_length,
        )?,
        number_row(
            document,
            "Pull to center",
            "0.001",
            4,
            &inputs.c_middle_attractive,
        )?,
        checkbox_row(document, "Keep on canvas", &inputs.keep_on_canvas)?,
    ];

    // Only the parameters of the chosen algorithm are worth showing
    let circle = circle_rows.iter().map(|row| (row, LayoutAlgorithm::Circle));
    let spring = spring_rows
        .iter()
        .map(|row| (row, LayoutAlgorithm::SpringEmbedder));
    for (row, shown_for) in circle.chain(spring) {
        parameters.append_child(row)?;
        reactive::style_set_property(
            HtmlElement::from(row.clone()),
            "display",
            inputs.algorithm.signal().map(move |option| {
                if option.algorithm == shown_for {
                    "flex"
                } else {
                    "none"
                }
            }),
        )?;
    }

    // Typing into any parameter hands the whole panel over to the user, so that the
    // defaults stop being recomputed underneath what they entered
    let customize_handler = ScopedClosure::<dyn FnMut()>::new({
        let customized = inputs.customized.clone();
        move || customized.set_neq(true)
    });
    parameters
        .add_event_listener_with_callback("input", customize_handler.as_ref().unchecked_ref())?;
    customize_handler.forget();

    let reset = document
        .create_element("button")?
        .dyn_into::<HtmlButtonElement>()?;
    reset.set_type("button");
    reset.set_text_content(Some("Reset to defaults"));
    reset.style().set_property("margin-top", "4px")?;
    parameters.append_child(&reset)?;

    let reset_handler = ScopedClosure::<dyn FnMut()>::new({
        let inputs = inputs.clone();
        let graph = graph.clone();
        let canvas_size = canvas_size.clone();
        move || {
            inputs.customized.set_neq(false);
            inputs.show_defaults(graph.lock_ref().state.size(), canvas_size.get());
        }
    });
    reset.add_event_listener_with_callback("click", reset_handler.as_ref().unchecked_ref())?;
    reset_handler.forget();

    let details = document
        .create_element("details")?
        .dyn_into::<HtmlElement>()?;
    details.style().set_property("margin-top", "4px")?;
    let summary = document.create_element("summary")?;
    summary.set_text_content(Some("Layout parameters"));
    details.append_child(&summary)?;
    details.append_child(&parameters)?;

    // The panel is opened onto the graph as it is now, not as it was when the widget did
    let open_handler = ScopedClosure::<dyn FnMut()>::new({
        let inputs = inputs.clone();
        let graph = graph.clone();
        let canvas_size = canvas_size.clone();
        move || {
            if !inputs.customized.get() {
                inputs.show_defaults(graph.lock_ref().state.size(), canvas_size.get());
            }
        }
    });
    details.add_event_listener_with_callback("toggle", open_handler.as_ref().unchecked_ref())?;
    open_handler.forget();

    root.append_child(&row)?;
    root.append_child(&details)?;

    Ok(root)
}

/// The parameters of the layout panel. Every parameter of both algorithms is here except
/// the vertex radius and the bounding rectangle, which are what the canvas says they are
#[derive(Clone)]
struct LayoutInputs {
    algorithm: Mutable<LayoutOption>,

    circle_radius: Mutable<f32>,
    center_x: Mutable<f32>,
    center_y: Mutable<f32>,

    iterations: Mutable<f32>,
    cooling_rate: Mutable<f32>,
    c_attractive: Mutable<f32>,
    c_repulsive: Mutable<f32>,
    ideal_spring_length: Mutable<f32>,
    c_middle_attractive: Mutable<f32>,
    keep_on_canvas: Mutable<bool>,

    /// Whether the parameters were touched by hand. Until they are they follow the graph,
    /// so that laying out a graph drawn after the widget opened still fits it
    customized: Mutable<bool>,
}

impl LayoutInputs {
    fn new(preset: GraphPreset) -> LayoutInputs {
        let inputs = LayoutInputs {
            algorithm: Mutable::new(
                SelectOption::selected_value_idx(0, &preset, LAYOUT_OPTIONS).unwrap(),
            ),
            circle_radius: Mutable::new(0.0),
            center_x: Mutable::new(0.0),
            center_y: Mutable::new(0.0),
            iterations: Mutable::new(0.0),
            cooling_rate: Mutable::new(0.0),
            c_attractive: Mutable::new(0.0),
            c_repulsive: Mutable::new(0.0),
            ideal_spring_length: Mutable::new(0.0),
            c_middle_attractive: Mutable::new(0.0),
            keep_on_canvas: Mutable::new(false),
            customized: Mutable::new(false),
        };

        // An empty graph on a canvas of the size the widget opens at is all there is to go
        // on until python sends a graph down
        inputs.show_defaults(0, DEFAULT_CANVAS_SIZE);
        inputs
    }

    /// Fill every parameter with the one that suits the graph as it is right now
    fn show_defaults(&self, vertices: usize, canvas_size: V2f) {
        let circle = default_circle::<HtmlCanvas>(canvas_size);
        self.circle_radius.set_neq(circle.circle_radius);
        self.center_x.set_neq(circle.center.x);
        self.center_y.set_neq(circle.center.y);

        let spring = default_spring::<HtmlCanvas>(vertices, canvas_size);
        self.iterations.set_neq(spring.iterations as f32);
        self.cooling_rate.set_neq(spring.cooling_rate);
        self.c_attractive.set_neq(spring.c_attractive);
        self.c_repulsive.set_neq(spring.c_repulsive);
        self.ideal_spring_length.set_neq(spring.ideal_spring_length);
        self.c_middle_attractive.set_neq(
            spring
                .bounds
                .and_then(|bounds| bounds.c_middle_attractive)
                .unwrap_or(0.0),
        );
        self.keep_on_canvas.set_neq(spring.bounds.is_some());
    }

    fn circle(&self) -> CircleEdge {
        CircleEdge {
            circle_radius: self.circle_radius.get(),
            vertex_radius: HtmlCanvas::vertex_radius(),
            center: V2f {
                x: self.center_x.get(),
                y: self.center_y.get(),
            },
        }
    }

    fn spring(&self, vertices: usize, canvas_size: V2f) -> SpringEmbedder {
        // A hand typed iteration count still has to leave the page responsive
        let iterations = usize::min(
            self.iterations.get().max(0.0) as usize,
            max_spring_iterations(vertices),
        );

        SpringEmbedder {
            cooling_rate: self.cooling_rate.get(),
            c_attractive: self.c_attractive.get(),
            c_repulsive: self.c_repulsive.get(),
            ideal_spring_length: self.ideal_spring_length.get(),
            iterations,
            bounds: self
                .keep_on_canvas
                .get()
                .then(|| default_bounds::<HtmlCanvas>(canvas_size, self.c_middle_attractive.get())),
        }
    }
}

impl WasmWidget for GraphWidget {
    type BackendMessage = GraphBackendMessage;
    type FrontendMessage = GraphFrontendMessage;

    fn handle_message(&mut self, message: Self::FrontendMessage) -> Result<(), JsValue> {
        match message {
            GraphFrontendMessage::SetGraph {
                sequence,
                graph: new_graph,
            } => {
                SyncState::take_from_python(&self.graph, sequence, new_graph);

                Ok(())
            }
        }
    }

    fn mount(
        &mut self,
        context: Context<GraphBackendMessage>,
        element: Element,
    ) -> Result<(), JsValue> {
        let document = web_sys::window().unwrap().document().unwrap();

        let controls = document
            .create_element("div")?
            .dyn_into::<HtmlDivElement>()?;
        controls.style().set_property("display", "flex")?;
        controls.style().set_property("flex-direction", "row")?;
        controls.style().set_property("align-items", "center")?;
        controls.style().set_property("width", "fit-content")?;
        controls.style().set_property("gap", "8px")?;
        controls.style().set_property("margin-bottom", "4px")?;

        let mode_select = SelectOption::create_element_reactive(
            &document,
            "Edit mode",
            self.preset,
            EDIT_OPTIONS,
            self.edit.option.clone(),
        )?;

        let edge_options = document
            .create_element("label")?
            .dyn_into::<HtmlLabelElement>()?;
        edge_options.style().set_property("align-items", "center")?;
        edge_options.style().set_property("gap", "4px")?;

        let edge_vertex = SelectOption::create_element_reactive(
            &document,
            "Edge vertex",
            self.preset,
            EDGE_VERTEX_OPTIONS,
            self.edit.edge_vertex.clone(),
        )?;
        let edge_options_text = document.create_element("span")?;
        edge_options_text.set_text_content(Some("New vertex"));
        edge_options.append_child(&edge_options_text)?;
        edge_options.append_child(&edge_vertex)?;

        let move_options = document
            .create_element("label")?
            .dyn_into::<HtmlLabelElement>()?;
        move_options.style().set_property("align-items", "center")?;
        move_options.style().set_property("gap", "4px")?;

        let alternating_moves = document
            .create_element("input")?
            .dyn_into::<HtmlInputElement>()?;
        alternating_moves.set_type("checkbox");
        reactive::checkbox(&alternating_moves, &self.edit.alternating_moves)?;

        let move_options_text = document.create_element("span")?;
        move_options_text.set_text_content(Some("Alternating Moves"));
        move_options.append_child(&alternating_moves)?;
        move_options.append_child(&move_options_text)?;

        // Show only those of the extra controls that the mode has a use for
        reactive::style_set_property(
            HtmlElement::from(edge_options.clone()),
            "display",
            self.edit.option.signal().map(|option| {
                if matches!(option.mode, EditMode::ToggleEdge) {
                    "flex"
                } else {
                    "none"
                }
            }),
        )?;
        reactive::style_set_property(
            HtmlElement::from(move_options.clone()),
            "display",
            self.edit.option.signal().map(|option| {
                if option.mode.opposite_player().is_some() {
                    "flex"
                } else {
                    "none"
                }
            }),
        )?;

        controls.append_child(&mode_select)?;
        controls.append_child(&edge_options)?;
        controls.append_child(&move_options)?;
        element.append_child(&controls)?;

        let layout = layout_controls(
            &document,
            self.preset,
            &self.layout,
            &self.graph,
            &self.canvas_size,
        )?;
        element.append_child(&layout)?;

        let canvas_container = document
            .create_element("div")?
            .dyn_into::<HtmlDivElement>()?;
        let container_style = canvas_container.style();
        container_style.set_property("resize", "both")?;
        container_style.set_property("overflow", "hidden")?;
        container_style.set_property("width", &format!("{}px", DEFAULT_CANVAS_SIZE.x as u32))?;
        container_style.set_property("height", &format!("{}px", DEFAULT_CANVAS_SIZE.y as u32))?;
        container_style.set_property("min-width", &format!("{}px", MIN_CANVAS_SIZE.x as u32))?;
        container_style.set_property("min-height", &format!("{}px", MIN_CANVAS_SIZE.y as u32))?;

        let canvas = document
            .create_element("canvas")?
            .dyn_into::<HtmlCanvasElement>()?;
        canvas.style().set_property("display", "block")?;
        canvas.style().set_property("width", "100%")?;
        canvas.style().set_property("height", "100%")?;
        canvas.style().set_property("user-select", "none")?;
        reactive::canvas_interactions(&canvas, &self.interactions)?;

        canvas_container.append_child(&canvas)?;
        element.append_child(&canvas_container)?;
        reactive::element_size(&canvas_container, &self.canvas_size)?;

        reactive::frames(
            map_ref! {
                let _graph = self.graph.signal_ref(|_| ()),
                let _canvas_size = self.canvas_size.signal(),
                let _edit_mode = self.edit.option.signal().dedupe(),
                let _edit_mode = self.edit.edge_vertex.signal().dedupe() => ()
            },
            &self.interactions,
            {
                let canvas = canvas.clone();
                let graph = self.graph.clone();
                let canvas_size = self.canvas_size.clone();
                let interactions = self.interactions.clone();
                let edit = self.edit.clone();
                let preset = self.preset;
                move || {
                    GraphWidget::update(&canvas, &graph, &canvas_size, &interactions, &edit, preset)
                }
            },
        );

        report_edits_to_python(&self.graph, &context, |sequence, graph| {
            GraphBackendMessage::SetGraph { sequence, graph }
        });

        context.send_message(&GraphBackendMessage::Initialized);

        Ok(())
    }
}

#[wasm_bindgen]
pub fn render_graph_widget_impl(
    model: AnyWidgetModel,
    el: Element,
    raw_preset: u32,
) -> Result<(), JsValue> {
    let preset = GraphPreset::from_flag_bits(raw_preset).unwrap();
    let widget = GraphWidget::new(preset);
    widget.render(model, el)
}
