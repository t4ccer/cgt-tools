use cgt::{
    graph::{Graph, VertexIndex},
    has::Has,
    numeric::v2f::V2f,
};
use cgt_py_messages::{
    GraphBackendMessage, GraphFrontendMessage, Vertex, VertexColor, WidgetGraph,
};
use core::f64;
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

use crate::{ActiveElement, SelectOption, SelectOptionElement};

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
    const fn new_vertex_color(self) -> Option<VertexColor> {
        match self {
            EditMode::ToggleEdge => Some(VertexColor::White),
            EditMode::AddColorVertex(color) => Some(color),
            EditMode::MoveVertex | EditMode::RemoveVertex => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Target {
    Canvas,
    Vertex(VertexIndex),
}

#[derive(Clone, Copy, PartialEq)]
enum Drag {
    NewVertex {
        pressed_at: V2f,
    },
    MoveVertex {
        grab_offset: V2f,
        pressed_at: V2f,
        moved: bool,
    },
    NewEdge {
        cursor: V2f,
    },
}

struct SharedState {
    edit_mode: EditMode,
    // TODO: Move into EditMode
    edge_creates_vertex: bool,
    state: Option<HtmlState>,
    canvas_size: V2f,
    graph: WidgetGraph,
    active: ActiveElement<Target, Drag>,
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
const VERTEX_RADIUS: f32 = 16.0;

/// How far the cursor may travel between press and release and still count as a click
const CLICK_SLOP: f32 = 4.0;

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
                active: ActiveElement::None,
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
    V2f {
        x: f32::clamp(
            position.x,
            VERTEX_RADIUS,
            f32::max(VERTEX_RADIUS, canvas_size.x - VERTEX_RADIUS),
        ),
        y: f32::clamp(
            position.y,
            VERTEX_RADIUS,
            f32::max(VERTEX_RADIUS, canvas_size.y - VERTEX_RADIUS),
        ),
    }
}

fn shaded(color: VertexColor, factor: f32) -> String {
    let rgb = color.rgb();
    let scale = |shift: u32| ((((rgb >> shift) & 0xff) as f32) * factor).round() as u32;
    format!("#{:02x}{:02x}{:02x}", scale(16), scale(8), scale(0))
}

fn vertex_position(graph: &WidgetGraph, vertex: VertexIndex) -> V2f {
    *graph.get_vertex(vertex).get_inner()
}

fn vertex_at(graph: &WidgetGraph, position: V2f) -> Option<VertexIndex> {
    graph
        .vertex_indices()
        .rev()
        .find(|&vertex| position.inside_circle(vertex_position(graph, vertex), VERTEX_RADIUS))
}

impl GraphWidget {
    fn draw_graph(this: &SharedState) -> Result<(), JsValue> {
        let Some(state) = &this.state else {
            return Ok(());
        };

        let context = state
            .canvas
            .get_context("2d")?
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()?;

        context.set_fill_style_str("#cccccc");
        context.fill_rect(
            0.0,
            0.0,
            this.canvas_size.x as f64,
            this.canvas_size.y as f64,
        );

        context.set_line_width(2.0);
        context.set_stroke_style_str("#000000");
        for (u, v) in this.graph.edges() {
            let u_position = vertex_position(&this.graph, u);
            let v_position = vertex_position(&this.graph, v);
            context.begin_path();
            context.move_to(u_position.x as f64, u_position.y as f64);
            context.line_to(v_position.x as f64, v_position.y as f64);
            context.stroke();
        }

        if let ActiveElement::Dragging(Target::Vertex(from), Drag::NewEdge { cursor }) = this.active
        {
            let from_position = vertex_position(&this.graph, from);
            let hovered = vertex_at(&this.graph, cursor).filter(|&hovered| hovered != from);
            let target = hovered.map_or(cursor, |hovered| vertex_position(&this.graph, hovered));

            let would_remove =
                hovered.is_some_and(|hovered| this.graph.are_adjacent(from, hovered));
            context.set_stroke_style_str(if would_remove { "#f92672" } else { "#4e4afb" });
            context.begin_path();
            context.move_to(from_position.x as f64, from_position.y as f64);
            context.line_to(target.x as f64, target.y as f64);
            context.stroke();
        }

        for vertex in this.graph.vertex_indices() {
            #[derive(Clone, Copy)]
            enum VertexState {
                None,
                Hover,
                Pressed,
            }

            let vertex_state = match this.active {
                ActiveElement::Pressed(Target::Vertex(active))
                | ActiveElement::Dragging(Target::Vertex(active), _)
                    if active == vertex =>
                {
                    VertexState::Pressed
                }
                ActiveElement::Hover(Target::Vertex(active)) if active == vertex => {
                    VertexState::Hover
                }
                _ => VertexState::None,
            };

            let position = vertex_position(&this.graph, vertex);
            context.begin_path();
            context.arc(
                position.x as f64,
                position.y as f64,
                VERTEX_RADIUS as f64,
                0.0,
                2.0 * f64::consts::PI,
            )?;
            let color: VertexColor = *this.graph.get_vertex(vertex).get_inner();
            context.set_fill_style_str(&shaded(
                color,
                match vertex_state {
                    VertexState::None => 1.0,
                    VertexState::Hover => 0.9,
                    VertexState::Pressed => 0.7,
                },
            ));
            context.fill();

            context.set_line_width(2.0);
            context.set_stroke_style_str("#000000");
            context.stroke();
        }

        Ok(())
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

    fn target_at(this: &SharedState, position: V2f) -> Target {
        vertex_at(&this.graph, position).map_or(Target::Canvas, Target::Vertex)
    }

    fn handle_drag(this: &mut SharedState, cursor: V2f) -> bool {
        match this.active {
            ActiveElement::None | ActiveElement::Hover(_) => {
                let hover = match GraphWidget::target_at(this, cursor) {
                    Target::Canvas => ActiveElement::None,
                    vertex => ActiveElement::Hover(vertex),
                };
                let changed = hover != this.active;
                this.active = hover;
                changed
            }
            ActiveElement::Pressed(_) => false,
            ActiveElement::Dragging(target, drag) => match drag {
                Drag::NewVertex { .. } => false,
                Drag::MoveVertex {
                    grab_offset,
                    pressed_at,
                    moved,
                } => {
                    let Target::Vertex(vertex) = target else {
                        return false;
                    };
                    if !moved && V2f::distance(pressed_at, cursor) <= CLICK_SLOP {
                        return false;
                    }

                    let new_position = clamp_to_canvas(cursor + grab_offset, this.canvas_size);
                    let position: &mut V2f = this.graph.get_vertex_mut(vertex).get_inner_mut();
                    *position = new_position;
                    this.active = ActiveElement::Dragging(
                        target,
                        Drag::MoveVertex {
                            grab_offset,
                            pressed_at,
                            moved: true,
                        },
                    );
                    true
                }
                Drag::NewEdge { .. } => {
                    this.active = ActiveElement::Dragging(target, Drag::NewEdge { cursor });
                    true
                }
            },
        }
    }

    /// Apply whatever the mouse was doing when it was released at `cursor`.
    /// Returns whether the graph changed
    fn handle_release(this: &mut SharedState, cursor: V2f) -> bool {
        match this.active {
            ActiveElement::None | ActiveElement::Hover(_) => false,

            // Only the removing mode presses a vertex without dragging it
            ActiveElement::Pressed(Target::Vertex(vertex)) => {
                if this.edit_mode == EditMode::RemoveVertex
                    && vertex_at(&this.graph, cursor) == Some(vertex)
                {
                    this.graph.remove_vertex(vertex);
                    true
                } else {
                    false
                }
            }
            ActiveElement::Pressed(Target::Canvas) => false,

            ActiveElement::Dragging(Target::Canvas, Drag::NewVertex { pressed_at }) => {
                let is_click = V2f::distance(pressed_at, cursor) <= CLICK_SLOP;
                match this.edit_mode.new_vertex_color() {
                    Some(color) if is_click && vertex_at(&this.graph, cursor).is_none() => {
                        GraphWidget::add_vertex_at(this, cursor, color);
                        true
                    }
                    _ => false,
                }
            }
            ActiveElement::Dragging(Target::Vertex(vertex), Drag::MoveVertex { moved, .. }) => {
                if moved {
                    return true;
                }

                // The vertex was clicked rather than dragged
                match this.edit_mode {
                    EditMode::AddColorVertex(new_color) => {
                        let color: &mut VertexColor =
                            this.graph.get_vertex_mut(vertex).get_inner_mut();
                        let changed = *color != new_color;
                        *color = new_color;
                        changed
                    }
                    EditMode::MoveVertex | EditMode::ToggleEdge | EditMode::RemoveVertex => false,
                }
            }
            ActiveElement::Dragging(Target::Vertex(from), Drag::NewEdge { .. }) => {
                match vertex_at(&this.graph, cursor) {
                    // Dropped back on the vertex the edge came from
                    Some(target) if target == from => false,
                    Some(target) => {
                        let connected = this.graph.are_adjacent(from, target);
                        this.graph.connect(from, target, !connected);
                        true
                    }
                    None if this.edge_creates_vertex => {
                        let color = this
                            .edit_mode
                            .new_vertex_color()
                            .unwrap_or(VertexColor::White);
                        let target = GraphWidget::add_vertex_at(this, cursor, color);
                        this.graph.connect(from, target, true);
                        true
                    }
                    None => false,
                }
            }

            // Drags that do not go with what they pressed, no press creates them
            ActiveElement::Dragging(..) => false,
        }
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
                this.active = ActiveElement::None;
                GraphWidget::draw_graph(&this)
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
                    this.active = ActiveElement::None;
                    GraphWidget::draw_graph(&this)?;
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
            move |event: MouseEvent| {
                if event.button() != 0 {
                    return Ok(());
                }

                let cursor = mouse_event_to_canvas(&event);
                let mut this = this.lock().unwrap();

                let active = match GraphWidget::target_at(&this, cursor) {
                    Target::Canvas => ActiveElement::Dragging(
                        Target::Canvas,
                        Drag::NewVertex { pressed_at: cursor },
                    ),
                    target @ Target::Vertex(vertex) => match this.edit_mode {
                        EditMode::MoveVertex | EditMode::AddColorVertex(_) => {
                            ActiveElement::Dragging(
                                target,
                                Drag::MoveVertex {
                                    grab_offset: vertex_position(&this.graph, vertex) - cursor,
                                    pressed_at: cursor,
                                    moved: false,
                                },
                            )
                        }
                        EditMode::ToggleEdge => {
                            ActiveElement::Dragging(target, Drag::NewEdge { cursor })
                        }
                        EditMode::RemoveVertex => ActiveElement::Pressed(target),
                    },
                };
                this.active = active;
                GraphWidget::draw_graph(&this)
            }
        });
        canvas
            .add_event_listener_with_callback("mousedown", down_handler.as_ref().unchecked_ref())?;
        down_handler.forget();

        let move_handler = ScopedClosure::<dyn FnMut(MouseEvent) -> Result<(), JsValue>>::new({
            let this = Arc::clone(&self.shared);
            move |event| {
                let cursor = mouse_event_to_canvas(&event);
                let mut this = this.lock().unwrap();

                if GraphWidget::handle_drag(&mut this, cursor) {
                    GraphWidget::draw_graph(&this)?;
                }

                Ok(())
            }
        });
        canvas
            .add_event_listener_with_callback("mousemove", move_handler.as_ref().unchecked_ref())?;
        move_handler.forget();

        let up_handler = ScopedClosure::<dyn FnMut(MouseEvent) -> Result<(), JsValue>>::new({
            let this = Arc::clone(&self.shared);
            let context = context.clone();
            move |event| {
                let cursor = mouse_event_to_canvas(&event);
                let mut this = this.lock().unwrap();

                if GraphWidget::handle_release(&mut this, cursor) {
                    GraphWidget::send_graph(&this, &context);
                }

                this.active = match GraphWidget::target_at(&this, cursor) {
                    Target::Canvas => ActiveElement::None,
                    vertex => ActiveElement::Hover(vertex),
                };
                GraphWidget::draw_graph(&this)
            }
        });
        canvas.add_event_listener_with_callback("mouseup", up_handler.as_ref().unchecked_ref())?;
        up_handler.forget();

        let leave_handler = ScopedClosure::<dyn FnMut() -> Result<(), JsValue>>::new({
            let this = Arc::clone(&self.shared);
            let context = context.clone();
            move || {
                let mut this = this.lock().unwrap();

                if matches!(
                    this.active,
                    ActiveElement::Dragging(_, Drag::MoveVertex { moved: true, .. })
                ) {
                    GraphWidget::send_graph(&this, &context);
                }

                this.active = ActiveElement::None;
                GraphWidget::draw_graph(&this)
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
                GraphWidget::draw_graph(&this)
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

        GraphWidget::draw_graph(&this)?;
        context.send_message(&GraphBackendMessage::Initialized);

        Ok(())
    }
}

#[wasm_bindgen]
pub fn render_graph_widget_impl(model: AnyWidgetModel, el: Element) -> Result<(), JsValue> {
    GraphWidget::new().render(model, el)
}

// TODO: Generalize once we migrate to Canvas interface
#[cfg(test)]
mod tests {
    use super::*;

    fn state_with_vertices(positions: &[V2f]) -> SharedState {
        SharedState {
            edit_mode: EditMode::MoveVertex,
            edge_creates_vertex: true,
            state: None,
            canvas_size: DEFAULT_CANVAS_SIZE,
            graph: Graph::from_edges(
                &[],
                &positions
                    .iter()
                    .map(|&position| Vertex {
                        position,
                        color: VertexColor::White,
                    })
                    .collect::<Vec<_>>(),
            ),
            active: ActiveElement::None,
        }
    }

    /// Press on empty canvas at `at`
    fn press_canvas(this: &mut SharedState, at: V2f) {
        this.active = ActiveElement::Dragging(Target::Canvas, Drag::NewVertex { pressed_at: at });
    }

    /// Press on `from` to start dragging an edge out of it
    fn press_edge(this: &mut SharedState, from: VertexIndex, cursor: V2f) {
        this.active = ActiveElement::Dragging(Target::Vertex(from), Drag::NewEdge { cursor });
    }

    /// Press on a vertex without dragging it, as the removing mode does
    fn press_vertex(this: &mut SharedState, vertex: VertexIndex) {
        this.active = ActiveElement::Pressed(Target::Vertex(vertex));
    }

    fn vertex_color(this: &SharedState, vertex: VertexIndex) -> VertexColor {
        *this.graph.get_vertex(vertex).get_inner()
    }

    #[test]
    fn click_on_empty_canvas_adds_vertex() {
        let mut this = state_with_vertices(&[]);
        this.edit_mode = EditMode::ToggleEdge;
        press_canvas(&mut this, V2f { x: 100.0, y: 100.0 });

        assert!(GraphWidget::handle_release(
            &mut this,
            V2f { x: 100.0, y: 100.0 }
        ));
        assert_eq!(this.graph.size(), 1);
        assert_eq!(
            vertex_position(&this.graph, VertexIndex { index: 0 }),
            V2f { x: 100.0, y: 100.0 }
        );
    }

    #[test]
    fn move_mode_does_not_create_vertices() {
        let mut this = state_with_vertices(&[]);
        this.edit_mode = EditMode::MoveVertex;
        press_canvas(&mut this, V2f { x: 100.0, y: 100.0 });

        assert!(!GraphWidget::handle_release(
            &mut this,
            V2f { x: 100.0, y: 100.0 }
        ));
        assert_eq!(this.graph.size(), 0);
    }

    #[test]
    fn dragging_across_canvas_does_not_add_vertex() {
        let mut this = state_with_vertices(&[]);
        this.edit_mode = EditMode::ToggleEdge;
        press_canvas(&mut this, V2f { x: 100.0, y: 100.0 });

        assert!(!GraphWidget::handle_release(
            &mut this,
            V2f { x: 200.0, y: 100.0 }
        ));
        assert_eq!(this.graph.size(), 0);
    }

    #[test]
    fn click_on_existing_vertex_does_not_add_vertex() {
        // Press lands next to a vertex but the cursor drifts onto it before release
        let mut this = state_with_vertices(&[V2f { x: 100.0, y: 100.0 }]);
        this.edit_mode = EditMode::ToggleEdge;
        press_canvas(
            &mut this,
            V2f {
                x: 100.0 + VERTEX_RADIUS + 1.0,
                y: 100.0,
            },
        );

        assert!(!GraphWidget::handle_release(
            &mut this,
            V2f {
                x: 100.0 + VERTEX_RADIUS - 1.0,
                y: 100.0
            }
        ));
        assert_eq!(this.graph.size(), 1);
    }

    #[test]
    fn new_vertex_is_kept_within_canvas() {
        let mut this = state_with_vertices(&[]);
        this.edit_mode = EditMode::ToggleEdge;
        press_canvas(&mut this, V2f { x: 0.0, y: 0.0 });

        assert!(GraphWidget::handle_release(
            &mut this,
            V2f { x: 0.0, y: 0.0 }
        ));
        assert_eq!(
            vertex_position(&this.graph, VertexIndex { index: 0 }),
            V2f {
                x: VERTEX_RADIUS,
                y: VERTEX_RADIUS
            }
        );
    }

    #[test]
    fn dragging_between_vertices_toggles_edge() {
        let (from, to) = (VertexIndex { index: 0 }, VertexIndex { index: 1 });
        let mut this =
            state_with_vertices(&[V2f { x: 100.0, y: 100.0 }, V2f { x: 200.0, y: 100.0 }]);

        press_edge(&mut this, from, V2f { x: 200.0, y: 100.0 });
        assert!(GraphWidget::handle_release(
            &mut this,
            V2f { x: 200.0, y: 100.0 }
        ));
        assert!(this.graph.are_adjacent(from, to));
        assert!(this.graph.are_adjacent(to, from));

        press_edge(&mut this, from, V2f { x: 200.0, y: 100.0 });
        assert!(GraphWidget::handle_release(
            &mut this,
            V2f { x: 200.0, y: 100.0 }
        ));
        assert!(!this.graph.are_adjacent(from, to));
        assert!(!this.graph.are_adjacent(to, from));
    }

    #[test]
    fn dragging_edge_onto_itself_is_ignored() {
        let from = VertexIndex { index: 0 };
        let mut this = state_with_vertices(&[V2f { x: 100.0, y: 100.0 }]);
        press_edge(&mut this, from, V2f { x: 100.0, y: 100.0 });

        assert!(!GraphWidget::handle_release(
            &mut this,
            V2f { x: 100.0, y: 100.0 }
        ));
        assert!(!this.graph.are_adjacent(from, from));
    }

    #[test]
    fn dropping_edge_on_empty_canvas_creates_connected_vertex() {
        let from = VertexIndex { index: 0 };
        let mut this = state_with_vertices(&[V2f { x: 100.0, y: 100.0 }]);
        this.edit_mode = EditMode::ToggleEdge;
        press_edge(&mut this, from, V2f { x: 300.0, y: 300.0 });

        assert!(GraphWidget::handle_release(
            &mut this,
            V2f { x: 300.0, y: 300.0 }
        ));
        assert_eq!(this.graph.size(), 2);

        let created = VertexIndex { index: 1 };
        assert_eq!(
            vertex_position(&this.graph, created),
            V2f { x: 300.0, y: 300.0 }
        );
        assert!(this.graph.are_adjacent(from, created));
    }

    #[test]
    fn dropping_edge_on_empty_canvas_is_ignored_when_disabled() {
        let mut this = state_with_vertices(&[V2f { x: 100.0, y: 100.0 }]);
        this.edit_mode = EditMode::ToggleEdge;
        this.edge_creates_vertex = false;
        press_edge(
            &mut this,
            VertexIndex { index: 0 },
            V2f { x: 300.0, y: 300.0 },
        );

        assert!(!GraphWidget::handle_release(
            &mut this,
            V2f { x: 300.0, y: 300.0 }
        ));
        assert_eq!(this.graph.size(), 1);
        assert_eq!(this.graph.edges().count(), 0);
    }

    #[test]
    fn remove_mode_removes_clicked_vertex_and_its_edges() {
        let mut this =
            state_with_vertices(&[V2f { x: 100.0, y: 100.0 }, V2f { x: 200.0, y: 100.0 }]);
        this.graph
            .connect(VertexIndex { index: 0 }, VertexIndex { index: 1 }, true);
        this.edit_mode = EditMode::RemoveVertex;
        press_vertex(&mut this, VertexIndex { index: 0 });

        assert!(GraphWidget::handle_release(
            &mut this,
            V2f { x: 100.0, y: 100.0 }
        ));
        assert_eq!(this.graph.size(), 1);
        assert_eq!(this.graph.edges().count(), 0);
        // The vertex that outlived the removal is the one that was not clicked
        assert_eq!(
            vertex_position(&this.graph, VertexIndex { index: 0 }),
            V2f { x: 200.0, y: 100.0 }
        );
    }

    #[test]
    fn remove_mode_does_not_create_vertices() {
        let mut this = state_with_vertices(&[]);
        this.edit_mode = EditMode::RemoveVertex;
        press_canvas(&mut this, V2f { x: 100.0, y: 100.0 });

        assert!(!GraphWidget::handle_release(
            &mut this,
            V2f { x: 100.0, y: 100.0 }
        ));
        assert_eq!(this.graph.size(), 0);
    }

    /// Press a vertex, drag the cursor through `path` and release wherever it ended
    fn press_drag_release(this: &mut SharedState, vertex: VertexIndex, path: &[V2f]) -> bool {
        let pressed_at = vertex_position(&this.graph, vertex);
        this.active = ActiveElement::Dragging(
            Target::Vertex(vertex),
            Drag::MoveVertex {
                grab_offset: V2f::ZERO,
                pressed_at,
                moved: false,
            },
        );

        for &cursor in path {
            GraphWidget::handle_drag(this, cursor);
        }
        GraphWidget::handle_release(this, path.last().copied().unwrap_or(pressed_at))
    }

    #[test]
    fn color_mode_colors_clicked_vertex() {
        let vertex = VertexIndex { index: 0 };
        let mut this = state_with_vertices(&[V2f { x: 100.0, y: 100.0 }]);
        this.edit_mode = EditMode::AddColorVertex(VertexColor::Blue);

        assert!(press_drag_release(&mut this, vertex, &[]));
        assert_eq!(vertex_color(&this, vertex), VertexColor::Blue);

        // Recoloring to the same color changes nothing, no need to bother the backend
        assert!(!press_drag_release(&mut this, vertex, &[]));
        assert_eq!(vertex_color(&this, vertex), VertexColor::Blue);
    }

    #[test]
    fn color_mode_drags_vertices_without_coloring_them() {
        let vertex = VertexIndex { index: 0 };
        let mut this = state_with_vertices(&[V2f { x: 100.0, y: 100.0 }]);
        this.edit_mode = EditMode::AddColorVertex(VertexColor::Red);

        assert!(press_drag_release(
            &mut this,
            vertex,
            &[V2f { x: 150.0, y: 100.0 }, V2f { x: 200.0, y: 120.0 }]
        ));
        assert_eq!(
            vertex_position(&this.graph, vertex),
            V2f { x: 200.0, y: 120.0 }
        );
        assert_eq!(vertex_color(&this, vertex), VertexColor::White);
    }

    #[test]
    fn move_mode_drags_vertices_but_a_click_changes_nothing() {
        let vertex = VertexIndex { index: 0 };
        let mut this = state_with_vertices(&[V2f { x: 100.0, y: 100.0 }]);
        this.edit_mode = EditMode::MoveVertex;

        assert!(press_drag_release(
            &mut this,
            vertex,
            &[V2f { x: 200.0, y: 100.0 }]
        ));
        assert_eq!(
            vertex_position(&this.graph, vertex),
            V2f { x: 200.0, y: 100.0 }
        );

        assert!(!press_drag_release(&mut this, vertex, &[]));
        assert_eq!(
            vertex_position(&this.graph, vertex),
            V2f { x: 200.0, y: 100.0 }
        );
    }

    #[test]
    fn a_wobbling_click_does_not_nudge_the_vertex() {
        let vertex = VertexIndex { index: 0 };
        let mut this = state_with_vertices(&[V2f { x: 100.0, y: 100.0 }]);
        this.edit_mode = EditMode::AddColorVertex(VertexColor::Green);

        let wobble = CLICK_SLOP - 1.0;
        assert!(press_drag_release(
            &mut this,
            vertex,
            &[
                V2f {
                    x: 100.0 + wobble,
                    y: 100.0
                },
                V2f { x: 100.0, y: 100.0 }
            ]
        ));
        assert_eq!(
            vertex_position(&this.graph, vertex),
            V2f { x: 100.0, y: 100.0 }
        );
        assert_eq!(vertex_color(&this, vertex), VertexColor::Green);
    }

    #[test]
    fn color_mode_creates_vertex_of_that_color() {
        let mut this = state_with_vertices(&[]);
        this.edit_mode = EditMode::AddColorVertex(VertexColor::Green);
        press_canvas(&mut this, V2f { x: 100.0, y: 100.0 });

        assert!(GraphWidget::handle_release(
            &mut this,
            V2f { x: 100.0, y: 100.0 }
        ));
        assert_eq!(
            vertex_color(&this, VertexIndex { index: 0 }),
            VertexColor::Green
        );
    }

    #[test]
    fn releasing_off_the_pressed_vertex_does_nothing() {
        let mut this =
            state_with_vertices(&[V2f { x: 100.0, y: 100.0 }, V2f { x: 200.0, y: 100.0 }]);
        this.edit_mode = EditMode::RemoveVertex;
        press_vertex(&mut this, VertexIndex { index: 0 });

        assert!(!GraphWidget::handle_release(
            &mut this,
            V2f { x: 200.0, y: 100.0 }
        ));
        assert_eq!(this.graph.size(), 2);
    }

    #[test]
    fn overlapping_vertices_pick_the_topmost_one() {
        let this = state_with_vertices(&[V2f { x: 100.0, y: 100.0 }, V2f { x: 105.0, y: 100.0 }]);

        assert_eq!(
            vertex_at(&this.graph, V2f { x: 102.0, y: 100.0 }),
            Some(VertexIndex { index: 1 })
        );
        assert_eq!(vertex_at(&this.graph, V2f { x: 300.0, y: 300.0 }), None);
    }

    #[test]
    fn vertices_are_clamped_to_the_resized_canvas() {
        let mut this = state_with_vertices(&[]);
        this.edit_mode = EditMode::ToggleEdge;
        this.canvas_size = MIN_CANVAS_SIZE;
        press_canvas(
            &mut this,
            V2f {
                x: 1000.0,
                y: 1000.0,
            },
        );

        assert!(GraphWidget::handle_release(
            &mut this,
            V2f {
                x: 1000.0,
                y: 1000.0
            }
        ));
        assert_eq!(
            vertex_position(&this.graph, VertexIndex { index: 0 }),
            V2f {
                x: MIN_CANVAS_SIZE.x - VERTEX_RADIUS,
                y: MIN_CANVAS_SIZE.y - VERTEX_RADIUS
            }
        );
    }

    #[test]
    fn vertex_shades_match_the_grid_widget_palette() {
        assert_eq!(shaded(VertexColor::Red, 1.0), "#f92672");
        assert_eq!(shaded(VertexColor::Red, 0.9), "#e02267");
    }
}
