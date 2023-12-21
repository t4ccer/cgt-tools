use std::{
    fs::File,
    io::{self, stderr, stdout, Stderr, Stdout},
};

macro_rules! define_file_or_std {
    ($name:ident, $writer:ident, $std_enum:ident, $std_impl:ident, $std_mk:ident) => {
        #[derive(Debug, Clone)]
        pub enum $name {
            FilePath(String),
            $std_enum,
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                if &value == "-" {
                    Self::$std_enum
                } else {
                    Self::FilePath(value)
                }
            }
        }

        impl $name {
            fn create_with<'a, F>(&'a self, f: F) -> io::Result<$writer>
            where
                F: FnOnce(&'a str) -> io::Result<File>,
            {
                match self {
                    Self::FilePath(ref fp) => Ok($writer::File(f(fp)?)),
                    Self::$std_enum => Ok($writer::$std_enum($std_mk())),
                }
            }

            #[allow(dead_code)]
            pub fn open(&self) -> io::Result<$writer> {
                self.create_with(File::open)
            }

            #[allow(dead_code)]
            pub fn create(&self) -> io::Result<$writer> {
                self.create_with(File::create)
            }
        }

        pub enum $writer {
            File(File),
            $std_enum($std_impl),
        }

        impl io::Write for $writer {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                match self {
                    Self::File(f) => f.write(buf),
                    Self::$std_enum(fd) => {
                        let mut lock = fd.lock();
                        lock.write(buf)
                    }
                }
            }

            fn flush(&mut self) -> io::Result<()> {
                match self {
                    Self::File(f) => f.flush(),
                    Self::$std_enum(fd) => {
                        let mut lock = fd.lock();
                        lock.flush()
                    }
                }
            }
        }
    };
}

define_file_or_std!(
    FileOrStderr,
    FileOrStderrWriter,
    FileOrStderr,
    Stderr,
    stderr
);

define_file_or_std!(
    FileOrStdout,
    FileOrStdoutWriter,
    FileOrStdout,
    Stdout,
    stdout
);
