use crate::{log, process::Process};
use base64::{Engine, prelude::BASE64_STANDARD};
use serde_json::{Value, json};
use std::{
    env,
    io::{self, BufRead, BufReader, Read, Write},
    net::TcpStream,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant},
};

/// Names the browser goes by on `PATH`, tried in order when `CHROME` is not set
const NAMES: &[&str] = &[
    "chromium",
    "chromium-browser",
    "google-chrome-stable",
    "google-chrome",
    "chrome",
];

/// Chrome does not check that the key of a WebSocket handshake is random, so this is the example
/// key of RFC 6455
const WEBSOCKET_KEY: &str = "dGhlIHNhbXBsZSBub25jZQ==";

fn other(err: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> io::Error {
    io::Error::other(err)
}

fn executable() -> io::Result<PathBuf> {
    if let Some(chrome) = env::var_os("CHROME") {
        return Ok(chrome.into());
    }
    let path = env::var_os("PATH").unwrap_or_default();
    NAMES
        .iter()
        .flat_map(|name| env::split_paths(&path).map(move |dir| dir.join(name)))
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| other("found no Chrome or Chromium on `PATH`, set `CHROME` to one"))
}

/// A headless Chrome showing one page, driven through its remote debugging protocol over the
/// WebSocket that `--remote-debugging-port` opens. Only the handful of messages the guides need are
/// implemented, which keeps a protocol crate, and all the code it generates, out of the build
pub struct Chrome {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
    session: Option<String>,
    next_id: u64,
    _process: Process,
}

impl Chrome {
    /// Starts Chrome with its profile in `profile`, showing a blank page of `width` by `height`
    /// CSS pixels at twice the usual pixel density
    ///
    /// # Errors
    ///
    /// When Chrome cannot be found or started, or connecting to its remote debugging port fails
    pub fn start(profile: &Path, width: u32, height: u32) -> io::Result<Self> {
        let executable = executable()?;
        let mut process = Process::spawn(
            Command::new(&executable)
                .args([
                    "--headless=new",
                    "--disable-gpu",
                    "--hide-scrollbars",
                    "--no-first-run",
                    "--no-default-browser-check",
                    "--remote-debugging-port=0",
                    // The sandbox needs user namespaces that Ubuntu runners do not allow, and the
                    // browser only ever opens the local notebook server
                    "--no-sandbox",
                ])
                .arg(format!("--user-data-dir={}", profile.display()))
                .arg(format!("--window-size={width},{height}"))
                .arg("about:blank"),
        )?;
        let url = process.wait_for_line("Chrome", Duration::from_mins(1), |line| {
            line.strip_prefix("DevTools listening on ")
                .map(str::to_owned)
        })?;
        log(format!("Started `{}`", executable.display()));

        let (host, path) = url
            .strip_prefix("ws://")
            .and_then(|url| url.split_once('/'))
            .ok_or_else(|| other(format!("unexpected DevTools address `{url}`")))?;
        let writer = TcpStream::connect(host)?;
        writer.set_read_timeout(Some(Duration::from_mins(10)))?;
        let mut reader = BufReader::new(writer.try_clone()?);
        write!(
            &writer,
            "GET /{path} HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
             Sec-WebSocket-Key: {WEBSOCKET_KEY}\r\nSec-WebSocket-Version: 13\r\n\r\n"
        )?;
        let mut line = String::new();
        reader.read_line(&mut line)?;
        if !line.starts_with("HTTP/1.1 101") {
            return Err(other(format!("DevTools refused the connection: {line}")));
        }
        while line != "\r\n" && !line.is_empty() {
            line.clear();
            reader.read_line(&mut line)?;
        }

        let mut chrome = Self {
            reader,
            writer,
            session: None,
            next_id: 0,
            _process: process,
        };
        let target = chrome.send("Target.createTarget", json!({ "url": "about:blank" }))?;
        let session = chrome.send(
            "Target.attachToTarget",
            json!({ "targetId": target["targetId"], "flatten": true }),
        )?;
        chrome.session = session["sessionId"].as_str().map(str::to_owned);
        chrome.send("Page.enable", json!({}))?;
        chrome.send(
            "Emulation.setDeviceMetricsOverride",
            json!({ "width": width, "height": height, "deviceScaleFactor": 2, "mobile": false }),
        )?;
        Ok(chrome)
    }

    fn send_frame(&mut self, text: &str) -> io::Result<()> {
        // A text frame that is complete in itself
        let mut frame = vec![0x81];
        // Frames from a client must be marked as masked. A key of zeros leaves the payload as it
        // is, which saves masking it
        let masked = 0x80;
        match text.len() {
            len @ 0..=125 => frame.push(masked | len as u8),
            len @ 126..=0xffff => {
                frame.push(masked | 0x7e);
                frame.extend((len as u16).to_be_bytes());
            }
            len => {
                frame.push(masked | 0x7f);
                frame.extend((len as u64).to_be_bytes());
            }
        }
        frame.extend([0; 4]);
        frame.extend(text.as_bytes());
        self.writer.write_all(&frame)
    }

    fn read_message(&mut self) -> io::Result<Value> {
        let mut message = Vec::new();
        loop {
            let mut head = [0; 2];
            self.reader.read_exact(&mut head)?;
            let len = match head[1] & 0x7f {
                126 => {
                    let mut len = [0; 2];
                    self.reader.read_exact(&mut len)?;
                    u64::from(u16::from_be_bytes(len))
                }
                127 => {
                    let mut len = [0; 8];
                    self.reader.read_exact(&mut len)?;
                    u64::from_be_bytes(len)
                }
                len => u64::from(len),
            };
            let mut payload = vec![0; len as usize];
            self.reader.read_exact(&mut payload)?;
            match head[0] & 0x0f {
                0x8 => return Err(other("Chrome closed the DevTools connection")),
                // Pings and pongs, which Chrome does not need answered on a local connection
                0x9 | 0xa => continue,
                _ => message.extend(payload),
            }
            if head[0] & 0x80 != 0 {
                return serde_json::from_slice(&message).map_err(other);
            }
        }
    }

    /// Calls the protocol method `method` on the page and returns its result
    fn send(&mut self, method: &str, params: Value) -> io::Result<Value> {
        self.next_id += 1;
        let id = self.next_id;
        let mut message = json!({ "id": id, "method": method });
        message["params"] = params;
        if let Some(session) = &self.session {
            message["sessionId"] = session.clone().into();
        }
        self.send_frame(&message.to_string())?;
        // Events arrive among the replies, and none of them is needed
        loop {
            let mut reply = self.read_message()?;
            if reply["id"] == id {
                if let Some(error) = reply.get("error") {
                    return Err(other(format!("{method} failed: {}", error["message"])));
                }
                return Ok(reply["result"].take());
            }
        }
    }

    /// Runs `source` in every page loaded from now on, before the page's own scripts
    pub fn add_script(&mut self, source: &str) -> io::Result<()> {
        self.send(
            "Page.addScriptToEvaluateOnNewDocument",
            json!({ "source": source }),
        )
        .map(drop)
    }

    /// Opens `url` and waits until the page shows it
    pub fn navigate(&mut self, url: &str) -> io::Result<()> {
        self.send("Page.navigate", json!({ "url": url }))?;
        let arrived = format!("location.href === {}", Value::from(url));
        let deadline = Instant::now() + Duration::from_mins(2);
        // Evaluating fails while the old page goes away, which only means waiting longer
        while !matches!(self.evaluate(&arrived), Ok(Value::Bool(true))) {
            if Instant::now() > deadline {
                return Err(other(format!("`{url}` did not open in time")));
            }
            thread::sleep(Duration::from_millis(100));
        }
        Ok(())
    }

    /// Evaluates the JavaScript `expression`, waiting for the promise it may return
    pub fn evaluate(&mut self, expression: &str) -> io::Result<Value> {
        let mut result = self.send(
            "Runtime.evaluate",
            json!({ "expression": expression, "awaitPromise": true, "returnByValue": true }),
        )?;
        if let Some(exception) = result.get("exceptionDetails") {
            let description = exception["exception"]["description"]
                .as_str()
                .or_else(|| exception["text"].as_str())
                .unwrap_or_default();
            return Err(other(format!("`{expression}` failed: {description}")));
        }
        Ok(result["result"]["value"].take())
    }

    /// A PNG of the part of the page at `x` and `y` of `width` by `height` CSS pixels
    pub fn screenshot(&mut self, x: f64, y: f64, width: f64, height: f64) -> io::Result<Vec<u8>> {
        let result = self.send(
            "Page.captureScreenshot",
            json!({
                "format": "png",
                "fromSurface": true,
                "clip": { "x": x, "y": y, "width": width, "height": height, "scale": 1 },
            }),
        )?;
        BASE64_STANDARD
            .decode(result["data"].as_str().unwrap_or_default())
            .map_err(other)
    }
}
