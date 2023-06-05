use cgt::{domineering, transposition_table::TranspositionTable};
use leptos::*;

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    mount_to_body(
        |cx| view! {cx, <Domineering state=DomineeringState::new(cx, domineering::Position::empty(4, 4).unwrap())/>},
    )
}

struct DomineeringState {
    position: ReadSignal<domineering::Position>,
    set_position: WriteSignal<domineering::Position>,
    cache: ReadSignal<TranspositionTable<domineering::Position>>,
}

impl DomineeringState {
    fn new(cx: Scope, position: domineering::Position) -> Self {
        let (position, set_position) = create_signal(cx, position);
        let (cache, _) = create_signal(cx, TranspositionTable::new());
        DomineeringState {
            position,
            set_position,
            cache,
        }
    }
}

#[component]
fn Grid(cx: Scope, state: DomineeringState) -> impl IntoView {
    let width = move || state.position.get().width();
    let height = move || state.position.get().height();

    let on_click = move |x, y| {
        let old_value = state.position.get().at(x, y);
        state.set_position.update(|pos| pos.set(x, y, !old_value));
    };

    #[rustfmt::skip]
    view!{cx,
	  <div class="flex flex-col gap-y-2 border-4 border-light-gray w-fit p-2">
	  <For each = move || 0..height()
               key = |y| *y
               view = move |cx, y| {
		   view! {cx,
			  <div class="flex flex-row gap-x-2">
			    <For each = move || 0..width()
	                         key = |x| *x
      	                         view = move |cx, x| {
				     let class = move || if state.position.get().at(x, y) {
					 "bg-dark-gray aspect-square h-16"
				     } else {
					 "bg-light-gray aspect-square h-16"
				     };
				     view! {cx,
      					    <div class=class
					         on:click=move |_| on_click(x, y)>
					    </div>
				     }}/>
			  </div>
		   }}/>
	  </div>
    }
}

#[component]
fn Domineering(cx: Scope, state: DomineeringState) -> impl IntoView {
    let canonical_form = move || {
        state.cache.with(|c| {
            let game = state.position.get().canonical_form(c);
            let temp = c.game_backend().temperature(game);
            let canonical_form = c.game_backend().print_game_to_str(game);
            (canonical_form, temp)
        })
    };

    let resize = move |dx: i8, dy: i8| {
        let old_position = state.position.get();
        let new_width = (old_position.width() as i8 + dx) as u8;
        let new_height = (old_position.height() as i8 + dy) as u8;
        let mut new_position =
            match domineering::Position::empty(new_width.clamp(1, 5), new_height.clamp(1, 5)) {
                Err(_) => return,
                Ok(pos) => pos,
            };
        for y in 0..old_position.height() {
            for x in 0..old_position.width() {
                new_position.set(x, y, old_position.at(x, y));
            }
        }
        state.set_position.set(new_position);
    };

    #[rustfmt::skip]
    view! {cx,
	   <div class="flex flex-row gap-x-4">
             <div class="flex flex-col w-fit gap-y-4">
	         <Grid state/>
	         <div class="flex flex-row">
	           <div class="flex flex-row justify-center items-center w-12 h-10 grow border-2 border-dashed border-cyan rounded-lg gap-4">
	           <button class="flex w-fit aspect-square bg-pink rounded-lg" on:click = move |_| resize(0, -1)>
	             <i class="material-icons text-center">"remove"</i>
	           </button>
	           <button class="flex w-fit aspect-square bg-green rounded-lg" on:click = move |_| resize(0, 1)>
	             <i class="material-icons text-center">"add"</i>
	           </button>
	         </div>
	       </div>
             </div>
	     <div class="flex flex-col">
	       <div class="flex flex-col justify-center items-center w-12 h-10 grow border-2 border-dashed border-cyan rounded-lg gap-4">
	         <button class="flex w-fit aspect-square bg-green rounded-lg" on:click = move |_| resize(1, 0)>
	           <i class="material-icons text-center">"add"</i>
	         </button>
	         <button class="flex w-fit aspect-square bg-pink rounded-lg"  on:click = move |_| resize(-1, 0)>
	           <i class="material-icons text-center">"remove"</i>
	         </button>
	       </div>
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
