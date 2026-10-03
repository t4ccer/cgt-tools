use crate::{
    ThemeToggle,
    guides::{self, GUIDES_URL, Guide},
    play::{self, PLAY_URL},
    python_docs::{self, PythonApi},
};
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

/// Which sections the navigation bar links to
#[derive(Clone, Copy)]
struct Nav {
    play: bool,
}

/// Every page of the site as its path relative to the site root and its HTML, with a Python API
/// reference for each of `python_apis`, a page for each of `guides`, and the Quelhas page if there
/// are `models`, by their names and addresses, to play against
pub fn pages(
    mut python_apis: Vec<PythonApi>,
    guides: Vec<Guide>,
    models: Vec<(String, String)>,
) -> Vec<(String, String)> {
    let nav = Nav {
        play: !models.is_empty(),
    };
    let mut pages = vec![(
        "index.html".to_owned(),
        render("cgt-tools".to_owned(), "home", nav, move || {
            view! { <Home play=nav.play /> }
        }),
    )];
    if nav.play {
        pages.push((
            "play/index.html".to_owned(),
            render("Play - cgt-tools".to_owned(), "docs", nav, play::index_page),
        ));
        pages.push((
            "play/quelhas/index.html".to_owned(),
            render("Quelhas - cgt-tools".to_owned(), "docs", nav, move || {
                play::quelhas_page(models)
            }),
        ));
    }
    let links = guides.iter().map(Guide::link).collect::<Vec<_>>();
    {
        let links = links.clone();
        pages.push((
            "guides/index.html".to_owned(),
            render("Guides - cgt-tools".to_owned(), "docs", nav, move || {
                guides::index_page(&links)
            }),
        ));
    }
    for guide in guides {
        let path = format!("guides/{}/index.html", guide.slug);
        let title = format!("{} - cgt-tools", guide.title());
        let links = links.clone();
        pages.push((
            path,
            render(title, "docs", nav, move || guides::page(&guide, &links)),
        ));
    }
    let latest = python_docs::sort_versions(&mut python_apis);
    let versions = python_apis
        .iter()
        .map(|api| api.version.clone())
        .collect::<Vec<_>>();
    if let Some(target) = latest.as_ref().or_else(|| versions.first()) {
        pages.push((
            "python/docs/latest/index.html".to_owned(),
            redirect(&python_docs::docs_url(target)),
        ));
        let versions = versions.clone();
        let latest = latest.clone();
        pages.push((
            "python/docs/index.html".to_owned(),
            render(
                "Python API - cgt-tools".to_owned(),
                "docs",
                nav,
                move || python_docs::index_page(&versions, latest.as_deref()),
            ),
        ));
    }
    for api in python_apis {
        let path = format!("python/docs/{}/index.html", api.version);
        let title = format!("Python API {} - cgt-tools", api.version);
        let versions = versions.clone();
        let latest = latest.clone();
        pages.push((
            path,
            render(title, "docs", nav, move || {
                python_docs::page(&api, &versions, latest.as_deref())
            }),
        ));
    }
    pages
}

/// A page that sends visitors to `url`. The script keeps the fragment of the address, which the
/// refresh would drop, so links into a section of the target still work
fn redirect(url: &str) -> String {
    format!(
        "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>cgt-tools</title><script>location.replace(\"{url}\" + location.hash)</script><meta http-equiv=\"refresh\" content=\"0; url={url}\"></head><body><a href=\"{url}\">{url}</a></body></html>\n"
    )
}

fn render<V>(
    title: String,
    body_class: &'static str,
    nav: Nav,
    page: impl FnOnce() -> V + Send + 'static,
) -> String
where
    V: IntoView + 'static,
{
    let owner = Owner::new_root(Some(Arc::new(SsrSharedContext::new_islands())));
    owner.with(|| {
        let options = LeptosOptions::builder().output_name("cgt_website").build();
        view! { <Shell options title body_class nav>{page()}</Shell> }.to_html()
    })
}

#[component]
fn Shell(
    options: LeptosOptions,
    title: String,
    body_class: &'static str,
    nav: Nav,
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
                <Header nav />
                <main>{children()}</main>
                <Footer />
            </body>
        </html>
    }
}

#[component]
fn Header(nav: Nav) -> impl IntoView {
    view! {
        <header class="navbar">
            <a class="navbar-brand" href="/">
                "cgt-tools"
            </a>
            <nav class="navbar-nav">
                <a class="nav-link" href=GUIDES_URL>
                    "Guides"
                </a>
                {nav
                    .play
                    .then(|| {
                        view! {
                            <a class="nav-link" href=PLAY_URL>
                                "Play"
                            </a>
                        }
                    })}
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
fn Home(play: bool) -> impl IntoView {
    view! {
        <section class="hero">
            <h1>"cgt-tools"</h1>
            <p class="lead">{DESCRIPTION}</p>
            <a class="btn btn-primary" href=GUIDES_URL>
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
                        {if play {
                            view! {
                                <a href=PLAY_URL>"Play"</a>
                                " combinatorial games against an AI that runs in your browser or another player." // TODO: (locally or online)
                            }
                                .into_any()
                        } else {
                            "Coming soon!".into_any()
                        }}
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
