use crate::{
    chrome::Chrome,
    log,
    process::{Process, TempDir},
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use std::{
    collections::BTreeMap, fs, io, net::TcpListener, path::Path, process::Command, time::Duration,
};

pub const WIDGET_VIEW: &str = "application/vnd.jupyter.widget-view+json";

/// Size of the browser window in CSS pixels, tall enough for any widget to fit on screen whole
const WINDOW: (u32, u32) = (1000, 2000);

const RUNNER: &str = include_str!("jupyter.js");

/// Jupyter settings that keep every cell in the page, so that all of them can be measured, and
/// that stop a news prompt from covering the notebook
const SETTINGS: &[(&str, &str)] = &[
    (
        "@jupyterlab/notebook-extension/tracker.jupyterlab-settings",
        r#"{ "windowingMode": "none" }"#,
    ),
    (
        "@jupyterlab/apputils-extension/notification.jupyterlab-settings",
        r#"{ "fetchNews": "false", "checkForUpdates": false }"#,
    ),
];

/// A notebook after it has been run
pub struct Executed {
    /// The notebook as Jupyter saved it, with the outputs of every cell
    pub notebook: String,
    /// Screenshots of widget outputs at twice the size they appear at, by the index of their cell
    pub screenshots: BTreeMap<usize, Vec<u8>>,
}

#[derive(Deserialize)]
struct Rect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

fn other(err: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> io::Error {
    io::Error::other(err)
}

/// A Jupyter server and a headless browser to drive it. Notebooks run in the browser as they do
/// for a person, because widgets are drawn, and pass edits back to the kernel, only in a front end
pub struct Jupyter {
    chrome: Chrome,
    port: u16,
    _server: Process,
    /// Dropped last, after both processes that use it have stopped
    dir: TempDir,
}

impl Jupyter {
    /// Starts a notebook server with `python`, which also runs the kernels, with the files of the
    /// server and the browser in `dir`, which is emptied first and removed at the end
    ///
    /// # Errors
    ///
    /// When the server or the browser cannot be started
    pub fn start(python: &Path, dir: &Path) -> io::Result<Self> {
        // A build that panics or is interrupted leaves its files behind, because the release
        // profile aborts on a panic and a signal ends the process without running any `Drop`
        let _ = fs::remove_dir_all(dir);
        fs::create_dir_all(dir)?;
        let dir = TempDir(dir.to_path_buf());
        let config = dir.0.join("config");
        for (path, contents) in SETTINGS {
            let path = config.join("lab/user-settings").join(path);
            fs::create_dir_all(path.parent().unwrap_or(&config))?;
            fs::write(path, contents)?;
        }
        fs::create_dir_all(dir.0.join("notebooks"))?;

        let port = TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();
        let mut server = Process::spawn(
            Command::new(python)
                .args([
                    "-m",
                    "notebook",
                    "--no-browser",
                    "--expose-app-in-browser",
                    "--ServerApp.ip=127.0.0.1",
                    "--ServerApp.token=",
                    "--ServerApp.password=",
                    "--ServerApp.port_retries=0",
                    // Otherwise an interrupt makes the server ask on its standard input, which is
                    // closed, whether to shut down, and it goes on running
                    "--ServerApp.answer_yes=True",
                ])
                .arg(format!("--ServerApp.port={port}"))
                .arg(format!(
                    "--ServerApp.root_dir={}",
                    dir.0.join("notebooks").display()
                ))
                // Settings, kernels and open documents of the person building the site stay out
                .env("JUPYTER_CONFIG_DIR", &config)
                .env("JUPYTER_DATA_DIR", dir.0.join("data"))
                .env("JUPYTER_RUNTIME_DIR", dir.0.join("runtime")),
        )?;
        let address = format!("127.0.0.1:{port}");
        server.wait_for_line("Jupyter", Duration::from_mins(2), move |line| {
            line.contains(&address).then(String::new)
        })?;
        log(format!("Jupyter is listening on port {port}"));

        let mut chrome = Chrome::start(&dir.0.join("chrome"), WINDOW.0, WINDOW.1)?;
        chrome.add_script(RUNNER)?;

        Ok(Self {
            chrome,
            port,
            _server: server,
            dir,
        })
    }

    /// Runs every cell of the notebook `contents` under the file name `name`
    ///
    /// # Errors
    ///
    /// When the notebook cannot be opened or saved, a cell does not finish in time, or a widget
    /// cannot be displayed or photographed
    pub fn run(&mut self, name: &str, contents: &str) -> io::Result<Executed> {
        let path = self.dir.0.join("notebooks").join(name);
        fs::write(&path, contents)?;
        self.chrome
            .navigate(&format!("http://127.0.0.1:{}/notebooks/{name}", self.port))?;

        self.call::<()>("cgtRunner.open()")?;
        log(format!("{name}: opened, and its kernel started"));
        let cells: usize = self.call("cgtRunner.runAll()")?;
        log(format!("{name}: ran {cells} code cells"));
        self.call::<()>("cgtRunner.save()")?;
        let notebook = fs::read_to_string(&path)?;

        let mut screenshots = BTreeMap::new();
        let saved: Value = serde_json::from_str(&notebook).map_err(other)?;
        let cells = saved["cells"].as_array().map_or(&[][..], Vec::as_slice);
        for (index, cell) in cells.iter().enumerate() {
            let outputs = cell["outputs"].as_array().map_or(&[][..], Vec::as_slice);
            if outputs
                .iter()
                .any(|output| output["data"].get(WIDGET_VIEW).is_some())
            {
                let rect: Rect = self.call(&format!("cgtRunner.measure({index})"))?;
                let png = self
                    .chrome
                    .screenshot(rect.x, rect.y, rect.width, rect.height)?;
                screenshots.insert(index, png);
            }
        }
        log(format!("{name}: took {} screenshots", screenshots.len()));

        Ok(Executed {
            notebook,
            screenshots,
        })
    }

    /// Evaluates `expression`, a call on `cgtRunner`, and parses the JSON that its promise
    /// resolves to
    fn call<T: DeserializeOwned>(&mut self, expression: &str) -> io::Result<T> {
        match self.chrome.evaluate(expression)? {
            Value::String(json) => serde_json::from_str(&json).map_err(other),
            value => Err(other(format!(
                "`{expression}` gave {value} instead of a JSON string"
            ))),
        }
    }
}
