use sycamore::{prelude::*, rt::Event};

#[component(inline_props)]
pub fn AddRemoveButton<'a, G: Html, F, D>(
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
