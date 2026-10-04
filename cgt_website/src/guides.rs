use crate::{
    ScrollSpy,
    jupyter::{Jupyter, WIDGET_VIEW},
    log,
};
use leptos::prelude::*;
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
    sync::LazyLock,
};
use syntect::{
    html::{ClassStyle, ClassedHTMLGenerator},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);

/// A guide, written as a Jupyter notebook so that it can be run and downloaded
pub struct Guide {
    /// Name of the notebook without its extension, which is also the directory its page is
    /// written to
    pub slug: String,
    title: String,
    lead: String,
    blocks: Vec<Block>,
    /// Level 2 headings as their anchors and texts
    headings: Vec<(String, String)>,
    /// For a notebook, the notebook with its outputs cleared. Saved widget outputs only show
    /// "model not found" in a notebook reopened without their widget state
    download: Option<String>,
    /// Screenshots of widget outputs, by the index of their cell. A saved widget output has
    /// nothing to show, because only a running kernel and front end can draw it
    screenshots: BTreeMap<usize, Vec<u8>>,
}

/// What a guide links to from its own and from other pages
#[derive(Clone)]
pub struct GuideLink {
    slug: String,
    title: String,
    lead: String,
}

enum Block {
    Markdown(String),
    Code {
        source: String,
        highlighted: Option<String>,
        outputs: Vec<Rendered>,
    },
}

enum Rendered {
    Text(String, &'static str),
    Html(String),
    Image(String),
    /// Taken at twice the resolution it is shown at, so its size is given as half of the image's
    Screenshot {
        src: String,
        width: u32,
        height: u32,
    },
}

#[derive(Deserialize)]
struct Notebook {
    cells: Vec<Cell>,
    #[serde(default)]
    metadata: NotebookMetadata,
}

#[derive(Deserialize, Default)]
struct NotebookMetadata {
    kernelspec: Option<KernelSpec>,
}

#[derive(Deserialize)]
struct KernelSpec {
    language: String,
}

#[derive(Deserialize)]
#[serde(tag = "cell_type", rename_all = "lowercase")]
enum Cell {
    Markdown {
        source: Text,
        #[serde(default)]
        attachments: BTreeMap<String, BTreeMap<String, Text>>,
    },
    Code {
        source: Text,
        outputs: Vec<Output>,
    },
    #[serde(other)]
    Raw,
}

/// A string, which notebooks may also store as a list of lines
#[derive(Deserialize)]
#[serde(untagged)]
enum Text {
    One(String),
    Lines(Vec<String>),
}

impl Text {
    fn join(&self) -> String {
        match self {
            Self::One(text) => text.clone(),
            Self::Lines(lines) => lines.concat(),
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "output_type", rename_all = "snake_case")]
enum Output {
    Stream { name: String, text: Text },
    DisplayData { data: BTreeMap<String, Value> },
    ExecuteResult { data: BTreeMap<String, Value> },
    Error { ename: String, evalue: String },
}

/// Width and height of the PNG image `png`, from its header
fn png_size(png: &[u8]) -> Option<(u32, u32)> {
    let dimension = |at: usize| Some(u32::from_be_bytes(png.get(at..at + 4)?.try_into().ok()?));
    png.starts_with(b"\x89PNG")
        .then(|| Some((dimension(16)?, dimension(20)?)))
        .flatten()
}

fn value_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(lines) => Some(lines.iter().filter_map(Value::as_str).collect()),
        _ => None,
    }
}

fn plain_text(events: &[Event<'_>]) -> String {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Text(text) | Event::Code(text) => Some(text.as_ref()),
            Event::SoftBreak | Event::HardBreak => Some(" "),
            _ => None,
        })
        .collect()
}

fn slug(text: &str) -> String {
    text.chars()
        .filter_map(|c| match c {
            c if c.is_alphanumeric() => Some(c.to_ascii_lowercase()),
            ' ' | '-' => Some('-'),
            _ => None,
        })
        .collect()
}

fn highlight(code: &str, language: &str) -> Option<String> {
    let syntax = SYNTAXES.find_syntax_by_token(language)?;
    let mut generator = ClassedHTMLGenerator::new_with_class_style(
        syntax,
        &SYNTAXES,
        ClassStyle::SpacedPrefixed { prefix: "hl-" },
    );
    for line in LinesWithEndings::from(code) {
        generator
            .parse_html_for_line_which_includes_newline(line)
            .ok()?;
    }
    Some(generator.finalize())
}

/// Index one past the end of the element that starts at `start`
fn element_end(events: &[Event<'_>], start: usize) -> usize {
    let mut depth = 0_usize;
    for (i, event) in events.iter().enumerate().skip(start) {
        match event {
            Event::Start(_) => depth += 1,
            Event::End(_) => {
                depth -= 1;
                if depth == 0 {
                    return i + 1;
                }
            }
            _ => {}
        }
    }
    events.len()
}

/// Gives level 2 and 3 headings anchors, recording the level 2 ones in `headings`, resolves
/// images attached to the cell, highlights fenced code, and puts tables in a box that scrolls
/// sideways when they are wider than the page
fn prepare_markdown<'a>(
    events: &[Event<'a>],
    attachments: &BTreeMap<String, BTreeMap<String, Text>>,
    headings: &mut Vec<(String, String)>,
) -> Vec<Event<'a>> {
    let mut prepared = Vec::with_capacity(events.len());
    let mut i = 0;
    while i < events.len() {
        match &events[i] {
            Event::Start(Tag::Heading {
                level: level @ (HeadingLevel::H2 | HeadingLevel::H3),
                id: None,
                classes,
                attrs,
            }) => {
                let end = element_end(events, i);
                let text = plain_text(&events[i + 1..end]);
                let id = slug(&text);
                if *level == HeadingLevel::H2 {
                    headings.push((id.clone(), text));
                }
                prepared.push(Event::Start(Tag::Heading {
                    level: *level,
                    id: Some(id.into()),
                    classes: classes.clone(),
                    attrs: attrs.clone(),
                }));
                i += 1;
            }
            Event::Start(Tag::Image {
                link_type,
                dest_url,
                title,
                id,
            }) if dest_url.starts_with("attachment:") => {
                let attachment = attachments
                    .get(&dest_url["attachment:".len()..])
                    .and_then(|files| files.iter().next());
                let dest_url = attachment.map_or_else(
                    || dest_url.clone(),
                    |(mime, data)| format!("data:{mime};base64,{}", data.join().trim()).into(),
                );
                prepared.push(Event::Start(Tag::Image {
                    link_type: *link_type,
                    dest_url,
                    title: title.clone(),
                    id: id.clone(),
                }));
                i += 1;
            }
            event @ Event::Start(Tag::Table(_)) => {
                prepared.push(Event::Html("<div class=\"table-scroll\">".into()));
                prepared.push(event.clone());
                i += 1;
            }
            event @ Event::End(TagEnd::Table) => {
                prepared.push(event.clone());
                prepared.push(Event::Html("</div>".into()));
                i += 1;
            }
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(language))) => {
                let end = element_end(events, i);
                let code = plain_text(&events[i + 1..end]);
                match highlight(&code, language) {
                    Some(html) => {
                        prepared.push(Event::Html(
                            format!("<pre class=\"code\"><code>{html}</code></pre>").into(),
                        ));
                        i = end;
                    }
                    None => {
                        prepared.extend(events[i..end].iter().cloned());
                        i = end;
                    }
                }
            }
            event => {
                prepared.push(event.clone());
                i += 1;
            }
        }
    }
    prepared
}

fn to_html<'a>(events: impl IntoIterator<Item = Event<'a>>) -> String {
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, events.into_iter());
    html
}

fn screenshot_name(cell: usize) -> String {
    format!("cell-{cell}.png")
}

/// Renders an output of the cell at index `cell`, and fails on errors, because a guide must run
/// cleanly
fn render_output(
    output: &Output,
    cell: usize,
    slug: &str,
    screenshot: Option<&[u8]>,
) -> Result<Option<Rendered>, String> {
    let data = match output {
        Output::Stream { name, text } => {
            let class = if name == "stderr" {
                "nb-stderr"
            } else {
                "nb-text"
            };
            return Ok(Some(Rendered::Text(text.join(), class)));
        }
        Output::Error { ename, evalue } => {
            return Err(format!("cell {cell} raised {ename}: {evalue}"));
        }
        Output::DisplayData { data } | Output::ExecuteResult { data } => data,
    };
    if data.contains_key(WIDGET_VIEW) {
        let (width, height) = screenshot
            .and_then(png_size)
            .ok_or_else(|| format!("cell {cell} shows a widget, but it has no screenshot"))?;
        return Ok(Some(Rendered::Screenshot {
            src: format!("{}{}", guide_url(slug), screenshot_name(cell)),
            width: width / 2,
            height: height / 2,
        }));
    }
    let text = |mime| data.get(mime).and_then(value_text);
    if let Some(svg) = text("image/svg+xml") {
        return Ok(Some(Rendered::Html(svg)));
    }
    if let Some(png) = text("image/png") {
        let png = png.split_whitespace().collect::<String>();
        return Ok(Some(Rendered::Image(format!(
            "data:image/png;base64,{png}"
        ))));
    }
    if let Some(html) = text("text/html") {
        return Ok(Some(Rendered::Html(html)));
    }
    Ok(text("text/plain").map(|text| Rendered::Text(text, "nb-text")))
}

/// The notebook in `raw` with every code cell's outputs and execution count removed
fn cleared(raw: &str) -> serde_json::Result<String> {
    let mut notebook: Value = serde_json::from_str(raw)?;
    if let Some(cells) = notebook.get_mut("cells").and_then(Value::as_array_mut) {
        for cell in cells.iter_mut().filter(|cell| cell["cell_type"] == "code") {
            cell["outputs"] = Value::Array(Vec::new());
            cell["execution_count"] = Value::Null;
        }
    }
    let mut cleared = serde_json::to_string_pretty(&notebook)?;
    cleared.push('\n');
    Ok(cleared)
}

/// Collects the parts of a guide from its Markdown and code, in order
#[derive(Default)]
struct Builder {
    title: Option<String>,
    lead: String,
    headings: Vec<(String, String)>,
    blocks: Vec<Block>,
}

impl Builder {
    /// Adds Markdown, taking the title and the paragraph after it from the first heading of the
    /// guide
    fn markdown(&mut self, source: &str, attachments: &BTreeMap<String, BTreeMap<String, Text>>) {
        let mut events = Parser::new_ext(source, Options::ENABLE_TABLES).collect::<Vec<_>>();
        if self.title.is_none()
            && let Some(Event::Start(Tag::Heading {
                level: HeadingLevel::H1,
                ..
            })) = events.first()
        {
            let end = element_end(&events, 0);
            self.title = Some(plain_text(&events[1..end]));
            events.drain(..end);
            if matches!(events.first(), Some(Event::Start(Tag::Paragraph))) {
                let end = element_end(&events, 0);
                self.lead = to_html(events.drain(..end).skip(1).take(end - 2));
            }
        }
        let events = prepare_markdown(&events, attachments, &mut self.headings);
        self.blocks.push(Block::Markdown(to_html(events)));
    }

    fn build(
        self,
        slug: &str,
        download: Option<String>,
        screenshots: BTreeMap<usize, Vec<u8>>,
    ) -> Result<Guide, String> {
        Ok(Guide {
            slug: slug.to_owned(),
            title: self
                .title
                .ok_or("the guide must start with a level 1 heading, its title")?,
            lead: self.lead,
            blocks: self.blocks,
            headings: self.headings,
            download,
            screenshots,
        })
    }
}

impl Guide {
    fn from_markdown(slug: &str, source: &str) -> Result<Self, String> {
        let mut builder = Builder::default();
        builder.markdown(source, &BTreeMap::new());
        builder.build(slug, None, BTreeMap::new())
    }

    /// Runs the notebook `raw` in `jupyter`, which supplies its outputs
    fn from_notebook(slug: &str, raw: &str, jupyter: &mut Jupyter) -> Result<Self, String> {
        let download = cleared(raw).map_err(|err| err.to_string())?;
        let executed = jupyter
            .run(&format!("{slug}.ipynb"), &download)
            .map_err(|err| err.to_string())?;
        let notebook: Notebook =
            serde_json::from_str(&executed.notebook).map_err(|err| err.to_string())?;
        let language = notebook
            .metadata
            .kernelspec
            .as_ref()
            .map_or("python", |kernelspec| kernelspec.language.as_str());

        let mut builder = Builder::default();
        for (index, cell) in notebook.cells.iter().enumerate() {
            match cell {
                Cell::Markdown {
                    source,
                    attachments,
                } => builder.markdown(&source.join(), attachments),
                Cell::Code { source, outputs } => {
                    let screenshot = executed.screenshots.get(&index).map(Vec::as_slice);
                    let outputs = outputs
                        .iter()
                        .map(|output| render_output(output, index, slug, screenshot))
                        .filter_map(Result::transpose)
                        .collect::<Result<_, _>>()?;
                    let source = source.join();
                    builder.blocks.push(Block::Code {
                        highlighted: highlight(&source, language),
                        source,
                        outputs,
                    });
                }
                Cell::Raw => {}
            }
        }
        builder.build(slug, Some(download), executed.screenshots)
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn link(&self) -> GuideLink {
        GuideLink {
            slug: self.slug.clone(),
            title: self.title.clone(),
            lead: self.lead.clone(),
        }
    }

    fn download_name(&self) -> String {
        format!("{}.ipynb", self.slug)
    }

    /// The files besides the page that the guide's directory holds
    pub fn files(&self) -> Vec<(String, Vec<u8>)> {
        let dir = format!("guides/{}", self.slug);
        let download = self.download.as_ref().map(|download| {
            (
                format!("{dir}/{}", self.download_name()),
                download.clone().into_bytes(),
            )
        });
        let screenshots = self
            .screenshots
            .iter()
            .map(|(cell, png)| (format!("{dir}/{}", screenshot_name(*cell)), png.clone()));
        download.into_iter().chain(screenshots).collect()
    }
}

/// Reads the guides at `paths`, in that order. A Markdown guide is shown as it is written, and a
/// notebook is first run with `python`, which needs the `notebook` package and `cgt_py`, to fill
/// in its outputs, with the files of the notebook server in `work_dir`
///
/// # Errors
///
/// When a guide cannot be read or run, a cell raises, a guide does not start with a title, or two
/// guides have the same file name
pub fn read_guides(paths: &[PathBuf], python: &Path, work_dir: &Path) -> io::Result<Vec<Guide>> {
    let mut jupyter = None;
    let mut guides: Vec<Guide> = Vec::with_capacity(paths.len());
    for path in paths {
        let error = |err: String| io::Error::other(format!("{}: {err}", path.display()));
        let contents = fs::read_to_string(path).map_err(|err| error(err.to_string()))?;
        log(format!("Reading guide `{}`", path.display()));
        let name = path.file_name().and_then(|name| name.to_str());
        let guide = match name.and_then(|name| name.rsplit_once('.')) {
            // The slug names the directory of the page
            Some((slug, _)) if guides.iter().any(|guide| guide.slug == slug) => {
                Err(format!("another guide is also called `{slug}`"))
            }
            Some((slug, "md")) => Guide::from_markdown(slug, &contents),
            Some((slug, "ipynb")) => {
                let mut running = match jupyter.take() {
                    Some(running) => running,
                    None => {
                        log(format!("Starting Jupyter with `{}`", python.display()));
                        Jupyter::start(python, work_dir)?
                    }
                };
                let guide = Guide::from_notebook(slug, &contents, &mut running);
                jupyter = Some(running);
                guide
            }
            _ => Err("a guide must be a Markdown file or a notebook".to_owned()),
        };
        guides.push(guide.map_err(error)?);
    }
    Ok(guides)
}

pub const GUIDES_URL: &str = "/guides/";

fn guide_url(slug: &str) -> String {
    format!("{GUIDES_URL}{slug}/")
}

fn code_view(source: &str, highlighted: Option<&str>) -> impl IntoView + use<> {
    highlighted.map_or_else(
        || {
            view! {
                <pre class="code">
                    <code>{source.to_owned()}</code>
                </pre>
            }
            .into_any()
        },
        |html| {
            view! {
                <pre class="code">
                    <code inner_html=html.to_owned()></code>
                </pre>
            }
            .into_any()
        },
    )
}

fn output_view(output: &Rendered) -> AnyView {
    match output {
        Rendered::Text(text, class) => view! { <pre class=*class>{text.clone()}</pre> }.into_any(),
        Rendered::Html(html) => {
            view! { <div class="nb-html" inner_html=html.clone()></div> }.into_any()
        }
        Rendered::Image(src) => view! { <img src=src.clone() alt="" /> }.into_any(),
        Rendered::Screenshot { src, width, height } => view! {
            <img
                src=src.clone()
                width=*width
                height=*height
                alt="Screenshot of the widget shown by this cell"
            />
        }
        .into_any(),
    }
}

fn contents_view(current: &Guide, guides: &[GuideLink]) -> impl IntoView + use<> {
    let items = guides
        .iter()
        .map(|guide| {
            let active = guide.slug == current.slug;
            let headings = active.then(|| {
                let headings = current
                    .headings
                    .iter()
                    .map(|(id, text)| {
                        view! {
                            <li>
                                <a href=format!("#{id}")>{text.clone()}</a>
                            </li>
                        }
                    })
                    .collect_view();
                view! { <ul>{headings}</ul> }
            });
            view! {
                <li class:active=active>
                    <a href=guide_url(&guide.slug) aria-current=active.then_some("page")>
                        {guide.title.clone()}
                    </a>
                    {headings}
                </li>
            }
        })
        .collect_view();
    view! {
        <nav class="section-nav">
            <details open>
                <summary>"Guides"</summary>
                <ul class="list-nested">{items}</ul>
            </details>
        </nav>
    }
}

/// The page of `guide`, with a sidebar linking to every guide in `guides`
pub fn page(guide: &Guide, guides: &[GuideLink]) -> impl IntoView + use<> {
    let blocks = guide
        .blocks
        .iter()
        .map(|block| match block {
            Block::Markdown(html) => {
                view! { <div class="nb-markdown" inner_html=html.clone()></div> }.into_any()
            }
            Block::Code {
                source,
                highlighted,
                outputs,
            } => {
                let outputs = (!outputs.is_empty()).then(|| {
                    let outputs = outputs.iter().map(output_view).collect_view();
                    view! { <div class="nb-output">{outputs}</div> }
                });
                view! { <div class="nb-code">{code_view(source, highlighted.as_deref())} {outputs}</div> }
                .into_any()
            }
        })
        .collect_view();
    view! {
        <div class="docs-layout container">
            <aside class="docs-sidebar">{contents_view(guide, guides)}</aside>
            <div class="docs-content">
                <details class="toc-mobile">
                    <summary>"Contents"</summary>
                    {contents_view(guide, guides)}
                </details>
                <h1>{guide.title.clone()}</h1>
                <p class="lead" inner_html=guide.lead.clone()></p>
                {guide
                    .download
                    .as_ref()
                    .map(|_| {
                        view! {
                            <p>
                                <a
                                    class="btn btn-outline btn-sm"
                                    href=format!(
                                        "{}{}",
                                        guide_url(&guide.slug),
                                        guide.download_name(),
                                    )
                                    download=guide.download_name()
                                >
                                    "Download notebook"
                                </a>
                            </p>
                        }
                    })}
                {blocks}
                <ScrollSpy />
            </div>
        </div>
    }
}

/// Links to every guide in `guides`, with their summaries
pub fn index_page(guides: &[GuideLink]) -> impl IntoView + use<> {
    let items = guides
        .iter()
        .map(|guide| {
            view! {
                <li>
                    <a href=guide_url(&guide.slug)>{guide.title.clone()}</a>
                    <p inner_html=guide.lead.clone()></p>
                </li>
            }
        })
        .collect_view();
    view! {
        <div class="container">
            <div class="docs-content">
                <h1>"Guides"</h1>
                <p class="lead">
                    "Walkthroughs of cgt-tools."
                </p>
                <ul class="guide-list">{items}</ul>
            </div>
        </div>
    }
}
