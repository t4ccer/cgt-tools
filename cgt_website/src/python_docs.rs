use leptos::prelude::*;
use pulldown_cmark::{Event, LinkType, Parser, Tag, TagEnd};
use semver::Version;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

/// Signatures longer than this many characters put every parameter on its own line
const SIGNATURE_WIDTH: usize = 70;

/// The API of one version of `cgt_py`
pub struct PythonApi {
    /// Name of the version, which is also the directory its page is written to
    pub version: String,
    pub package: Package,
}

impl PythonApi {
    fn release(&self) -> Option<Version> {
        Version::parse(self.version.strip_prefix('v')?).ok()
    }
}

/// Sorts versions in the order the version switcher lists them, unreleased versions first and then
/// releases from the newest, and returns the newest stable release
pub fn sort_versions(apis: &mut [PythonApi]) -> Option<String> {
    apis.sort_by_cached_key(|api| {
        let release = api.release();
        (release.is_some(), std::cmp::Reverse(release))
    });
    apis.iter()
        .find(|api| api.release().is_some_and(|release| release.pre.is_empty()))
        .map(|api| api.version.clone())
}

pub fn docs_url(version: &str) -> String {
    format!("/python/docs/{version}/")
}

// The types below mirror `pyo3_stub_gen::docgen::ir`, which is what `api_reference.json` is
// serialized from. Depending on pyo3-stub-gen instead would pull pyo3, and with it a Python
// interpreter, into the website build

#[derive(Deserialize)]
pub struct Package {
    modules: BTreeMap<String, Module>,
}

#[derive(Deserialize)]
struct Module {
    name: String,
    doc: String,
    items: Vec<Item>,
}

#[derive(Deserialize)]
#[serde(tag = "kind")]
enum Item {
    Class(Class),
    Function(Function),
    TypeAlias(TypeAlias),
    Variable(Variable),
    /// Submodules have their own entries in [`Package::modules`]
    #[serde(other)]
    Module,
}

#[derive(Deserialize)]
struct Class {
    name: String,
    doc: String,
    bases: Vec<TypeExpr>,
    methods: Vec<Function>,
    attributes: Vec<Attribute>,
    deprecated: Option<Deprecated>,
}

#[derive(Deserialize)]
struct Function {
    name: String,
    doc: String,
    signatures: Vec<Signature>,
    is_async: bool,
    deprecated: Option<Deprecated>,
}

#[derive(Deserialize)]
struct Signature {
    parameters: Vec<Parameter>,
    return_type: Option<TypeExpr>,
}

#[derive(Deserialize)]
struct Parameter {
    name: String,
    #[serde(rename = "type_")]
    ty: TypeExpr,
    default: Option<DefaultValue>,
}

#[derive(Deserialize)]
#[serde(tag = "kind")]
enum DefaultValue {
    Simple { value: String },
    Expression { display: String },
}

impl DefaultValue {
    fn text(&self) -> &str {
        match self {
            Self::Simple { value } => value,
            Self::Expression { display } => display,
        }
    }
}

#[derive(Deserialize)]
struct TypeAlias {
    name: String,
    doc: String,
    definition: TypeExpr,
}

#[derive(Deserialize)]
struct Variable {
    name: String,
    doc: String,
    #[serde(rename = "type_")]
    ty: Option<TypeExpr>,
}

#[derive(Deserialize)]
struct Attribute {
    name: String,
    doc: String,
    #[serde(rename = "type_")]
    ty: Option<TypeExpr>,
    #[serde(default)]
    is_property: bool,
    #[serde(default)]
    is_readonly: bool,
    #[serde(default)]
    deprecated: Option<Deprecated>,
}

#[derive(Deserialize)]
struct TypeExpr {
    /// The whole expression, such as `Optional[Sequence[int]]`
    display: String,
    link_target: Option<LinkTarget>,
    /// Arguments of a generic type, or the members of a union
    children: Vec<Self>,
}

#[derive(Deserialize)]
struct LinkTarget {
    fqn: String,
}

#[derive(Deserialize)]
struct Deprecated {
    since: Option<String>,
    note: Option<String>,
}

/// The items of one kind from every module, each with the module it is in
struct Section<'a> {
    id: &'static str,
    title: &'static str,
    items: Vec<(&'a Module, &'a Item)>,
}

impl Class {
    /// Shown as the signature of the class rather than as one of its members
    fn constructor(&self) -> Option<&Function> {
        self.methods
            .iter()
            .find(|method| matches!(method.name.as_str(), "__new__" | "__init__"))
    }
}

impl Item {
    fn name(&self) -> &str {
        match self {
            Self::Class(Class { name, .. })
            | Self::Function(Function { name, .. })
            | Self::TypeAlias(TypeAlias { name, .. })
            | Self::Variable(Variable { name, .. }) => name,
            Self::Module => "",
        }
    }
}

fn sections(package: &Package) -> Vec<Section<'_>> {
    let mut sections = [
        ("classes", "Classes"),
        ("functions", "Functions"),
        ("type-aliases", "Type Aliases"),
        ("variables", "Variables"),
    ]
    .map(|(id, title)| Section {
        id,
        title,
        items: Vec::new(),
    });
    for module in package.modules.values() {
        for item in &module.items {
            let section = match item {
                Item::Class(_) => 0,
                Item::Function(_) => 1,
                Item::TypeAlias(_) => 2,
                Item::Variable(_) => 3,
                Item::Module => continue,
            };
            sections[section].items.push((module, item));
        }
    }
    sections
        .into_iter()
        .filter(|section| !section.items.is_empty())
        .collect()
}

/// Fully qualified names of the items and members that have an anchor on the page
fn anchors(package: &Package) -> BTreeSet<String> {
    let mut anchors = BTreeSet::new();
    for module in package.modules.values() {
        for item in &module.items {
            let fqn = format!("{}.{}", module.name, item.name());
            if let Item::Class(class) = item {
                let constructor = class.constructor().map(|constructor| &constructor.name);
                let attributes = class.attributes.iter().map(|attribute| &attribute.name);
                let methods = class
                    .methods
                    .iter()
                    .map(|method| &method.name)
                    .filter(|name| Some(*name) != constructor);
                anchors.extend(
                    attributes
                        .chain(methods)
                        .map(|name| format!("{fqn}.{name}")),
                );
            }
            anchors.insert(fqn);
        }
    }
    anchors
}

/// Resolves names used in one module to anchors on the page
#[derive(Clone, Copy)]
struct Links<'a> {
    module: &'a str,
    anchors: &'a BTreeSet<String>,
}

impl Links<'_> {
    fn resolve(self, name: &str) -> Option<String> {
        [format!("{}.{name}", self.module), name.to_owned()]
            .into_iter()
            .find(|fqn| self.anchors.contains(fqn))
            .map(|fqn| format!("#{fqn}"))
    }

    /// pyo3-stub-gen leaves `link_target` empty even for classes of the module itself, so names
    /// of items on the page are linked too
    fn href(self, ty: &TypeExpr, name: &str) -> Option<String> {
        ty.link_target
            .as_ref()
            .map(|target| format!("#{}", target.fqn))
            .or_else(|| self.resolve(name))
    }

    /// Resolves code in a docstring that names an item or a member, such as `Snort`,
    /// `Graph.directed` or `CanonicalForm.cool()`. Bare member names are left alone, because
    /// docstrings use them for parameters too
    fn code_href(self, code: &str) -> Option<String> {
        self.resolve(code.strip_suffix("()").unwrap_or(code))
    }
}

fn is_union(display: &str) -> bool {
    let mut depth = 0_usize;
    display.chars().any(|c| {
        match c {
            '[' => depth += 1,
            ']' => depth = depth.saturating_sub(1),
            '|' if depth == 0 => return true,
            _ => {}
        }
        false
    })
}

fn type_view(ty: &TypeExpr, links: Links<'_>) -> AnyView {
    if ty.children.is_empty() {
        return name_view(&ty.display, links.href(ty, &ty.display));
    }
    let union = is_union(&ty.display);
    let separator = if union { " | " } else { ", " };
    let children = ty
        .children
        .iter()
        .enumerate()
        .map(|(i, child)| {
            view! {
                {(i > 0).then_some(separator)}
                {type_view(child, links)}
            }
        })
        .collect_view();
    if union {
        return children.into_any();
    }
    let base = ty
        .display
        .split_once('[')
        .map_or(ty.display.as_str(), |(base, _)| base);
    view! {
        {name_view(base, links.href(ty, base))}
        "["
        {children}
        "]"
    }
    .into_any()
}

fn name_view(name: &str, href: Option<String>) -> AnyView {
    let name = name.to_owned();
    match href {
        Some(href) => view! { <a href=href>{name}</a> }.into_any(),
        None => name.into_any(),
    }
}

fn parameter_text(parameter: &Parameter) -> String {
    let default = parameter
        .default
        .as_ref()
        .map(|default| format!(" = {}", default.text()))
        .unwrap_or_default();
    format!("{}: {}{default}", parameter.name, parameter.ty.display)
}

/// Renders every overload on its own line as `keyword name(parameters) -> return`, or a single
/// line without the parentheses when there are no signatures
fn signatures_view(
    keyword: Option<&'static str>,
    name: &str,
    signatures: &[Signature],
    returns: bool,
    links: Links<'_>,
) -> AnyView {
    let head = || {
        view! {
            {keyword
                .map(|keyword| {
                    view! {
                        <span class="sig-keyword">{keyword}</span>
                        " "
                    }
                })}
            <span class="sig-name">{name.to_owned()}</span>
        }
    };
    if signatures.is_empty() {
        return view! { <code>{head()}</code> }.into_any();
    }
    let head_width = keyword.map_or(0, |keyword| keyword.len() + 1) + name.len();
    signatures
        .iter()
        .enumerate()
        .map(|(i, signature)| {
            view! {
                {(i > 0).then_some("\n")}
                <code>
                    {head()}
                    {parameters_view(
                        &signature.parameters,
                        signature.return_type.as_ref().filter(|_| returns),
                        head_width,
                        links,
                    )}
                </code>
            }
        })
        .collect_view()
        .into_any()
}

fn parameters_view(
    parameters: &[Parameter],
    return_type: Option<&TypeExpr>,
    head_width: usize,
    links: Links<'_>,
) -> AnyView {
    let returns = return_type.map(|ty| {
        view! {
            " -> "
            {type_view(ty, links)}
        }
    });
    if parameters.is_empty() {
        return view! {
            "()"
            {returns}
        }
        .into_any();
    }
    let width = head_width
        + parameters
            .iter()
            .map(|parameter| parameter_text(parameter).len() + 2)
            .sum::<usize>()
        + return_type.map_or(0, |ty| ty.display.len() + 4);
    let (open, separator, close) = if width > SIGNATURE_WIDTH {
        ("(\n    ", ",\n    ", ",\n)")
    } else {
        ("(", ", ", ")")
    };
    let parameters = parameters
        .iter()
        .enumerate()
        .map(|(i, parameter)| {
            let default = parameter
                .default
                .as_ref()
                .map(|default| format!(" = {}", default.text()));
            view! {
                {(i > 0).then_some(separator)}
                {parameter.name.clone()}
                ": "
                {type_view(&parameter.ty, links)}
                {default}
            }
        })
        .collect_view();
    view! {
        {open}
        {parameters}
        {close}
        {returns}
    }
    .into_any()
}

fn docstring_view(doc: &str, links: Links<'_>) -> Option<impl IntoView + use<>> {
    if doc.trim().is_empty() {
        return None;
    }
    let mut in_link = false;
    let events = Parser::new(doc).flat_map(|event| {
        let href = match &event {
            Event::Start(Tag::Link { .. }) => {
                in_link = true;
                None
            }
            Event::End(TagEnd::Link) => {
                in_link = false;
                None
            }
            Event::Code(code) if !in_link => links.code_href(code),
            _ => None,
        };
        match href {
            Some(href) => vec![
                Event::Start(Tag::Link {
                    link_type: LinkType::Inline,
                    dest_url: href.into(),
                    title: "".into(),
                    id: "".into(),
                }),
                event,
                Event::End(TagEnd::Link),
            ],
            None => vec![event],
        }
    });
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, events);
    Some(view! { <div class="docstring" inner_html=html></div> })
}

fn deprecated_view(deprecated: Option<&Deprecated>) -> Option<impl IntoView + use<>> {
    let deprecated = deprecated?;
    let since = deprecated
        .since
        .as_ref()
        .map(|since| format!(" since {since}"));
    let note = deprecated.note.as_ref().map(|note| format!(": {note}"));
    Some(view! {
        <div class="callout callout-caution">
            <strong>"Deprecated"</strong>
            {since}
            {note}
        </div>
    })
}

fn heading_view(fqn: &str, name: &str) -> impl IntoView + use<> {
    view! {
        <h3 id=fqn
            .to_owned()>
            {name.to_owned()} <a class="anchor" href=format!("#{fqn}") aria-hidden="true">
                "#"
            </a>
        </h3>
    }
}

fn member_view(
    id: String,
    signature: impl IntoView + 'static,
    tag: Option<&'static str>,
    deprecated: Option<&Deprecated>,
    doc: &str,
    links: Links<'_>,
) -> AnyView {
    view! {
        <div class="member" id=id>
            <div class="member-signature">
                <pre>{signature}</pre>
                {tag.map(|tag| view! { <span class="tag">{tag}</span> })}
            </div>
            {deprecated_view(deprecated)}
            {docstring_view(doc, links)}
        </div>
    }
    .into_any()
}

fn class_view(class: &Class, fqn: &str, links: Links<'_>) -> impl IntoView + use<> {
    let constructor = class.constructor();
    let signature = signatures_view(
        Some("class"),
        &class.name,
        constructor.map_or(&[], |constructor| constructor.signatures.as_slice()),
        false,
        links,
    );
    let bases = (!class.bases.is_empty()).then(|| {
        let bases = class
            .bases
            .iter()
            .enumerate()
            .map(|(i, base)| {
                view! {
                    {(i > 0).then_some(", ")}
                    <code>{type_view(base, links)}</code>
                }
            })
            .collect_view();
        view! { <p>"Bases: " {bases}</p> }
    });
    let attributes = class.attributes.iter().map(|attribute| {
        let ty = attribute.ty.as_ref().map(|ty| {
            view! {
                ": "
                {type_view(ty, links)}
            }
        });
        let signature = view! {
            <code>
                {attribute
                    .is_property
                    .then(|| {
                        view! {
                            <span class="sig-keyword">"property"</span>
                            " "
                        }
                    })} <span class="sig-name">{attribute.name.clone()}</span> {ty}
            </code>
        };
        member_view(
            format!("{fqn}.{}", attribute.name),
            signature,
            attribute.is_readonly.then_some("read-only"),
            attribute.deprecated.as_ref(),
            &attribute.doc,
            links,
        )
    });
    let methods = class
        .methods
        .iter()
        .filter(|method| constructor.is_none_or(|constructor| constructor.name != method.name))
        .map(|method| {
            member_view(
                format!("{fqn}.{}", method.name),
                signatures_view(
                    method.is_async.then_some("async"),
                    &method.name,
                    &method.signatures,
                    true,
                    links,
                ),
                None,
                method.deprecated.as_ref(),
                &method.doc,
                links,
            )
        });
    let members = attributes.chain(methods).collect::<Vec<_>>();
    view! {
        {heading_view(fqn, &class.name)}
        <pre class="signature">{signature}</pre>
        {bases}
        {deprecated_view(class.deprecated.as_ref())}
        {docstring_view(&class.doc, links)}
        {(!members.is_empty()).then(move || view! { <div class="members">{members}</div> })}
    }
}

fn function_view(function: &Function, fqn: &str, links: Links<'_>) -> impl IntoView + use<> {
    view! {
        {heading_view(fqn, &function.name)}
        <pre class="signature">
            {signatures_view(
                function.is_async.then_some("async"),
                &function.name,
                &function.signatures,
                true,
                links,
            )}
        </pre>
        {deprecated_view(function.deprecated.as_ref())}
        {docstring_view(&function.doc, links)}
    }
}

fn type_alias_view(alias: &TypeAlias, fqn: &str, links: Links<'_>) -> impl IntoView + use<> {
    view! {
        {heading_view(fqn, &alias.name)}
        <pre class="signature">
            <code>
                <span class="sig-keyword">"type"</span>
                " "
                <span class="sig-name">{alias.name.clone()}</span>
                " = "
                {type_view(&alias.definition, links)}
            </code>
        </pre>
        {docstring_view(&alias.doc, links)}
    }
}

fn variable_view(variable: &Variable, fqn: &str, links: Links<'_>) -> impl IntoView + use<> {
    let ty = variable.ty.as_ref().map(|ty| {
        view! {
            ": "
            {type_view(ty, links)}
        }
    });
    view! {
        {heading_view(fqn, &variable.name)}
        <pre class="signature">
            <code>
                <span class="sig-name">{variable.name.clone()}</span>
                {ty}
            </code>
        </pre>
        {docstring_view(&variable.doc, links)}
    }
}

fn item_view(module: &Module, item: &Item, anchors: &BTreeSet<String>) -> AnyView {
    let links = Links {
        module: &module.name,
        anchors,
    };
    let fqn = format!("{}.{}", module.name, item.name());
    match item {
        Item::Class(class) => class_view(class, &fqn, links).into_any(),
        Item::Function(function) => function_view(function, &fqn, links).into_any(),
        Item::TypeAlias(alias) => type_alias_view(alias, &fqn, links).into_any(),
        Item::Variable(variable) => variable_view(variable, &fqn, links).into_any(),
        Item::Module => ().into_any(),
    }
}

fn contents_view(sections: &[Section<'_>]) -> impl IntoView + use<> {
    let sections = sections
        .iter()
        .map(|section| {
            let links = section
                .items
                .iter()
                .map(|(module, item)| {
                    let href = format!("#{}.{}", module.name, item.name());
                    view! {
                        <li>
                            <a href=href>{item.name().to_owned()}</a>
                        </li>
                    }
                })
                .collect_view();
            view! {
                <details open>
                    <summary>{section.title}</summary>
                    <ul class="list-nested">{links}</ul>
                </details>
            }
        })
        .collect_view();
    view! { <nav class="section-nav">{sections}</nav> }
}

fn version_switcher(
    current: &str,
    versions: &[String],
    latest: Option<&str>,
) -> impl IntoView + use<> {
    let items = versions
        .iter()
        .map(|version| {
            let active = version == current;
            let latest = (Some(version.as_str()) == latest)
                .then(|| view! { <span class="tag">"latest"</span> });
            view! {
                <li>
                    <a
                        class="dropdown-item"
                        class:active=active
                        aria-current=active.then_some("page")
                        href=docs_url(version)
                    >
                        {version.clone()}
                        {latest}
                    </a>
                </li>
            }
        })
        .collect_view();
    view! {
        <details class="version-switcher">
            <summary>
                <span>"Version " <span class="version-name">{current.to_owned()}</span></span>
                <SelectorIcon />
            </summary>
            <ul class="dropdown-menu">{items}</ul>
        </details>
    }
}

/// The reference of one version of the Python API, with a switcher between `versions`
pub fn page(api: &PythonApi, versions: &[String], latest: Option<&str>) -> impl IntoView + use<> {
    let sections = sections(&api.package);
    let anchors = anchors(&api.package);
    let notice = (latest != Some(api.version.as_str())).then(|| {
        let this = if api.release().is_some() {
            format!("This is the documentation of {}.", api.version)
        } else {
            "This is the documentation of unreleased changes.".to_owned()
        };
        let latest = latest.map(|latest| {
            view! {
                " The latest release is "
                <a href=docs_url(latest)>{latest.to_owned()}</a>
                "."
            }
        });
        view! { <div class="callout callout-note">{this} {latest}</div> }
    });
    let module_docs = api
        .package
        .modules
        .values()
        .filter_map(|module| {
            let links = Links {
                module: &module.name,
                anchors: &anchors,
            };
            docstring_view(&module.doc, links)
        })
        .collect_view();
    let content = sections
        .iter()
        .map(|section| {
            let items = section
                .items
                .iter()
                .map(|(module, item)| item_view(module, item, &anchors))
                .collect_view();
            view! {
                <h2 id=section.id>{section.title}</h2>
                {items}
            }
        })
        .collect_view();
    view! {
        <div class="docs-layout container">
            <aside class="docs-sidebar">
                {version_switcher(&api.version, versions, latest)} <hr class="section-divider" />
                {contents_view(&sections)}
            </aside>
            <div class="docs-content">
                <details class="toc-mobile">
                    <summary>"Contents"</summary>
                    {contents_view(&sections)}
                </details>
                <h1>"Python API"</h1>
                {notice}
                {module_docs}
                {content}
            </div>
        </div>
    }
}

#[component]
fn SelectorIcon() -> impl IntoView {
    view! {
        <svg
            xmlns="http://www.w3.org/2000/svg"
            width="16"
            height="16"
            viewBox="0 0 24 24"
            stroke-width="2"
            stroke="currentColor"
            fill="none"
            stroke-linecap="round"
            stroke-linejoin="round"
        >
            <path d="M8 9l4 -4l4 4" />
            <path d="M16 15l-4 4l-4 -4" />
        </svg>
    }
}
