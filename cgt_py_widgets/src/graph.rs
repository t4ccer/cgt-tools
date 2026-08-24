use cgt::{
    drawing::{Area, Button, Canvas, Color, Hits, Interaction, Interactions},
    graph::{
        Graph, VertexIndex,
        adjacency_matrix::{directed::DirectedGraph, undirected::UndirectedGraph},
        layout::{Bounds, CircleEdge, SpringEmbedder},
    },
    has::Has,
    impl_has,
    numeric::v2f::V2f,
    short::partizan::{
        Player,
        games::{
            digraph_placement::{self, DigraphPlacement},
            snort::{self, Snort},
        },
    },
};
use cgt_py_messages::{
    GraphBackendMessage, GraphFrontendMessage, GraphPreset, GraphPresetFlag, Vertex, VertexColor,
    WidgetGraph,
};
use jupyter_rust_widget_frontend::{AnyWidgetModel, Context, WasmWidget};
use std::sync::{Arc, Mutex};
use wasm_bindgen::{
    JsCast, JsValue,
    prelude::{ScopedClosure, wasm_bindgen},
};
use web_sys::{
    CanvasRenderingContext2d, Document, Element, HtmlButtonElement, HtmlCanvasElement,
    HtmlDivElement, HtmlElement, HtmlInputElement, HtmlLabelElement, HtmlSelectElement, MouseEvent,
    ResizeObserver,
};

use crate::{SelectOption, SelectOptionElement, canvas::HtmlCanvas};

struct HtmlState {
    canvas: HtmlCanvasElement,
    edit_mode: HtmlSelectElement,

    /// Kept around only so that it keeps observing the canvas container
    _resize_observer: ResizeObserver,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditMode {
    MoveVertex,
    ToggleEdge(Option<VertexColor>),
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
    const fn new_vertex_color(self) -> Option<VertexColor> {
        match self {
            EditMode::ToggleEdge(edge_vertex) => edge_vertex,
            EditMode::AddColorVertex(color) => Some(color),
            EditMode::MoveVertex | EditMode::RemoveVertex | EditMode::GameMove(_) => None,
        }
    }

    /// Whether vertices follow the pointer that drags them
    const fn moves_vertices(self) -> bool {
        match self {
            EditMode::MoveVertex | EditMode::AddColorVertex(_) => true,
            EditMode::ToggleEdge(_) | EditMode::RemoveVertex | EditMode::GameMove(_) => false,
        }
    }

    /// The same mode but played by the other player, if the mode plays moves at all
    const fn opposite_player(self) -> Option<EditMode> {
        match self {
            EditMode::MoveVertex
            | EditMode::ToggleEdge(_)
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

impl Frame {
    fn new() -> Self {
        Self {
            vertices: Hits::new(),
            background: Interaction::NONE,
        }
    }
}

/// What applying a [`Frame`] to the graph did
#[derive(Clone, Copy)]
struct Applied {
    /// The graph was modified, so the canvas has to be painted again
    changed: bool,

    /// The change is finished, so the backend should hear about it. Vertices being dragged
    /// around change on every frame but are only worth sending once they are dropped
    committed: bool,
}

impl Applied {
    fn none() -> Self {
        Self {
            changed: false,
            committed: false,
        }
    }
}

struct SharedState {
    preset: GraphPreset,
    edit_mode: EditMode,
    /// Whether the same player keeps moving instead of the players taking turns
    consecutive_moves: bool,
    /// Whether the layout parameters were touched by hand. Until they are they follow the
    /// graph, so that laying out a graph drawn after the widget opened still fits it
    layout_customized: bool,
    state: Option<HtmlState>,
    canvas_size: V2f,
    graph: WidgetGraph,
    interactions: Interactions,
}

const UNCOLORED_VERTEX: GraphPresetFlag =
    GraphPresetFlag::from_slice(&[GraphPresetFlag::Snort, GraphPresetFlag::Col]);

const PLAYER_VERTEX: GraphPresetFlag = GraphPresetFlag::from_slice(&[
    GraphPresetFlag::Snort,
    GraphPresetFlag::Col,
    GraphPresetFlag::DigraphPlacement,
]);

const GREEN_VERTEX: GraphPresetFlag = GraphPresetFlag::from_slice(&[]);

const PLAYABLE: GraphPresetFlag =
    GraphPresetFlag::from_slice(&[GraphPresetFlag::Snort, GraphPresetFlag::DigraphPlacement]);

const EDIT_OPTIONS: &[EditOption] = &[
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
        text: "Add/Remove Edge",
        mode: EditMode::ToggleEdge(None),
        visible_preset: GraphPresetFlag::all(),
    },
    EditOption {
        text: "Remove Vertex",
        mode: EditMode::RemoveVertex,
        visible_preset: GraphPresetFlag::all(),
    },
    // TODO: Col moves
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

/// Biggest circle the canvas holds. Growing it further to give a crowded graph more
/// circumference would only push vertices off the canvas, where they cannot be reached, so
/// a graph too big for its canvas is left overlapping until the canvas is dragged larger
fn default_circle(canvas_size: V2f) -> CircleEdge {
    CircleEdge {
        circle_radius: f32::min(canvas_size.x, canvas_size.y) * 0.5,
        vertex_radius: HtmlCanvas::vertex_radius(),
        center: V2f {
            x: canvas_size.x * 0.5,
            y: canvas_size.y * 0.5,
        },
    }
}

fn default_spring(vertices: usize, canvas_size: V2f) -> SpringEmbedder {
    let vertex_radius = HtmlCanvas::vertex_radius();

    // Every iteration walks every pair of vertices, so cap the total work to keep a big
    // graph from freezing the page for the whole time the button is held down
    let iterations = usize::clamp(
        MAX_LAYOUT_WORK / usize::max(vertices * vertices, 1),
        256,
        4096,
    );

    // Give every vertex its own patch of canvas to spread out into, but never ask for
    // less room than it takes to tell two of them apart
    let area = canvas_size.x * canvas_size.y;
    let ideal_spring_length = f32::max(
        f32::sqrt(area / f32::max(vertices as f32, 1.0)) * 0.5,
        vertex_radius * 3.0,
    );

    SpringEmbedder {
        // Cool down to almost nothing by the last iteration so that the layout settles
        cooling_rate: f32::powf(0.01, 1.0 / iterations as f32),
        c_attractive: 1.0,
        // Holds the balance struck by the values the svg rendering was tuned with, where
        // a repulsion of 250 went with an ideal length of 40
        c_repulsive: (250.0 / (40.0 * 40.0)) * ideal_spring_length * ideal_spring_length,
        ideal_spring_length,
        iterations,
        bounds: Some(canvas_bounds(canvas_size, 0.001)),
    }
}

/// Keeps the layout within the part of the canvas where a vertex can still be clicked
fn canvas_bounds(canvas_size: V2f, c_middle_attractive: f32) -> Bounds {
    let vertex_radius = HtmlCanvas::vertex_radius();
    Bounds {
        lower: V2f {
            x: vertex_radius,
            y: vertex_radius,
        },
        // A canvas too small to have an inside would give the bounds a lower edge above
        // their upper one, which clamping refuses to do
        upper: V2f {
            x: f32::max(vertex_radius, canvas_size.x - vertex_radius),
            y: f32::max(vertex_radius, canvas_size.y - vertex_radius),
        },
        c_middle_attractive: Some(c_middle_attractive),
    }
}

/// Ceiling on `iterations * vertices^2` that the default iteration count aims for, and the
/// ceiling that a hand typed iteration count is held to
const MAX_LAYOUT_WORK: usize = 1 << 22;

const DEFAULT_CANVAS_SIZE: V2f = V2f { x: 640.0, y: 400.0 };
const MIN_CANVAS_SIZE: V2f = V2f { x: 240.0, y: 160.0 };

struct GraphWidget {
    preset: GraphPreset,
    shared: Arc<Mutex<SharedState>>,
}

impl GraphWidget {
    fn new(preset: GraphPreset) -> GraphWidget {
        // Both dropdowns start on their first option, which is not the same one for every
        // preset, so the widget has to start on whatever those turn out to be
        let edit_mode = SelectOption::selected_value("0", &preset, EDIT_OPTIONS)
            .map_or(EditMode::MoveVertex, |option| option.mode);

        GraphWidget {
            preset,
            shared: Arc::new(Mutex::new(SharedState {
                preset,
                edit_mode,
                consecutive_moves: false,
                layout_customized: false,
                state: None,
                canvas_size: DEFAULT_CANVAS_SIZE,
                graph: Graph::empty(&[]),
                interactions: Interactions::new(),
            })),
        }
    }
}

fn mouse_event_to_canvas(event: &MouseEvent) -> V2f {
    V2f {
        x: event.offset_x() as f32,
        y: event.offset_y() as f32,
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
    fn sync_edit_mode(this: &SharedState) {
        let Some(state) = &this.state else {
            return;
        };

        let current_mode =
            SelectOption::selected_value(&state.edit_mode.value(), &this.preset, EDIT_OPTIONS);
        if current_mode.map(|o| o.mode) != Some(this.edit_mode)
            && let Some(mode) =
                SelectOption::find_index(|o| o.mode == this.edit_mode, &this.preset, EDIT_OPTIONS)
        {
            state.edit_mode.set_value(&mode.to_string());
        }
    }

    /// Paint the graph and report what the mouse did to it
    fn draw(this: &mut SharedState) -> Result<Frame, JsValue> {
        GraphWidget::sync_edit_mode(this);

        let SharedState {
            state,
            canvas_size,
            graph,
            interactions,
            edit_mode,
            preset,
            ..
        } = this;

        let Some(state) = state else {
            return Ok(Frame::new());
        };

        let context = state
            .canvas
            .get_context("2d")?
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()?;

        let canvas_size = *canvas_size;
        let edit_mode = *edit_mode;
        let preset = *preset;
        let graph: &WidgetGraph = graph;

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

            if matches!(edit_mode, EditMode::ToggleEdge(_)) {
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

        // Show whether dropping the edge here would add or remove it
        let would_remove = target.is_some_and(|target| graph.are_adjacent(from, target));
        let color = if would_remove {
            Color::RED
        } else {
            Color::BLUE
        };

        // Arrow head so that it is clear which way around the edge is about to go
        if preset.directed_edges() {
            canvas.arrow(start, end, HtmlCanvas::thin_line_weight(), color);
        } else {
            canvas.line(start, end, HtmlCanvas::thin_line_weight(), color);
        }
    }

    fn send_graph(this: &SharedState, context: &Context<GraphBackendMessage>) {
        context.send_message(&GraphBackendMessage::SetGraph {
            graph: this.graph.clone(),
        });
    }

    fn add_vertex_at(this: &mut SharedState, position: V2f, color: VertexColor) -> VertexIndex {
        this.graph.add_vertex(Vertex {
            position: clamp_to_canvas(position, this.canvas_size),
            color,
        })
    }

    fn move_dragged_vertex(this: &mut SharedState, frame: &Frame) -> Applied {
        let Some((vertex, drag)) = frame
            .vertices
            .dragged
            .filter(|_| this.edit_mode.moves_vertices())
        else {
            return Applied::none();
        };

        let new_position = clamp_to_canvas(drag.position(), this.canvas_size);
        let position: &mut V2f = this.graph.get_vertex_mut(vertex).get_inner_mut();
        let changed = *position != new_position;
        *position = new_position;

        Applied {
            changed,
            committed: drag.dropped,
        }
    }

    fn apply(this: &mut SharedState, frame: &Frame) -> Applied {
        let moved = GraphWidget::move_dragged_vertex(this, frame);

        if let Some(position) = frame.background.clicked
            && let Some(color) = this.edit_mode.new_vertex_color()
        {
            GraphWidget::add_vertex_at(this, position, color);
            return Applied {
                changed: true,
                committed: true,
            };
        }

        let applied = match this.edit_mode {
            EditMode::MoveVertex => Applied::none(),

            EditMode::AddColorVertex(new_color) => match frame.vertices.clicked {
                Some(vertex) => {
                    let color: &mut VertexColor = this.graph.get_vertex_mut(vertex).get_inner_mut();
                    let changed = *color != new_color;
                    *color = new_color;
                    Applied {
                        changed,
                        committed: changed,
                    }
                }
                None => Applied::none(),
            },

            EditMode::RemoveVertex => match frame.vertices.clicked {
                Some(vertex) => {
                    this.graph.remove_vertex(vertex);
                    Applied {
                        changed: true,
                        committed: true,
                    }
                }
                None => Applied::none(),
            },

            EditMode::ToggleEdge(edge_vertex) => GraphWidget::drop_edge(this, frame, edge_vertex),

            EditMode::GameMove(player) => match this.preset {
                GraphPreset::Snort => GraphWidget::snort_move(this, frame, player),
                GraphPreset::DigraphPlacement => {
                    GraphWidget::digraph_placement_move(this, frame, player)
                }
                // TODO: Col moves
                GraphPreset::Col => Applied::none(),
            },
        };

        Applied {
            changed: moved.changed || applied.changed,
            committed: moved.committed || applied.committed,
        }
    }

    /// Connect or disconnect two vertices. Games played on undirected graphs store an edge
    /// as a pair of opposite arcs, so for them the edge is dragged out in both directions
    /// at once
    fn connect(this: &mut SharedState, from: VertexIndex, to: VertexIndex, connect: bool) {
        this.graph.connect(from, to, connect);
        if !this.preset.directed_edges() {
            this.graph.connect(to, from, connect);
        }
    }

    fn drop_edge(
        this: &mut SharedState,
        frame: &Frame,
        edge_vertex: Option<VertexColor>,
    ) -> Applied {
        let Some((from, drag)) = frame.vertices.dragged.filter(|(_, drag)| drag.dropped) else {
            return Applied::none();
        };

        let connected = match frame.vertices.hovered {
            Some(target) if target == from => return Applied::none(),
            Some(target) => target,
            // Dropped on empty canvas, so the edge only lands somewhere if the dropdown
            // is set to leave a vertex behind
            None => {
                let Some(color) = edge_vertex else {
                    return Applied::none();
                };
                let target = GraphWidget::add_vertex_at(this, drag.cursor, color);
                GraphWidget::connect(this, from, target, true);
                return Applied {
                    changed: true,
                    committed: true,
                };
            }
        };

        let adjacent = this.graph.are_adjacent(from, connected);
        GraphWidget::connect(this, from, connected, !adjacent);
        Applied {
            changed: true,
            committed: true,
        }
    }

    /// Rearrange the whole graph with the chosen algorithm. Parameters that were never
    /// touched by hand are recomputed first, since the graph they were last filled in for
    /// is not the graph being laid out now
    fn apply_layout(this: &mut SharedState, inputs: &LayoutInputs) {
        let vertices = this.graph.size();
        let canvas_size = this.canvas_size;

        if !this.layout_customized {
            inputs.show_defaults(vertices, canvas_size);
        }

        match inputs.algorithm(this.preset) {
            LayoutAlgorithm::Circle => inputs.circle(canvas_size).layout(&mut this.graph),
            LayoutAlgorithm::SpringEmbedder => {
                inputs.spring(vertices, canvas_size).layout(&mut this.graph);
            }
        }

        // Parameters of one's own choosing are left to put vertices wherever they put
        // them, but a layout that diverged has to be caught: a position that is not a
        // number would not even survive being sent back to python
        for vertex in this.graph.vertex_indices() {
            let position: &mut V2f = this.graph.get_vertex_mut(vertex).get_inner_mut();
            if !position.x.is_finite() {
                position.x = canvas_size.x * 0.5;
            }
            if !position.y.is_finite() {
                position.y = canvas_size.y * 0.5;
            }
        }
    }

    /// Hand the turn over to the other player, unless the same player is set to keep moving
    fn pass_turn(this: &mut SharedState) {
        if !this.consecutive_moves
            && let Some(new_mode) = this.edit_mode.opposite_player()
        {
            this.edit_mode = new_mode;
        }
    }

    fn snort_move(this: &mut SharedState, frame: &Frame, player: Player) -> Applied {
        let Some(clicked) = frame.vertices.clicked else {
            return Applied::none();
        };

        let Ok(graph) = this.graph.try_map(|vertex| {
            snort::VertexColor::try_from(vertex.color).map(|color| SnortVertex {
                kind: snort::VertexKind::Single(color),
                position: vertex.position,
            })
        }) else {
            // Should be unreachable
            return Applied::none();
        };

        let position = Snort::new(UndirectedGraph::from_directed(&graph));
        let new_position = match player {
            Player::Left => {
                snort_move_in::<{ snort::VertexColor::TintLeft as u8 }>(&position, clicked)
            }
            Player::Right => {
                snort_move_in::<{ snort::VertexColor::TintRight as u8 }>(&position, clicked)
            }
        };

        let Some(new_position) = new_position else {
            return Applied::none();
        };

        this.graph = new_position.graph.as_directed().map(|vertex| Vertex {
            position: vertex.position,
            color: VertexColor::from(vertex.kind.color()),
        });
        GraphWidget::pass_turn(this);

        Applied {
            changed: true,
            committed: true,
        }
    }

    fn digraph_placement_move(this: &mut SharedState, frame: &Frame, player: Player) -> Applied {
        let Some(clicked) = frame.vertices.clicked else {
            return Applied::none();
        };

        let Ok(graph) = this.graph.try_map(|vertex| {
            digraph_placement::VertexColor::try_from(vertex.color).map(|color| {
                DigraphPlacementVertex {
                    color,
                    position: vertex.position,
                }
            })
        }) else {
            // Reachable only if the graph got a vertex of a color that the game has no
            // move for, e.g. by editing it as another game first
            return Applied::none();
        };

        let position = DigraphPlacement::new(graph);
        let new_position = match player {
            Player::Left => digraph_placement_move_in::<
                { digraph_placement::VertexColor::Left as u8 },
            >(&position, clicked),
            Player::Right => digraph_placement_move_in::<
                { digraph_placement::VertexColor::Right as u8 },
            >(&position, clicked),
        };

        let Some(new_position) = new_position else {
            return Applied::none();
        };

        this.graph = new_position.graph.map(|vertex| Vertex {
            position: vertex.position,
            color: VertexColor::from(vertex.color),
        });
        GraphWidget::pass_turn(this);

        Applied {
            changed: true,
            committed: true,
        }
    }

    fn update(
        this: &mut SharedState,
        context: &Context<GraphBackendMessage>,
    ) -> Result<(), JsValue> {
        let settling = matches!(this.interactions.pointer().button, Button::Released(_));

        let frame = GraphWidget::draw(this)?;
        let applied = GraphWidget::apply(this, &frame);
        if applied.committed {
            GraphWidget::send_graph(this, context);
        }
        if applied.changed || settling {
            GraphWidget::draw(this)?;
        }

        Ok(())
    }
}

#[derive(Clone, Copy)]
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

#[derive(Clone, Copy)]
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

#[derive(Clone, Copy)]
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
    step: &str,
) -> Result<(HtmlLabelElement, HtmlInputElement), JsValue> {
    // Wrapping the input in the label associates the two without needing a unique id
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
    if !step.is_empty() {
        input.set_step(step);
        input.set_min("0");
        input.style().set_property("width", "7em")?;
    }

    row.append_child(&text)?;
    row.append_child(&input)?;
    Ok((row, input))
}

/// The controls that pick a layout and run it, along with the collapsed panel holding the
/// parameters of both algorithms
#[derive(Clone)]
struct LayoutControls {
    /// Everything, ready to be put into the widget
    root: HtmlElement,
    inputs: LayoutInputs,
    apply: HtmlButtonElement,
    reset: HtmlButtonElement,
    /// Holds every parameter row, so that edits to any of them bubble up to one listener
    parameters: HtmlDivElement,
    /// The disclosure the parameters are collapsed into
    details: HtmlElement,
    circle_rows: Vec<HtmlLabelElement>,
    spring_rows: Vec<HtmlLabelElement>,
}

impl LayoutControls {
    fn create(document: &Document, preset: GraphPreset) -> Result<LayoutControls, JsValue> {
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
        let algorithm: HtmlSelectElement =
            SelectOption::create_element(document, "Layout", &preset, LAYOUT_OPTIONS)?;
        let apply = document
            .create_element("button")?
            .dyn_into::<HtmlButtonElement>()?;
        apply.set_type("button");
        apply.set_text_content(Some("Apply"));

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

        let (circle_radius_row, circle_radius) = parameter_row(document, "Radius", "number", "1")?;
        let (center_x_row, center_x) = parameter_row(document, "Center X", "number", "1")?;
        let (center_y_row, center_y) = parameter_row(document, "Center Y", "number", "1")?;

        let (iterations_row, iterations) = parameter_row(document, "Iterations", "number", "128")?;
        let (cooling_rate_row, cooling_rate) =
            parameter_row(document, "Cooling rate", "number", "0.0001")?;
        let (c_attractive_row, c_attractive) =
            parameter_row(document, "Attraction", "number", "0.1")?;
        let (c_repulsive_row, c_repulsive) = parameter_row(document, "Repulsion", "number", "10")?;
        let (ideal_spring_length_row, ideal_spring_length) =
            parameter_row(document, "Ideal edge length", "number", "1")?;
        let (c_middle_attractive_row, c_middle_attractive) =
            parameter_row(document, "Pull to center", "number", "0.001")?;
        let (keep_on_canvas_row, keep_on_canvas) =
            parameter_row(document, "Keep on canvas", "checkbox", "")?;

        let circle_rows = vec![circle_radius_row, center_x_row, center_y_row];
        let spring_rows = vec![
            iterations_row,
            cooling_rate_row,
            c_attractive_row,
            c_repulsive_row,
            ideal_spring_length_row,
            c_middle_attractive_row,
            keep_on_canvas_row,
        ];
        for parameter in circle_rows.iter().chain(spring_rows.iter()) {
            parameters.append_child(parameter)?;
        }

        let reset = document
            .create_element("button")?
            .dyn_into::<HtmlButtonElement>()?;
        reset.set_type("button");
        reset.set_text_content(Some("Reset to defaults"));
        reset.style().set_property("margin-top", "4px")?;
        parameters.append_child(&reset)?;

        let details = document
            .create_element("details")?
            .dyn_into::<HtmlElement>()?;
        details.style().set_property("margin-top", "4px")?;
        let summary = document.create_element("summary")?;
        summary.set_text_content(Some("Layout parameters"));
        details.append_child(&summary)?;
        details.append_child(&parameters)?;

        root.append_child(&row)?;
        root.append_child(&details)?;

        Ok(LayoutControls {
            root,
            inputs: LayoutInputs {
                algorithm,
                circle_radius,
                center_x,
                center_y,
                iterations,
                cooling_rate,
                c_attractive,
                c_repulsive,
                ideal_spring_length,
                c_middle_attractive,
                keep_on_canvas,
            },
            apply,
            reset,
            parameters,
            details,
            circle_rows,
            spring_rows,
        })
    }

    /// Only the parameters of the chosen algorithm are worth showing
    fn show_rows_of(&self, algorithm: LayoutAlgorithm) -> Result<(), JsValue> {
        let display = |shown| if shown { "flex" } else { "none" };
        for row in &self.circle_rows {
            row.style()
                .set_property("display", display(algorithm == LayoutAlgorithm::Circle))?;
        }
        for row in &self.spring_rows {
            row.style().set_property(
                "display",
                display(algorithm == LayoutAlgorithm::SpringEmbedder),
            )?;
        }
        Ok(())
    }
}

/// Number a parameter input holds, or `fallback` if it was left empty or unreadable
fn read_number(input: &HtmlInputElement, fallback: f32) -> f32 {
    let value = input.value_as_number();
    if value.is_finite() {
        value as f32
    } else {
        fallback
    }
}

/// The parameter fields of the layout panel. Every field of both algorithms is here except
/// the vertex radius and the bounding rectangle, which are what the canvas says they are
#[derive(Clone)]
struct LayoutInputs {
    algorithm: HtmlSelectElement,

    circle_radius: HtmlInputElement,
    center_x: HtmlInputElement,
    center_y: HtmlInputElement,

    iterations: HtmlInputElement,
    cooling_rate: HtmlInputElement,
    c_attractive: HtmlInputElement,
    c_repulsive: HtmlInputElement,
    ideal_spring_length: HtmlInputElement,
    c_middle_attractive: HtmlInputElement,
    keep_on_canvas: HtmlInputElement,
}

impl LayoutInputs {
    fn algorithm(&self, preset: GraphPreset) -> LayoutAlgorithm {
        SelectOption::selected_value(&self.algorithm.value(), &preset, LAYOUT_OPTIONS)
            .map_or(LayoutAlgorithm::Circle, |option| option.algorithm)
    }

    /// Fill every field with the parameter that suits the graph as it is right now
    fn show_defaults(&self, vertices: usize, canvas_size: V2f) {
        let circle = default_circle(canvas_size);
        self.circle_radius
            .set_value(&format!("{:.1}", circle.circle_radius));
        self.center_x.set_value(&format!("{:.1}", circle.center.x));
        self.center_y.set_value(&format!("{:.1}", circle.center.y));

        let spring = default_spring(vertices, canvas_size);
        self.iterations.set_value(&spring.iterations.to_string());
        self.cooling_rate
            .set_value(&format!("{:.6}", spring.cooling_rate));
        self.c_attractive
            .set_value(&format!("{:.2}", spring.c_attractive));
        self.c_repulsive
            .set_value(&format!("{:.1}", spring.c_repulsive));
        self.ideal_spring_length
            .set_value(&format!("{:.1}", spring.ideal_spring_length));
        self.c_middle_attractive.set_value(&format!(
            "{:.4}",
            spring
                .bounds
                .and_then(|bounds| bounds.c_middle_attractive)
                .unwrap_or(0.0)
        ));
        self.keep_on_canvas.set_checked(spring.bounds.is_some());
    }

    fn circle(&self, canvas_size: V2f) -> CircleEdge {
        let default = default_circle(canvas_size);
        CircleEdge {
            circle_radius: read_number(&self.circle_radius, default.circle_radius),
            vertex_radius: HtmlCanvas::vertex_radius(),
            center: V2f {
                x: read_number(&self.center_x, default.center.x),
                y: read_number(&self.center_y, default.center.y),
            },
        }
    }

    fn spring(&self, vertices: usize, canvas_size: V2f) -> SpringEmbedder {
        let default = default_spring(vertices, canvas_size);

        // A hand typed iteration count still has to leave the page responsive
        let iterations = read_number(&self.iterations, default.iterations as f32);
        let iterations = usize::min(
            iterations.max(0.0) as usize,
            MAX_LAYOUT_WORK / usize::max(vertices * vertices, 1),
        );

        SpringEmbedder {
            cooling_rate: read_number(&self.cooling_rate, default.cooling_rate),
            c_attractive: read_number(&self.c_attractive, default.c_attractive),
            c_repulsive: read_number(&self.c_repulsive, default.c_repulsive),
            ideal_spring_length: read_number(
                &self.ideal_spring_length,
                default.ideal_spring_length,
            ),
            iterations,
            bounds: self
                .keep_on_canvas
                .checked()
                .then(|| canvas_bounds(canvas_size, read_number(&self.c_middle_attractive, 0.0))),
        }
    }
}

impl WasmWidget for GraphWidget {
    type BackendMessage = GraphBackendMessage;
    type FrontendMessage = GraphFrontendMessage;

    fn handle_message(&mut self, message: Self::FrontendMessage) -> Result<(), JsValue> {
        match message {
            GraphFrontendMessage::SetGraph(new_graph) => {
                // Do not send that graph back to python
                let mut this = self.shared.lock().unwrap();
                this.graph = new_graph;
                GraphWidget::draw(&mut this)?;
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

        let mode_select: HtmlSelectElement =
            SelectOption::create_element(&document, "Edit mode", &self.preset, EDIT_OPTIONS)?;

        // Wrapping the dropdown in the label associates the two without needing a unique id
        let edge_options = document
            .create_element("label")?
            .dyn_into::<HtmlLabelElement>()?;
        edge_options.style().set_property("display", "none")?;
        edge_options.style().set_property("align-items", "center")?;
        edge_options.style().set_property("gap", "4px")?;

        let edge_vertex: HtmlSelectElement = SelectOption::create_element(
            &document,
            "Edge vertex",
            &self.preset,
            EDGE_VERTEX_OPTIONS,
        )?;
        let edge_options_text = document.create_element("span")?;
        edge_options_text.set_text_content(Some("New vertex"));
        edge_options.append_child(&edge_options_text)?;
        edge_options.append_child(&edge_vertex)?;

        let move_options = document
            .create_element("label")?
            .dyn_into::<HtmlLabelElement>()?;
        move_options.style().set_property("display", "none")?;
        move_options.style().set_property("align-items", "center")?;
        move_options.style().set_property("gap", "4px")?;

        let consecutive_moves = document
            .create_element("input")?
            .dyn_into::<HtmlInputElement>()?;
        consecutive_moves.set_type("checkbox");
        let move_options_text = document.create_element("span")?;
        move_options_text.set_text_content(Some("Consecutive Moves"));
        move_options.append_child(&consecutive_moves)?;
        move_options.append_child(&move_options_text)?;

        let mode_handler = ScopedClosure::<dyn FnMut() -> Result<(), JsValue>>::new({
            let preset = self.preset;
            let this = Arc::clone(&self.shared);
            let mode_select = mode_select.clone();
            let edge_options = edge_options.clone();
            let edge_vertex_select = edge_vertex.clone();
            let move_options = move_options.clone();
            let consecutive_moves = consecutive_moves.clone();
            move || {
                if let Some(mode) =
                    SelectOption::selected_value(&mode_select.value(), &preset, EDIT_OPTIONS)
                {
                    let display = |shown| if shown { "flex" } else { "none" };
                    edge_options.style().set_property(
                        "display",
                        display(matches!(mode.mode, EditMode::ToggleEdge(_))),
                    )?;
                    move_options
                        .style()
                        .set_property("display", display(mode.mode.opposite_player().is_some()))?;

                    let mut this = this.lock().unwrap();
                    this.edit_mode = mode.mode;

                    if let EditMode::ToggleEdge(edge_vertex) = &mut this.edit_mode {
                        *edge_vertex = SelectOption::selected_value(
                            &edge_vertex_select.value(),
                            &preset,
                            EDGE_VERTEX_OPTIONS,
                        )
                        .and_then(|option| option.color);
                    }

                    this.consecutive_moves = consecutive_moves.checked();
                    GraphWidget::draw(&mut this)?;
                }

                Ok(())
            }
        });
        mode_select
            .add_event_listener_with_callback("change", mode_handler.as_ref().unchecked_ref())?;
        edge_options
            .add_event_listener_with_callback("change", mode_handler.as_ref().unchecked_ref())?;
        move_options
            .add_event_listener_with_callback("change", mode_handler.as_ref().unchecked_ref())?;
        mode_handler.forget();

        controls.append_child(&mode_select)?;
        controls.append_child(&edge_options)?;
        controls.append_child(&move_options)?;
        element.append_child(&controls)?;

        let layout = LayoutControls::create(&document, self.preset)?;
        layout.inputs.show_defaults(0, DEFAULT_CANVAS_SIZE);
        layout.show_rows_of(layout.inputs.algorithm(self.preset))?;

        let layout_handler = ScopedClosure::<dyn FnMut() -> Result<(), JsValue>>::new({
            let preset = self.preset;
            let layout = layout.clone();
            move || layout.show_rows_of(layout.inputs.algorithm(preset))
        });
        layout
            .inputs
            .algorithm
            .add_event_listener_with_callback("change", layout_handler.as_ref().unchecked_ref())?;
        layout_handler.forget();

        // Typing into any parameter hands the whole panel over to the user, so that the
        // defaults stop being recomputed underneath what they entered
        let customize_handler = ScopedClosure::<dyn FnMut() -> Result<(), JsValue>>::new({
            let this = Arc::clone(&self.shared);
            move || {
                this.lock().unwrap().layout_customized = true;
                Ok(())
            }
        });
        layout.parameters.add_event_listener_with_callback(
            "input",
            customize_handler.as_ref().unchecked_ref(),
        )?;
        customize_handler.forget();

        let reset_handler = ScopedClosure::<dyn FnMut() -> Result<(), JsValue>>::new({
            let this = Arc::clone(&self.shared);
            let inputs = layout.inputs.clone();
            move || {
                let mut this = this.lock().unwrap();
                this.layout_customized = false;
                inputs.show_defaults(this.graph.size(), this.canvas_size);
                Ok(())
            }
        });
        layout
            .reset
            .add_event_listener_with_callback("click", reset_handler.as_ref().unchecked_ref())?;
        reset_handler.forget();

        let open_handler = ScopedClosure::<dyn FnMut() -> Result<(), JsValue>>::new({
            let this = Arc::clone(&self.shared);
            let inputs = layout.inputs.clone();
            move || {
                let this = this.lock().unwrap();
                if !this.layout_customized {
                    inputs.show_defaults(this.graph.size(), this.canvas_size);
                }
                Ok(())
            }
        });
        layout
            .details
            .add_event_listener_with_callback("toggle", open_handler.as_ref().unchecked_ref())?;
        open_handler.forget();

        let apply_handler = ScopedClosure::<dyn FnMut() -> Result<(), JsValue>>::new({
            let this = Arc::clone(&self.shared);
            let context = context.clone();
            let inputs = layout.inputs.clone();
            move || {
                let mut this = this.lock().unwrap();
                GraphWidget::apply_layout(&mut this, &inputs);
                GraphWidget::send_graph(&this, &context);
                GraphWidget::draw(&mut this)?;
                Ok(())
            }
        });
        layout
            .apply
            .add_event_listener_with_callback("click", apply_handler.as_ref().unchecked_ref())?;
        apply_handler.forget();

        element.append_child(&layout.root)?;

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
        canvas.set_width(DEFAULT_CANVAS_SIZE.x as u32);
        canvas.set_height(DEFAULT_CANVAS_SIZE.y as u32);
        canvas.style().set_property("display", "block")?;
        canvas.style().set_property("width", "100%")?;
        canvas.style().set_property("height", "100%")?;
        canvas.style().set_property("user-select", "none")?;

        let down_handler = ScopedClosure::<dyn FnMut(MouseEvent) -> Result<(), JsValue>>::new({
            let this = Arc::clone(&self.shared);
            let context = context.clone();
            move |event: MouseEvent| {
                if event.button() != 0 {
                    return Ok(());
                }

                let mut this = this.lock().unwrap();
                this.interactions
                    .pointer_pressed(mouse_event_to_canvas(&event));
                GraphWidget::update(&mut this, &context)
            }
        });
        canvas
            .add_event_listener_with_callback("mousedown", down_handler.as_ref().unchecked_ref())?;
        down_handler.forget();

        let move_handler = ScopedClosure::<dyn FnMut(MouseEvent) -> Result<(), JsValue>>::new({
            let this = Arc::clone(&self.shared);
            let context = context.clone();
            move |event| {
                let mut this = this.lock().unwrap();
                this.interactions
                    .pointer_moved(mouse_event_to_canvas(&event));
                GraphWidget::update(&mut this, &context)
            }
        });
        canvas
            .add_event_listener_with_callback("mousemove", move_handler.as_ref().unchecked_ref())?;
        move_handler.forget();

        let up_handler = ScopedClosure::<dyn FnMut(MouseEvent) -> Result<(), JsValue>>::new({
            let this = Arc::clone(&self.shared);
            let context = context.clone();
            move |event| {
                let mut this = this.lock().unwrap();
                this.interactions
                    .pointer_released(mouse_event_to_canvas(&event));
                GraphWidget::update(&mut this, &context)
            }
        });
        canvas.add_event_listener_with_callback("mouseup", up_handler.as_ref().unchecked_ref())?;
        up_handler.forget();

        let leave_handler = ScopedClosure::<dyn FnMut() -> Result<(), JsValue>>::new({
            let this = Arc::clone(&self.shared);
            let context = context.clone();
            move || {
                let mut this = this.lock().unwrap();
                this.interactions.pointer_left();
                GraphWidget::update(&mut this, &context)
            }
        });
        canvas.add_event_listener_with_callback(
            "mouseleave",
            leave_handler.as_ref().unchecked_ref(),
        )?;
        leave_handler.forget();

        canvas_container.append_child(&canvas)?;
        element.append_child(&canvas_container)?;

        let resize_handler = ScopedClosure::<dyn FnMut() -> Result<(), JsValue>>::new({
            let this = Arc::clone(&self.shared);
            let canvas_container = canvas_container.clone();
            move || {
                let canvas_size = V2f {
                    x: canvas_container.client_width() as f32,
                    y: canvas_container.client_height() as f32,
                };

                let mut this = this.lock().unwrap();
                if this.canvas_size == canvas_size {
                    return Ok(());
                }
                this.canvas_size = canvas_size;

                if let Some(state) = &this.state {
                    state.canvas.set_width(canvas_size.x as u32);
                    state.canvas.set_height(canvas_size.y as u32);
                }
                GraphWidget::draw(&mut this)?;

                Ok(())
            }
        });
        let resize_observer = ResizeObserver::new(resize_handler.as_ref().unchecked_ref())?;
        resize_observer.observe(&canvas_container);
        resize_handler.forget();

        let mut this = self.shared.lock().unwrap();
        this.state = Some(HtmlState {
            canvas,
            edit_mode: mode_select,
            _resize_observer: resize_observer,
        });

        GraphWidget::draw(&mut this)?;
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
