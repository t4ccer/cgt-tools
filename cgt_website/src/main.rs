use cgt_website::{Config, Package, PythonApi};
use std::{
    fs::{self, File},
    io::{self, BufReader},
    path::{Path, PathBuf},
};

const ASSETS: &[(&str, &[u8])] = &[
    ("CNAME", include_bytes!("../static/CNAME")),
    (".nojekyll", include_bytes!("../static/.nojekyll")),
    ("style.css", include_bytes!("../static/style.css")),
    ("fonts/OFL.txt", include_bytes!("../static/fonts/OFL.txt")),
    (
        "fonts/jost-400.woff2",
        include_bytes!("../static/fonts/jost-400.woff2"),
    ),
    (
        "fonts/jost-400-italic.woff2",
        include_bytes!("../static/fonts/jost-400-italic.woff2"),
    ),
    (
        "fonts/jost-500.woff2",
        include_bytes!("../static/fonts/jost-500.woff2"),
    ),
    (
        "fonts/jost-700.woff2",
        include_bytes!("../static/fonts/jost-700.woff2"),
    ),
];

fn write(path: &Path, contents: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)
}

fn usage() -> ! {
    eprintln!(
        "usage: cgt_website <output directory> --config <website.toml> [--python <python>] [<version>=<api_reference.json>]..."
    );
    std::process::exit(2);
}

/// Reads an argument of the form `<version>=<path>`, where `path` is the `api_reference.json` that
/// pyo3-stub-gen wrote for `version`
fn read_python_api(arg: &str) -> io::Result<PythonApi> {
    let Some((version, path)) = arg.split_once('=') else {
        usage();
    };
    let with_path = |err: &dyn std::fmt::Display| format!("{path}: {err}");
    let file = File::open(path).map_err(|err| io::Error::new(err.kind(), with_path(&err)))?;
    let package: Package = serde_json::from_reader(BufReader::new(file))
        .map_err(|err| io::Error::other(with_path(&err)))?;
    cgt_website::log(format!("Read the Python API of {version} from `{path}`"));
    Ok(PythonApi {
        version: version.to_owned(),
        package,
    })
}

fn main() -> io::Result<()> {
    let mut args = std::env::args();
    let Some(site) = args.nth(1).map(PathBuf::from) else {
        usage();
    };
    cgt_website::log(format!("Building the website in `{}`", site.display()));
    let mut config = None;
    let mut python = PathBuf::from("python3");
    let mut python_apis = Vec::new();
    while let Some(arg) = args.next() {
        if arg == "--config" {
            config = Some(args.next().map_or_else(|| usage(), PathBuf::from));
        } else if arg == "--python" {
            python = args.next().map_or_else(|| usage(), PathBuf::from);
        } else {
            python_apis.push(read_python_api(&arg)?);
        }
    }
    let Some(config) = config else {
        usage();
    };
    let config = Config::read(&config)?;
    let known: Vec<&str> = cgt_website::GAMES
        .iter()
        .map(cgt_website::GamePage::name)
        .collect();
    if let Some(name) = config
        .play
        .keys()
        .find(|name| !known.contains(&name.as_str()))
    {
        return Err(io::Error::other(format!(
            "the configuration has models of {name}, but the site only has pages for {}",
            known.join(", ")
        )));
    }
    let mut play = Vec::new();
    for game in cgt_website::GAMES {
        let models = config
            .play
            .get(game.name())
            .map_or(&[][..], |models| &models.0);
        play.push((game, cgt_website::read_models(game, models)?));
    }
    let guides = cgt_website::read_guides(&config.guides, &python)?;
    cgt_website::log("Writing the pages");
    for (path, contents) in ASSETS {
        write(&site.join(path), contents)?;
    }
    for (path, contents) in guides.iter().flat_map(cgt_website::Guide::files) {
        write(&site.join(path), &contents)?;
    }
    for (path, contents) in play
        .iter()
        .flat_map(|(_, models)| models)
        .filter_map(cgt_website::AiModel::file)
    {
        write(&site.join(path), contents)?;
    }
    let play = play
        .into_iter()
        .map(|(game, models)| {
            let models = models
                .into_iter()
                .map(|model| (model.name, model.url))
                .collect();
            (game, models)
        })
        .collect();
    for (path, html) in cgt_website::pages(python_apis, guides, play) {
        write(&site.join(path), html.as_bytes())?;
    }
    println!("Created website in `{}`", site.display());
    Ok(())
}
