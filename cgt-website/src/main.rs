use crate::{
    domineering::{Domineering, DomineeringState},
    snort::{Snort, SnortState},
};
use cgt::graph::undirected::Graph;
use sycamore::prelude::*;
use sycamore_router::{HistoryIntegration, Route, Router};

mod domineering;
mod snort;
mod utils;

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    sycamore::render(|cx| view! {cx, App {}})
}

#[derive(Route)]
enum Routes {
    #[to("/playground/domineering")]
    Domineering,
    #[to("/playground/snort")]
    Snort,
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
			let domineering_position =
			    cgt::short::partizan::games::domineering::Domineering::empty(4, 4).unwrap();
			let domineering_state = DomineeringState::new(cx, domineering_position);
			let snort_state = SnortState::new(
			    cx,
			    cgt::short::partizan::games::snort::Snort::new_three_star(10).unwrap());
			
			match route.get().as_ref() {
			    Routes::Domineering => view! {cx, Domineering(state=domineering_state)},
			    Routes::Snort => view! {cx, Snort(state=snort_state)},
			    Routes::NotFound => view! {cx, ":("},
			}
		    })
		}})
            }
	}
    }
}
