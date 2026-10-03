use crate::ThemeToggle;
use hydration_context::SsrSharedContext;
use leptos::{prelude::*, reactive::owner::Owner};
use std::sync::Arc;

const PYTHON_DOCS: &str = "/python/docs/latest/";
const RUST_DOCS: &str = "https://docs.rs/cgt/latest/cgt/";
const REPOSITORY: &str = "https://github.com/t4ccer/cgt-tools";
const DESCRIPTION: &str = "Combinatorial Game Theory toolkit in Rust and Python";

/// Applies the color scheme picked with [`ThemeToggle`] before the first paint, so that a page
/// does not flash in the system scheme while the islands load
const THEME_SCRIPT: &str = r#"try{const t=localStorage.getItem("theme");if(t)document.documentElement.dataset.theme=t}catch(_){}"#;

/// Every page of the site as its path relative to the site root and its HTML
pub fn pages() -> Vec<(&'static str, String)> {
    vec![(
        "index.html",
        render("cgt-tools", "home", || view! { <Home /> }),
    )]
}

fn render<V>(
    title: &'static str,
    body_class: &'static str,
    page: impl FnOnce() -> V + Send + 'static,
) -> String
where
    V: IntoView + 'static,
{
    let owner = Owner::new_root(Some(Arc::new(SsrSharedContext::new_islands())));
    owner.with(|| {
        let options = LeptosOptions::builder().output_name("cgt_website").build();
        view! { <Shell options title body_class>{page()}</Shell> }.to_html()
    })
}

#[component]
fn Shell(
    options: LeptosOptions,
    title: &'static str,
    body_class: &'static str,
    children: Children,
) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <meta name="description" content=DESCRIPTION />
                <title>{title}</title>
                <script inner_html=THEME_SCRIPT></script>
                <link rel="stylesheet" href="/style.css" />
                <HydrationScripts options islands=true />
            </head>
            <body class=body_class>
                <Header />
                <main>{children()}</main>
                <Footer />
            </body>
        </html>
    }
}

#[component]
fn Header() -> impl IntoView {
    view! {
        <header class="navbar">
            <a class="navbar-brand" href="/">
                "cgt-tools"
            </a>
            <nav class="navbar-nav">
                <a class="nav-link" href=PYTHON_DOCS>
                    "Python"
                </a>
                <a class="nav-link" href=RUST_DOCS>
                    "Rust"
                </a>
            </nav>
            <ThemeToggle />
            <a class="nav-link icon-link" href=REPOSITORY aria-label="GitHub repository">
                <GithubIcon />
            </a>
        </header>
    }
}

#[component]
fn Footer() -> impl IntoView {
    view! {
        <footer class="footer">
            <span>
                "Copyright © 2023-2026 Tomasz Maciosowski, "
                <a href=format!("{REPOSITORY}/blob/main/LICENSE")>"AGPL-3.0"</a>
            </span>
            <span>"Theme adapted from " <a href="https://github.com/thuliteio/doks">"Doks"</a></span>
        </footer>
    }
}

#[component]
fn Home() -> impl IntoView {
    view! {
        <section class="hero">
            <h1>"cgt-tools"</h1>
            <p class="lead">{DESCRIPTION}</p>
            <a class="btn btn-primary"> // TODO
                "Guides"
            </a>
            <a class="btn btn-outline" href=PYTHON_DOCS>
                "Python Docs"
            </a>
            <a class="btn btn-outline" href=RUST_DOCS>
                "Rust Docs"
            </a>
        </section>
        <div class="bg-dots"></div>
        <section class="features">
            <div class="container">
                <div class="row">
                    <Feature title="Python and Jupyter">
                        <code>"pip install cgt-py"</code>
                        " brings the library to Python, with interactive widgets for exploring positions in Jupyter."
                    </Feature>
                    <Feature title="Rust Library">
                        <code>"cargo add cgt"</code>
                        " for native performance computations and position searches, in parallel and with transposition tables."
                    </Feature>
                    <Feature title="Command Line">
                        <code>"cgt-cli"</code>
                        " runs searches from the terminal."
                    </Feature>
                    <Feature title="Train AI Players">
                        "Train AlphaZero-style computer players for combinatorial games."
                    </Feature>
                    <Feature title="Play Combinatorial Games">
                        "Coming soon!"
                    </Feature>
                </div>
            </div>
        </section>
    }
}

#[component]
fn Feature(title: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="feature">
            <h2>{title}</h2>
            <p>{children()}</p>
        </div>
    }
}

#[component]
fn GithubIcon() -> impl IntoView {
    view! {
        <svg
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
            <path d="M9 19c-4.3 1.4 -4.3 -2.5 -6 -3m12 5v-3.5c0 -1 .1 -1.4 -.5 -2c2.8 -.3 5.5 -1.4 5.5 -6a4.6 4.6 0 0 0 -1.3 -3.2a4.2 4.2 0 0 0 -.1 -3.2s-1.1 -.3 -3.5 1.3a12.3 12.3 0 0 0 -6.2 0c-2.4 -1.6 -3.5 -1.3 -3.5 -1.3a4.2 4.2 0 0 0 -.1 3.2a4.6 4.6 0 0 0 -1.3 3.2c0 4.6 2.7 5.7 5.5 6c-.6 .6 -.6 1.2 -.5 2v3.5" />
        </svg>
    }
}
