//! Static website of cgt-tools, rendered to HTML at build time and hydrated in islands mode

#[cfg(feature = "ssr")]
mod pages;
#[cfg(feature = "ssr")]
mod python_docs;

#[cfg(feature = "ssr")]
pub use pages::pages;
#[cfg(feature = "ssr")]
pub use python_docs::{Package, PythonApi};

use leptos::prelude::*;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_islands();
}

#[island]
pub fn ThemeToggle() -> impl IntoView {
    view! {
        <button
            class="nav-link icon-link"
            type="button"
            aria-label="Toggle dark mode"
            on:click=|_| {
                #[cfg(feature = "hydrate")]
                toggle_theme();
            }
        >
            <MoonIcon />
            <SunIcon />
        </button>
    }
}

#[cfg(feature = "hydrate")]
fn toggle_theme() {
    let window = window();
    let Some(root) = document().document_element() else {
        return;
    };
    let dark = root.get_attribute("data-theme").map_or_else(
        || {
            window
                .match_media("(prefers-color-scheme: dark)")
                .ok()
                .flatten()
                .is_some_and(|query| query.matches())
        },
        |theme| theme == "dark",
    );
    let theme = if dark { "light" } else { "dark" };
    let _ = root.set_attribute("data-theme", theme);
    if let Ok(Some(storage)) = window.local_storage() {
        let _ = storage.set_item("theme", theme);
    }
}

#[component]
fn MoonIcon() -> impl IntoView {
    view! {
        <svg
            class="icon-moon"
            xmlns="http://www.w3.org/2000/svg"
            width="24"
            height="24"
            viewBox="0 0 24 24"
            stroke-width="2"
            stroke="currentColor"
            fill="none"
            stroke-linecap="round"
            stroke-linejoin="round"
        >
            <path d="M12 3c.132 0 .263 0 .393 0a7.5 7.5 0 0 0 7.92 12.446a9 9 0 1 1 -8.313 -12.454l0 .008" />
        </svg>
    }
}

#[component]
fn SunIcon() -> impl IntoView {
    view! {
        <svg
            class="icon-sun"
            xmlns="http://www.w3.org/2000/svg"
            width="24"
            height="24"
            viewBox="0 0 24 24"
            stroke-width="2"
            stroke="currentColor"
            fill="none"
            stroke-linecap="round"
            stroke-linejoin="round"
        >
            <path d="M8 12a4 4 0 1 0 8 0a4 4 0 1 0 -8 0" />
            <path d="M3 12h1m8 -9v1m8 8h1m-9 8v1m-6.4 -15.4l.7 .7m12.1 -.7l-.7 .7m0 11.4l.7 .7m-12.1 -.7l-.7 .7" />
        </svg>
    }
}
