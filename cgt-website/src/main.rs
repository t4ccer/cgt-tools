use crate::domineering::{Domineering, DomineeringState};
use sycamore::prelude::*;
use sycamore_router::{HistoryIntegration, Route, Router};

mod domineering;
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
			let domineering_position = cgt::domineering::Position::empty(4, 4).unwrap();
			let domineering_state = DomineeringState::new(cx, domineering_position);
			match route.get().as_ref() {
			    Routes::Domineering => view! {cx, Domineering(state=domineering_state)},
			    Routes::Snort => view! {cx, "TODO"},
			    Routes::NotFound => view! {cx, ":("},
			}
		    })
		}})
            }
	}
    }
}
