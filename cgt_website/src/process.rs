use std::{
    fs,
    io::{self, BufRead, BufReader},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};

/// A child process that is killed when dropped, so that none outlives a failed build
pub struct Process(Child);

impl Process {
    /// Spawns `command` with its standard error piped, for [`Process::wait_for_line`]
    pub fn spawn(command: &mut Command) -> io::Result<Self> {
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map(Self)
    }

    /// Reads the standard error of the process until `find` picks something out of a line, and
    /// returns that. The rest is read and dropped in the background, because a full pipe would
    /// stop the process
    pub fn wait_for_line(
        &mut self,
        what: &str,
        timeout: Duration,
        find: impl Fn(&str) -> Option<String> + Send + 'static,
    ) -> io::Result<String> {
        let stderr = self.0.stderr.take();
        let (found, wait) = mpsc::channel();
        thread::spawn(move || {
            let mut log = Vec::new();
            for line in stderr
                .into_iter()
                .flat_map(|stderr| BufReader::new(stderr).lines().map_while(Result::ok))
            {
                if let Some(value) = find(&line) {
                    let _ = found.send(Ok(value));
                }
                log.push(line);
            }
            let _ = found.send(Err(log.join("\n")));
        });
        match wait.recv_timeout(timeout) {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(log)) => Err(io::Error::other(format!("{what} exited on start:\n{log}"))),
            Err(_) => Err(io::Error::other(format!("{what} did not start in time"))),
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A directory that is removed with everything in it when dropped
pub struct TempDir(pub PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
