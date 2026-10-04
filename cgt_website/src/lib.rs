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
pub mod play;
#[cfg(feature = "ssr")]
mod process;
#[cfg(feature = "ssr")]
mod python_docs;

#[cfg(feature = "ssr")]
pub use config::Config;
#[cfg(feature = "ssr")]
pub use guides::{Guide, read_guides};
#[cfg(feature = "ssr")]
pub use pages::pages;
#[cfg(feature = "ssr")]
pub use play::pages::{AiModel, GAMES, GamePage, read_models};

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
    use std::{cell::Cell, rc::Rc};
    use wasm_bindgen::{JsCast, closure::Closure};
    use web_sys::Element;

    /// How far below the top of the window a heading counts as reached, past the sticky navbar
    const REACHED: f64 = 100.0;

    let document = document();
    let Ok(links) = document.query_selector_all(".section-nav a[href^='#']") else {
        return;
    };
    // The headings do not change, so each link is paired with its heading once. The same heading
    // is linked from the sidebar and from the table of contents for small screens
    let targets: Vec<(Element, Element)> = (0..links.length())
        .filter_map(|i| {
            let link = links.item(i)?.dyn_into::<Element>().ok()?;
            let id = link.get_attribute("href")?.strip_prefix('#')?.to_owned();
            Some((link, document.get_element_by_id(&id)?))
        })
        .collect();
    let update = move || {
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
            targets.last()
        } else {
            targets
                .iter()
                .rev()
                .find(|(_, heading)| heading.get_bounding_client_rect().top() <= REACHED)
        }
        .map(|(_, heading)| heading);
        for (link, heading) in &targets {
            let active = current == Some(heading);
            let _ = link.class_list().toggle_with_force("active", active);
        }
    };
    update();
    // Scrolling fires many events per frame, and the links only need updating once a frame
    let scheduled = Rc::new(Cell::new(false));
    let frame = {
        let scheduled = scheduled.clone();
        Closure::<dyn FnMut()>::new(move || {
            scheduled.set(false);
            update();
        })
    };
    let listener = Closure::<dyn FnMut()>::new(move || {
        if !scheduled.replace(true) {
            let _ = window().request_animation_frame(frame.as_ref().unchecked_ref());
        }
    });
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
