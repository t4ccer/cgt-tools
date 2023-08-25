use std::collections::HashMap;

use crate::mouse::{MouseState, Position};
use cgt::short::partizan::{
    games::snort::Snort, partizan_game::PartizanGame, transposition_table::TranspositionTable,
};
use leptos::{ev::mousedown, *};
use leptos_use::use_element_size;
use viz_js::VizInstance;
use wasm_bindgen::UnwrapThrowExt;

mod mouse;

pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    log::info!("Running");
    mount_to_body(|cx| Playground(cx))
}

/// Return value if Some, short circuit if None. Intended to be use in unit effects
macro_rules! retry_option {
    ($e:expr) => {
        match $e {
            Some(e) => e,
            None => return,
        }
    };
}

#[derive(Clone)]
struct State {
    next_idx: u32,
    nodes: HashMap<u32, Component>,
}

impl State {
    fn new() -> Self {
        Self {
            next_idx: 0,
            nodes: HashMap::new(),
        }
    }

    fn add_component(&mut self, component: Component) -> u32 {
        let idx = self.next_idx;
        self.next_idx += 1;
        self.nodes.insert(idx, component);
        idx
    }
}

#[derive(Clone, Copy)]
enum Component {
    Snort(RwSignal<Snort>),
    LeftMovesOf(ReadSignal<Snort>),
}

#[component]
pub fn Playground(cx: Scope) -> impl IntoView {
    let workspace_ref = create_node_ref(cx);
    let workspace_size = use_element_size(cx, workspace_ref);

    let mouse = MouseState::new(cx, workspace_ref);

    let viewport_x = create_memo::<f64>(cx, move |x| {
        let dx = mouse.middle.delta_x.get();
        match x {
            None => 0.0,
            Some(x) => x - dx,
        }
    });
    let viewport_y = create_memo::<f64>(cx, move |y| {
        let dy = mouse.middle.delta_y.get();
        match y {
            None => 0.0,
            Some(y) => y - dy,
        }
    });

    let state = create_rw_signal(cx, State::new());
    state.update(|state| {
        state.add_component(Component::Snort(create_rw_signal(
            cx,
            Snort::new_three_star(8).unwrap_throw(),
        )));
    });

    // FIXME: It may be actually OK. It needs to live through whole app anyway
    let snort_tt: &'static _ = Box::leak(Box::new(TranspositionTable::new()));

    let rw_focused = create_rw_signal(cx, None);

    let workspace = svg::svg(cx)
        .attr("viewBox", move || {
            format!(
                "{} {} {} {}",
                viewport_x.get(),
                viewport_y.get(),
                workspace_size.width.get(),
                workspace_size.height.get()
            )
        })
        .attr("height", move || workspace_size.height.get())
        .attr("width", move || workspace_size.width.get())
        .attr("xmlns", "http://www.w3.org/2000/svg")
        .child(For(
            cx,
            ForProps::builder()
                .each(move || state.get().nodes)
                .key(|(idx, _)| *idx)
                .view(move |cx, component| match component.1 {
                    Component::Snort(snort) => SnortComponent(
                        cx,
                        SnortComponentProps::builder()
                            .idx(component.0)
                            .details(rw_focused)
                            .mouse(mouse)
                            .initial_position(Position {
                                x: 150.0 + viewport_x.get_untracked(),
                                y: 50.0 + viewport_y.get_untracked(),
                            })
                            .snort(move || snort.get())
                            .build(),
                    )
                    .into_view(cx),
                    Component::LeftMovesOf(snort) => For(
                        cx,
                        ForProps::builder()
                            .each(move || snort.get().sensible_left_moves(&snort_tt))
                            .key(|p| p.clone())
                            .view(move |cx, snort_move| {
                                let snort_move = snort_move.clone();
                                let get_snort_move = move || snort_move.clone();
                                SnortComponent(
                                    cx,
                                    SnortComponentProps::builder()
                                        .idx(component.0)
                                        .details(rw_focused)
                                        .mouse(mouse)
                                        .initial_position(Position {
                                            x: 150.0 + viewport_x.get_untracked(),
                                            y: 50.0 + viewport_y.get_untracked(),
                                        })
                                        .snort(get_snort_move)
                                        .build(),
                                )
                                .into_view(cx)
                            })
                            .build(),
                    )
                    .into_view(cx),
                })
                .build(),
        ));

    let add_snort = move || {
        state.update(|state| {
            state.add_component(Component::Snort(create_rw_signal(
                cx,
                Snort::new_three_star(6).unwrap_throw(),
            )));
        });
    };

    html::div(cx)
        .classes("flex w-screen h-screen")
        .child(
            html::div(cx).classes("flex w-1/6 h-screen bg-gray").child(
                html::button(cx)
                    .child("Snort")
                    .on(ev::click, move |_| add_snort()),
            ),
        )
        .child(
            html::div(cx)
                .node_ref(workspace_ref)
                .classes("flex w-4/6 h-screen")
                .child(workspace),
        )
        .child(
            html::div(cx)
                .classes("flex w-1/6 h-screen bg-gray")
                .child(Details(
                    cx,
                    DetailsProps::builder()
                        .focused(rw_focused)
                        .state(state)
                        .build(),
                )),
        )
}

#[component]
fn Details(cx: Scope, focused: RwSignal<Option<u32>>, state: RwSignal<State>) -> impl IntoView {
    let component = move || {
        focused
            .get()
            .and_then(|idx| state.get().nodes.get(&idx).copied())
    };
    html::span(cx).child(move || match component() {
        // FIXME: Re-render this without unfocusing
        None => html::div(cx),
        Some(component) => match component {
            Component::Snort(snort) => {
                let invalid_pos_input = create_rw_signal(cx, false);
                html::div(cx)
                    .classes("flex flex-col")
                    .child(
                        html::span(cx).child(move || format!("Degree: {}", snort.get().degree())),
                    )
                    .child(
                        html::span(cx).child("Position: ").child(
                            html::input(cx)
                                .prop("value", serde_json::to_string(&snort.get()).unwrap())
                                .on(ev::input, move |ev| {
                                    let pos_str = event_target_value(&ev);
                                    let new_snort = serde_json::from_str::<Snort>(&pos_str);
                                    match new_snort {
                                        Ok(new_snort) => {
                                            invalid_pos_input.set(false);
                                            snort.set(new_snort);
                                        }
                                        Err(_) => {
                                            invalid_pos_input.set(true);
                                        }
                                    }
                                }),
                        ),
                    )
                    .child(
                        html::span(cx)
                            .classes("text-pink font-bold")
                            .child(move || {
                                if invalid_pos_input.get() {
                                    "Invalid Position"
                                } else {
                                    ""
                                }
                            }),
                    )
                    .child(
                        html::button(cx)
                            .child("Get Sensible Left")
                            .on(ev::click, move |_| {
                                state.update(|state| {
                                    state.add_component(Component::LeftMovesOf(snort.read_only()));
                                });
                            }),
                    )
            }
            Component::LeftMovesOf(_) => html::div(cx),
        },
    })
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Svg {
    inner: String,
    attributes: Vec<(String, String)>,
}

impl Svg {
    fn render(&self, elem: HtmlElement<svg::Svg>) {
        for (attr_name, attr_value) in &self.attributes {
            let attr_value = if &*attr_name == "width" || &*attr_name == "height" {
                attr_value.strip_suffix("pt").unwrap_or(&*attr_value)
            } else {
                &*attr_value
            };
            elem.set_attribute(&attr_name, &attr_value).unwrap_throw();
        }
        elem.set_inner_html(&self.inner);
    }
}

async fn snort_to_svg(snort: Snort) -> Svg {
    let graphviz = VizInstance::new().await;
    let dot = snort.to_graphviz();
    let svg = graphviz
        .render_svg_element(dot, viz_js::Options::default())
        .expect_throw("Could not render graphviz");
    let inner = svg.inner_html();
    let attributes = svg
        .get_attribute_names()
        .iter()
        .map(|name| {
            let name = name.as_string().expect_throw("Attribute is not a string");
            let value = svg.get_attribute(&name).expect_throw("Unrechable");
            (name, value)
        })
        .collect::<Vec<_>>();
    Svg { inner, attributes }
}

fn snap_to_grid(inp: f64) -> f64 {
    let grid_size = 25.;
    (inp / grid_size).round() * grid_size
}

#[component]
fn SnortComponent<F>(
    cx: Scope,
    idx: u32,
    mouse: MouseState,
    initial_position: Position,
    details: RwSignal<Option<u32>>,
    snort: F,
) -> impl IntoView
where
    F: Fn() -> Snort + 'static,
{
    let svg_ref = create_node_ref(cx);

    let block_pos = BlockPosition::new(cx, initial_position, mouse);

    let snort_svg = create_resource(
        cx,
        move || snort(),
        |snort| async move { snort_to_svg(snort).await },
    );

    create_effect(cx, move |_| {
        let rect: HtmlElement<svg::Svg> = retry_option!(svg_ref.get());
        let svg = retry_option!(snort_svg.read(cx));
        svg.render(rect);
    });

    svg::svg(cx)
        .on(mousedown, move |_| block_pos.set_can_be_moved.set(true)) // TODO: Do it only in 'moving' mode
        .on(ev::click, move |_| details.set(Some(idx)))
        .attr("x", move || snap_to_grid(block_pos.x.get()))
        .attr("y", move || snap_to_grid(block_pos.y.get()))
        .child(
            svg::svg(cx)
                .node_ref(svg_ref)
                .attr("width", 350)
                .attr("height", 220),
        )
}

#[component]
fn BlockComponent(
    cx: Scope,
    mouse: MouseState,
    initial_position: Position,
    width: u32,
    height: u32,
) -> impl IntoView {
    let rect_ref = create_node_ref(cx);
    let block_pos = BlockPosition::new(cx, initial_position, mouse);
    svg::rect(cx)
        .node_ref(rect_ref)
        .on(mousedown, move |_| block_pos.set_can_be_moved.set(true)) // TODO: Do it only in 'moving' mode
        .attr("x", move || block_pos.x.get())
        .attr("y", move || block_pos.y.get())
        .attr("width", width)
        .attr("height", height)
        .attr("fill", "red")
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockPosition {
    x: Memo<f64>,
    y: Memo<f64>,
    set_can_be_moved: WriteSignal<bool>,
}

impl BlockPosition {
    fn new(cx: Scope, initial_pos: Position, mouse: MouseState) -> Self {
        let (can_be_moved, set_can_be_moved) = create_signal(cx, false);

        // It shouldn't require an effect to achieve that, docs says to not write to signals
        // in effects, but I'm out of ideas how to handle that.
        create_effect(cx, move |_| {
            let is_pressed = mouse.left.pressed.get();
            if !is_pressed {
                set_can_be_moved.set(false);
            }
        });

        macro_rules! mk_pos_memo {
            ($initial: expr, $delta: expr) => {
                create_memo::<f64>(cx, move |old| {
                    let should_be_moved = can_be_moved.get();
                    match old {
                        None => $initial,
                        Some(old) => {
                            if should_be_moved {
                                old + $delta.get()
                            } else {
                                *old
                            }
                        }
                    }
                })
            };
        }

        let rect_pos_x = mk_pos_memo!(initial_pos.x, mouse.left.delta_x);
        let rect_pos_y = mk_pos_memo!(initial_pos.y, mouse.left.delta_y);

        Self {
            x: rect_pos_x,
            y: rect_pos_y,
            set_can_be_moved,
        }
    }
}
