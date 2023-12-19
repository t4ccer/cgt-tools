use std::{
    fs::File,
    io::{self, stderr, Stderr},
};

#[derive(Debug, Clone)]
pub enum FileOrStderr {
    FilePath(String),
    Stderr,
}

impl From<String> for FileOrStderr {
    fn from(value: String) -> Self {
        if &value == "-" {
            Self::Stderr
        } else {
            Self::FilePath(value)
        }
    }
}

impl FileOrStderr {
    fn create_with<'a, F>(&'a self, f: F) -> io::Result<FileOrStderrWriter>
    where
        F: FnOnce(&'a str) -> io::Result<File>,
    {
        match self {
            FileOrStderr::FilePath(ref fp) => Ok(FileOrStderrWriter::File(f(fp)?)),
            FileOrStderr::Stderr => Ok(FileOrStderrWriter::Stderr(stderr())),
        }
    }

    #[allow(dead_code)]
    pub fn open(&self) -> io::Result<FileOrStderrWriter> {
        self.create_with(File::open)
    }

    #[allow(dead_code)]
    pub fn create(&self) -> io::Result<FileOrStderrWriter> {
        self.create_with(File::create)
    }
}

pub enum FileOrStderrWriter {
    File(File),
    Stderr(Stderr),
}

impl io::Write for FileOrStderrWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            FileOrStderrWriter::File(f) => f.write(buf),
            FileOrStderrWriter::Stderr(stderr) => {
                let mut lock = stderr.lock();
                lock.write(buf)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            FileOrStderrWriter::File(f) => f.flush(),
            FileOrStderrWriter::Stderr(stderr) => {
                let mut lock = stderr.lock();
                lock.flush()
            }
        }
    }
}
