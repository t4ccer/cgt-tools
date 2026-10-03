//! Static website of cgt-tools, rendered to HTML at build time and hydrated in islands mode

#[cfg(feature = "ssr")]
mod chrome;
#[cfg(feature = "ssr")]
mod config;
#[cfg(feature = "ssr")]
mod guides;
#[cfg(feature = "ssr")]
mod jupyter;
#[cfg(feature = "ssr")]
mod pages;
#[cfg(feature = "ssr")]
mod play;
#[cfg(feature = "ssr")]
mod process;
#[cfg(feature = "ssr")]
mod python_docs;
pub mod quelhas;

#[cfg(feature = "ssr")]
pub use config::Config;
#[cfg(feature = "ssr")]
pub use guides::{Guide, read_guides};
#[cfg(feature = "ssr")]
pub use pages::pages;
#[cfg(feature = "ssr")]
pub use play::{Model, read_models};

/// Prints a progress message with the time since the first message, because running the guides
/// makes a build take a while
#[cfg(feature = "ssr")]
pub fn log(message: impl std::fmt::Display) {
    static START: std::sync::LazyLock<std::time::Instant> =
        std::sync::LazyLock::new(std::time::Instant::now);
    eprintln!("[{:>6.1}s] {message}", START.elapsed().as_secs_f64());
}
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

/// Marks the link to the section being read in a table of contents
#[island]
pub fn ScrollSpy() -> impl IntoView {
    #[cfg(feature = "hydrate")]
    spy_on_scroll();
}

#[cfg(feature = "hydrate")]
fn spy_on_scroll() {
    use wasm_bindgen::{JsCast, closure::Closure};

    /// How far below the top of the window a heading counts as reached, past the sticky navbar
    const REACHED: f64 = 100.0;

    let update = || {
        let document = document();
        let Ok(links) = document.query_selector_all(".section-nav a[href^='#']") else {
            return;
        };
        let links = (0..links.length())
            .filter_map(|i| links.item(i)?.dyn_into::<web_sys::Element>().ok())
            .collect::<Vec<_>>();
        let headings = links
            .iter()
            .filter_map(|link| {
                let id = link.get_attribute("href")?.strip_prefix('#')?.to_owned();
                let top = document
                    .get_element_by_id(&id)?
                    .get_bounding_client_rect()
                    .top();
                Some((id, top))
            })
            .collect::<Vec<_>>();
        let window = window();
        let height = window
            .inner_height()
            .ok()
            .and_then(|height| height.as_f64());
        let scrolled = window.scroll_y().ok();
        let page = document
            .document_element()
            .map(|root| f64::from(root.scroll_height()));
        // The last sections may be too short to ever reach the top
        let at_bottom = matches!((height, scrolled, page), (Some(height), Some(scrolled), Some(page)) if height + scrolled >= page - 2.0);
        let current = if at_bottom {
            headings.last()
        } else {
            headings.iter().rev().find(|(_, top)| *top <= REACHED)
        }
        .map(|(id, _)| format!("#{id}"));
        for link in &links {
            let active = current.is_some() && link.get_attribute("href") == current;
            let _ = link.class_list().toggle_with_force("active", active);
        }
    };
    update();
    let listener = Closure::<dyn Fn()>::new(update);
    for event in ["scroll", "resize"] {
        let _ = window().add_event_listener_with_callback(event, listener.as_ref().unchecked_ref());
    }
    // The listeners live as long as the page
    listener.forget();
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
