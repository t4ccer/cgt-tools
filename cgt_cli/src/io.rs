use std::{
    ffi::OsStr,
    fs::File,
    io::{self, Stderr, Stdin, Stdout, stderr, stdin, stdout},
    marker::PhantomData,
    path::{Path, PathBuf},
};

const STD_SYMBOL: &str = "-";

pub enum FilePathOr<Stream> {
    FilePath(PathBuf),
    Std(PhantomData<Stream>),
}

impl<Stream> Clone for FilePathOr<Stream> {
    fn clone(&self) -> FilePathOr<Stream> {
        match self {
            FilePathOr::FilePath(file_path) => FilePathOr::FilePath(file_path.clone()),
            FilePathOr::Std(_) => FilePathOr::Std(PhantomData),
        }
    }
}

impl<Stream> core::fmt::Debug for FilePathOr<Stream> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FilePathOr::FilePath(file_path) => f.debug_tuple("FilePath").field(file_path).finish(),
            FilePathOr::Std(ty) => f.debug_tuple("Std").field(&ty).finish(),
        }
    }
}

impl<Stream> ::core::fmt::Display for FilePathOr<Stream> {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        match self {
            FilePathOr::FilePath(file_path) => write!(f, "{}", file_path.display()),
            FilePathOr::Std(_) => write!(f, "{}", STD_SYMBOL),
        }
    }
}

impl<Stream> From<String> for FilePathOr<Stream> {
    fn from(path: String) -> FilePathOr<Stream> {
        if path == STD_SYMBOL {
            FilePathOr::Std(PhantomData)
        } else {
            FilePathOr::FilePath(path.into())
        }
    }
}

impl<Stream> From<PathBuf> for FilePathOr<Stream> {
    fn from(path: PathBuf) -> FilePathOr<Stream> {
        if path.as_os_str() == OsStr::new(STD_SYMBOL) {
            FilePathOr::Std(PhantomData)
        } else {
            FilePathOr::FilePath(path)
        }
    }
}

impl<Stream> FilePathOr<Stream>
where
    Stream: StdStream,
{
    fn with_file_path<'a, F>(&'a self, mk_file: F) -> io::Result<FileOr<Stream>>
    where
        F: FnOnce(&'a Path) -> io::Result<File>,
    {
        match self {
            FilePathOr::FilePath(path) => Ok(FileOr::File(mk_file(path)?)),
            FilePathOr::Std(_) => Ok(FileOr::Std(Stream::get_handle())),
        }
    }

    pub fn open(&self) -> io::Result<FileOr<Stream>> {
        self.with_file_path(File::open)
    }

    pub fn create(&self) -> io::Result<FileOr<Stream>> {
        self.with_file_path(File::create)
    }
}

pub enum FileOr<Stream> {
    File(File),
    Std(Stream),
}

impl<Stream> io::Write for FileOr<Stream>
where
    Stream: io::Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            FileOr::File(file) => file.write(buf),
            FileOr::Std(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            FileOr::File(file) => file.flush(),
            FileOr::Std(stream) => stream.flush(),
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        match self {
            FileOr::File(file) => file.write_all(buf),
            FileOr::Std(stream) => stream.write_all(buf),
        }
    }
}

impl<Stream> io::Read for FileOr<Stream>
where
    Stream: io::Read,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            FileOr::File(file) => file.read(buf),
            FileOr::Std(stream) => stream.read(buf),
        }
    }
}

impl<Stream> From<Stream> for FileOr<Stream>
where
    Stream: StdStream,
{
    fn from(stream: Stream) -> FileOr<Stream> {
        FileOr::Std(stream)
    }
}

impl<Stream> From<File> for FileOr<Stream> {
    fn from(file: File) -> FileOr<Stream> {
        FileOr::File(file)
    }
}

pub trait StdStream {
    fn get_handle() -> Self;
}

impl StdStream for Stdout {
    fn get_handle() -> Stdout {
        stdout()
    }
}

impl StdStream for Stderr {
    fn get_handle() -> Stderr {
        stderr()
    }
}

impl StdStream for Stdin {
    fn get_handle() -> Stdin {
        stdin()
    }
}
