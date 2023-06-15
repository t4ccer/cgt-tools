use cgt::{domineering, transposition_table::TranspositionTable};
use sycamore::{prelude::*, rt::Event};
use sycamore_router::{HistoryIntegration, Route, Router};

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    sycamore::render(|cx| view! {cx, App {}})
}

#[derive(Route)]
enum Routes {
    #[to("/playground/domineering")]
    Domineering,
    #[not_found]
    NotFound,
}

#[component]
fn App<'a, G: Html>(cx: Scope<'a>) -> View<G> {
    #[rustfmt::skip]
    view! {
	cx,
	div(class="flex flex-col w-screen justify-center items-center gap-y-4") {
            nav(class="flex w-full bg-gray h-12 sticky top-0") {}
            main(class="flex w-full max-w-6xl") {
		Router(integration=HistoryIntegration::new(), view=|cx, route: &ReadSignal<Routes>| { view! {
		    cx,
		    ({
			let position = domineering::Position::empty(4, 4).unwrap();
			let state = DomineeringState::new(cx, position);
			match route.get().as_ref() {
			    Routes::Domineering => view! {cx, Domineering(state=state)},
			    Routes::NotFound => view! {cx, ":("},
			}
		    })
		}})
            }
	}
    }
}

#[derive(Clone, Copy)]
struct DomineeringState<'a> {
    position: &'a Signal<domineering::Position>,
    cache: &'a ReadSignal<TranspositionTable<domineering::Position>>,
}

impl<'a> DomineeringState<'a> {
    fn new(cx: Scope<'a>, position: domineering::Position) -> Self {
        let position = create_signal(cx, position);
        let cache = create_signal(cx, TranspositionTable::new(GRID_MAX_SIZE));
        DomineeringState { position, cache }
    }
}

#[component(inline_props)]
fn Grid<'a, G: Html>(cx: Scope<'a>, state: DomineeringState<'a>) -> View<G> {
    let width = state.position.map(cx, |pos| (0..pos.width()).collect());
    let height = state.position.map(cx, |pos| (0..pos.height()).collect());

    let on_click = move |x, y| {
        let mut old_position = *state.position.get();
        let old_value = old_position.at(x, y);
        old_position.set(x, y, !old_value);
        state.position.set(old_position);
    };

    #[rustfmt::skip]
    view!{cx,
	  div(class="flex flex-col gap-y-2 border-4 border-light-gray w-fit p-2") {
	      Keyed(iterable=height, key=|y| *y, view=move |cx, y| view! {
		  cx,
		  div(class="flex flex-row gap-x-2") {
		      Keyed(iterable=width, key=|x| *x, view=move |cx, x| view! {
			  cx,
			  ({
			      let tile_class = state.position.map(cx, move |pos| {
				  if pos.at(x, y) {
				      "bg-dark-gray cursor-pointer aspect-square h-16"
				  } else {
				      "bg-light-gray cursor-pointer aspect-square h-16"
				  }
			      });
			      view!{cx, div(class=tile_class, on:click=move |_| on_click(x, y)) }
			  })
			  
		      })
		  }
	      })
	  }
    }
}

fn resize<'a>(state: DomineeringState<'a>, dx: i8, dy: i8) {
    let old_position = state.position.get();
    let new_width = (old_position.width() as i8 + dx) as u8;
    let new_height = (old_position.height() as i8 + dy) as u8;

    let mut new_position = match domineering::Position::empty(new_width, new_height) {
        Err(_) => return,
        Ok(pos) => pos,
    };
    for y in 0..old_position.height() {
        for x in 0..old_position.width() {
            new_position.set(x, y, old_position.at(x, y));
        }
    }
    state.position.set(new_position);
}

const GRID_MAX_SIZE: u8 = 16;

#[component(inline_props)]
fn ResizeWidth<'a, G: Html>(cx: Scope<'a>, state: DomineeringState<'a>) -> View<G> {
    let block_expand_width = create_memo(cx, || {
        let position = state.position.get();
        (position.height() + 1) * position.width() > GRID_MAX_SIZE
    });

    #[rustfmt::skip]
    view! {
	cx,
	div(class="flex flex-col") {
	    div(class="flex flex-col justify-center items-center w-12 h-10 grow border-2 \
		       border-dashed border-cyan rounded-lg gap-4") {
		AddRemoveButton(enabled_color = "bg-green".to_string(),
                                icon = "add".to_string(),
                                on_click = move |_| resize(state, 1, 0),
                                disabled = || *block_expand_width.get())

		AddRemoveButton(enabled_color = "bg-pink".to_string(),
                                icon = "remove".to_string(),
                                on_click = move |_| resize(state, -1, 0),
                                disabled = move || state.position.get().width() <= 1)
	    }
	}
    }
}

#[component(inline_props)]
fn AddRemoveButton<'a, G: Html, F, D>(
    cx: Scope<'a>,
    enabled_color: String,
    icon: String,
    on_click: F,
    disabled: D,
) -> View<G>
where
    F: FnMut(Event) + 'a,
    D: Fn() -> bool + 'a,
{
    #[rustfmt::skip]
    view! {
	cx,
	button(class=format!("{} flex w-fit aspect-square disabled:bg-gray \
			      disabled:cursor-not-allowed rounded-lg", enabled_color),
	       on:click=on_click,
	       prop:disabled = disabled())
	{
	    i(class="material-icons text-center"){
		(icon)
	    }
	}
    }
}

#[component(inline_props)]
fn ResizeHeight<'a, G: Html>(cx: Scope<'a>, state: DomineeringState<'a>) -> View<G> {
    let block_expand_height = create_memo(cx, || {
        let position = state.position.get();
        (position.height() + 1) * position.width() > GRID_MAX_SIZE
    });

    #[rustfmt::skip]
    view! {
	cx,
	div(class="flex flex-row") {
	    div(class="flex flex-row justify-center items-center w-12 h-10 grow border-2 \
		       border-dashed border-cyan rounded-lg gap-4") {
		AddRemoveButton(enabled_color = "bg-pink".to_string(),
                                icon = "remove".to_string(),
                                on_click = move |_| resize(state, 0, -1),
                                disabled = move || state.position.get().height() <= 1)

		AddRemoveButton(enabled_color = "bg-green".to_string(),
                                icon = "add".to_string(),
                                on_click = move |_| resize(state, 0, 1),
                                disabled = || *block_expand_height.get())
	    }
	}
    }
}

#[component(inline_props)]
fn Domineering<'a, G: Html>(cx: Scope<'a>, state: DomineeringState<'a>) -> View<G> {
    let game_info = state.position.map(cx, |pos| {
        let cache = state.cache.get();
        let game = pos.canonical_form(&cache);
        let canonical_form = cache.game_backend().print_game_to_str(game);
        let temp = cache.game_backend().temperature(game);
        (canonical_form, temp)
    });
    let canonical_form = game_info.map(cx, |(cf, _)| cf.clone());
    let temperature = game_info.map(cx, |(_, temp)| temp.clone());

    #[rustfmt::skip]
    view!{
	cx,
	div(class="flex flex-row gap-x-4") {
	    div(class="flex flex-col w-fit gap-y-4") {
		Grid(state=state)
		ResizeHeight(state=state)
	    }
            ResizeWidth(state=state)
            div(class="flex flex-col") {
		span(class="text-white font-mono"){"Canonical form: " (canonical_form.get())}
		span(class="text-white font-mono"){"Temperature: " (temperature.get())}
		span(class="text-white font-mono"){"Canonical form: TODO"}
	    }
	}
    }
}
