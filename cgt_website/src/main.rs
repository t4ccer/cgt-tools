use cgt_website::{Package, PythonApi};
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
    eprintln!("usage: cgt_website <output directory> [<version>=<api_reference.json>]...");
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
    let python_apis = args
        .map(|arg| read_python_api(&arg))
        .collect::<io::Result<Vec<_>>>()?;
    for (path, contents) in ASSETS {
        write(&site.join(path), contents)?;
    }
    for (path, html) in cgt_website::pages(python_apis) {
        write(&site.join(path), html.as_bytes())?;
    }
    println!("Created website in `{}`", site.display());
    Ok(())
}
