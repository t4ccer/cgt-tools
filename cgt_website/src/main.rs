use std::{
    fs, io,
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

fn main() -> io::Result<()> {
    let Some(site) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: cgt_website <output directory>");
        std::process::exit(2);
    };
    for (path, contents) in ASSETS {
        write(&site.join(path), contents)?;
    }
    for (path, html) in cgt_website::pages() {
        write(&site.join(path), html.as_bytes())?;
    }
    println!("Created website in `{}`", site.display());
    Ok(())
}
