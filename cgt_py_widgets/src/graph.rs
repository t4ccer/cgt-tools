use cgt::{
    drawing::{Area, Button, Canvas, Color, Hits, Interaction, Interactions},
    graph::{Graph, VertexIndex},
    has::Has,
    numeric::v2f::V2f,
};
use cgt_py_messages::{
    GraphBackendMessage, GraphFrontendMessage, Vertex, VertexColor, WidgetGraph,
};
use jupyter_rust_widget_frontend::{AnyWidgetModel, Context, WasmWidget};
use std::sync::{Arc, Mutex};
use wasm_bindgen::{
    JsCast, JsValue,
    prelude::{ScopedClosure, wasm_bindgen},
};
use web_sys::{
    CanvasRenderingContext2d, Element, HtmlCanvasElement, HtmlDivElement, HtmlInputElement,
    HtmlLabelElement, HtmlSelectElement, MouseEvent, ResizeObserver,
};

use crate::{SelectOption, SelectOptionElement, canvas::HtmlCanvas};

struct HtmlState {
    canvas: HtmlCanvasElement,

    /// Kept around only so that it keeps observing the canvas container
    _resize_observer: ResizeObserver,
}

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
}

impl EditMode {
    /// Color of the vertex that clicking on empty canvas adds, if the mode adds one at all
    const fn new_vertex_color(self) -> Option<VertexColor> {
        match self {
            EditMode::ToggleEdge => Some(VertexColor::White),
            EditMode::AddColorVertex(color) => Some(color),
            EditMode::MoveVertex | EditMode::RemoveVertex => None,
        }
    }

    /// Whether vertices follow the pointer that drags them
    const fn moves_vertices(self) -> bool {
        match self {
            EditMode::MoveVertex | EditMode::AddColorVertex(_) => true,
            EditMode::ToggleEdge | EditMode::RemoveVertex => false,
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
    edit_mode: EditMode,
    // TODO: Move into EditMode
    // TODO: This also needs to track what vertex we want to add, either from dropdown
    // or forced for e.g. BipartiteSnort
    edge_creates_vertex: bool,
    state: Option<HtmlState>,
    canvas_size: V2f,
    graph: WidgetGraph,
    interactions: Interactions,
}

const EDIT_OPTIONS: &[EditOption] = &[
    EditOption {
        text: "Add White Vertex",
        mode: EditMode::AddColorVertex(VertexColor::White),
    },
    EditOption {
        text: "Add Blue Vertex",
        mode: EditMode::AddColorVertex(VertexColor::Blue),
    },
    EditOption {
        text: "Add Red Vertex",
        mode: EditMode::AddColorVertex(VertexColor::Red),
    },
    EditOption {
        text: "Add Green Vertex",
        mode: EditMode::AddColorVertex(VertexColor::Green),
    },
    EditOption {
        text: "Move Vertex",
        mode: EditMode::MoveVertex,
    },
    EditOption {
        text: "Add/Remove Edge",
        mode: EditMode::ToggleEdge,
    },
    EditOption {
        text: "Remove Vertex",
        mode: EditMode::RemoveVertex,
    },
];

const DEFAULT_CANVAS_SIZE: V2f = V2f { x: 640.0, y: 400.0 };
const MIN_CANVAS_SIZE: V2f = V2f { x: 240.0, y: 160.0 };

struct GraphWidget {
    shared: Arc<Mutex<SharedState>>,
}

impl GraphWidget {
    fn new() -> GraphWidget {
        GraphWidget {
            shared: Arc::new(Mutex::new(SharedState {
                edit_mode: EDIT_OPTIONS[0].mode,
                edge_creates_vertex: true,
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

impl GraphWidget {
    /// Paint the graph and report what the mouse did to it
    fn draw(this: &mut SharedState) -> Result<Frame, JsValue> {
        let SharedState {
            state,
            canvas_size,
            graph,
            interactions,
            edit_mode,
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

            if edit_mode == EditMode::ToggleEdge {
                GraphWidget::draw_new_edge(canvas, graph, &vertices);
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
    fn draw_new_edge(canvas: &mut HtmlCanvas<'_>, graph: &WidgetGraph, hits: &Hits<VertexIndex>) {
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
        canvas.line(start, end, HtmlCanvas::thin_line_weight(), color);
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

            EditMode::ToggleEdge => GraphWidget::drop_edge(this, frame),
        };

        Applied {
            changed: moved.changed || applied.changed,
            committed: moved.committed || applied.committed,
        }
    }

    fn drop_edge(this: &mut SharedState, frame: &Frame) -> Applied {
        let Some((from, drag)) = frame.vertices.dragged.filter(|(_, drag)| drag.dropped) else {
            return Applied::none();
        };

        let connected = match frame.vertices.hovered {
            Some(target) if target == from => return Applied::none(),
            Some(target) => target,
            None if this.edge_creates_vertex => {
                let color = this
                    .edit_mode
                    .new_vertex_color()
                    .unwrap_or(VertexColor::White);
                let target = GraphWidget::add_vertex_at(this, drag.cursor, color);
                this.graph.connect(from, target, true);
                return Applied {
                    changed: true,
                    committed: true,
                };
            }
            None => return Applied::none(),
        };

        let adjacent = this.graph.are_adjacent(from, connected);
        this.graph.connect(from, connected, !adjacent);
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
}

impl SelectOptionElement for EditOption {
    // TODO: Game presets
    type Preset = ();

    fn text(&self) -> &str {
        self.text
    }

    fn is_visible(&self, (): &Self::Preset) -> bool {
        true
    }
}

impl WasmWidget for GraphWidget {
    type BackendMessage = GraphBackendMessage;
    type FrontendMessage = GraphFrontendMessage;

    fn handle_message(&mut self, message: Self::FrontendMessage) -> Result<(), JsValue> {
        match message {
            GraphFrontendMessage::SetGraph(new_graph) => {
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
            SelectOption::create_element(&document, "Edit mode", &(), EDIT_OPTIONS)?;

        // Wrapping the checkbox in the label associates the two without needing a unique id
        let edge_options = document
            .create_element("label")?
            .dyn_into::<HtmlLabelElement>()?;
        edge_options.style().set_property("display", "none")?;
        edge_options.style().set_property("align-items", "center")?;
        edge_options.style().set_property("gap", "4px")?;

        let edge_creates_vertex = document
            .create_element("input")?
            .dyn_into::<HtmlInputElement>()?;
        edge_creates_vertex.set_type("checkbox");
        edge_creates_vertex.set_checked(true);
        let edge_options_text = document.create_element("span")?;
        edge_options_text.set_text_content(Some("Add vertex"));
        edge_options.append_child(&edge_creates_vertex)?;
        edge_options.append_child(&edge_options_text)?;

        let mode_handler = ScopedClosure::<dyn FnMut() -> Result<(), JsValue>>::new({
            let this = Arc::clone(&self.shared);
            let mode_select = mode_select.clone();
            let edge_options = edge_options.clone();
            let edge_creates_vertex = edge_creates_vertex.clone();
            move || {
                if let Some(mode) =
                    SelectOption::selected_value(&mode_select.value(), &(), EDIT_OPTIONS)
                {
                    let display = if mode.mode == EditMode::ToggleEdge {
                        "flex"
                    } else {
                        "none"
                    };
                    edge_options.style().set_property("display", display)?;

                    let mut this = this.lock().unwrap();
                    this.edit_mode = mode.mode;
                    this.edge_creates_vertex = edge_creates_vertex.checked();
                    GraphWidget::draw(&mut this)?;
                }

                Ok(())
            }
        });
        mode_select
            .add_event_listener_with_callback("change", mode_handler.as_ref().unchecked_ref())?;
        edge_options
            .add_event_listener_with_callback("change", mode_handler.as_ref().unchecked_ref())?;
        mode_handler.forget();

        controls.append_child(&mode_select)?;
        controls.append_child(&edge_options)?;
        element.append_child(&controls)?;

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
            _resize_observer: resize_observer,
        });

        GraphWidget::draw(&mut this)?;
        context.send_message(&GraphBackendMessage::Initialized);

        Ok(())
    }
}

#[wasm_bindgen]
pub fn render_graph_widget_impl(model: AnyWidgetModel, el: Element) -> Result<(), JsValue> {
    GraphWidget::new().render(model, el)
}
