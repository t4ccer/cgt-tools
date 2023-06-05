use cgt::{domineering, transposition_table::TranspositionTable};
use leptos::*;

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    let position = domineering::Position::empty(4, 4).unwrap();
    mount_to_body(|cx| view! {cx, <Domineering position/>})
}

#[component]
pub fn Domineering(cx: Scope, position: domineering::Position) -> impl IntoView {
    let (position, set_position) = create_signal(cx, position);
    let (cache, _) = create_signal(cx, TranspositionTable::new());

    let width = position.get().width();
    let height = position.get().height();

    let on_click = move |x, y| {
        let old_value = position.get().at(x, y);
        set_position.update(|pos| pos.set(x, y, !old_value));
    };

    let canonical_form = move || {
        cache.with(|c| {
            let game = position.get().canonical_form(c);
            let temp = c.game_backend().temperature(game);
            let canonical_form = c.game_backend().print_game_to_str(game);
            (canonical_form, temp)
        })
    };

    #[rustfmt::skip]
    view! {cx,
	   <div class="flex flex-row gap-x-4">
             <div class="flex flex-col gap-y-2 border-4 w-fit p-2">
	     <For each = move || 0..height
	          key = |y| *y
                  view = move |cx, y| {
		      view! {cx,
			     <div class="flex flex-row gap-x-2">
			       <For each = move || 0..width
	                            key = |x| *x
      	                            view = move |cx, x| {
					let class = move || if position.get().at(x, y) {
					    "bg-neutral-800 aspect-square h-16"
					} else {
					    "bg-neutral-200 aspect-square h-16"
					};
					view! {cx,
      					       <div class=class
					            on:click=move |_| on_click(x, y)>
					       </div>
					}
				    }/>
			     </div>
		      }}/>
             </div>
	     <div class="flex flex-col">  
               <span class="text-white font-mono">"Canonical form: "
	         {move || canonical_form().0}
	       </span>
               <span class="text-white font-mono">"Temperature: "
	         {move || canonical_form().1.to_string()}
	       </span>
	     </div>
	   </div>
    }
}
