use std::{
    fs::File,
    io::{self, Stderr, Stdin, Stdout, stderr, stdin, stdout},
};

macro_rules! define_output_file_or_std {
    ($name:ident, $writer:ident, $std_enum:ident, $std_impl:ident, $std_mk:ident) => {
        #[derive(Debug, Clone)]
        pub enum $name {
            FilePath(String),
            $std_enum,
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                match self {
                    Self::$std_enum => write!(f, "-"),
                    Self::FilePath(value) => write!(f, "{value}"),
                }
            }
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

define_output_file_or_std!(
    FileOrStderr,
    FileOrStderrWriter,
    FileOrStderr,
    Stderr,
    stderr
);

define_output_file_or_std!(
    FileOrStdout,
    FileOrStdoutWriter,
    FileOrStdout,
    Stdout,
    stdout
);

macro_rules! define_input_file_or_std {
    ($name:ident, $reader:ident, $std_enum:ident, $std_impl:ident, $std_mk:ident) => {
        #[derive(Debug, Clone)]
        pub enum $name {
            FilePath(String),
            $std_enum,
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                match self {
                    Self::$std_enum => write!(f, "-"),
                    Self::FilePath(value) => write!(f, "{value}"),
                }
            }
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
            fn create_with<'a, F>(&'a self, f: F) -> io::Result<$reader>
            where
                F: FnOnce(&'a str) -> io::Result<File>,
            {
                match self {
                    Self::FilePath(ref fp) => Ok($reader::File(f(fp)?)),
                    Self::$std_enum => Ok($reader::$std_enum($std_mk())),
                }
            }

            #[allow(dead_code)]
            pub fn open(&self) -> io::Result<$reader> {
                self.create_with(File::open)
            }
        }

        pub enum $reader {
            File(File),
            $std_enum($std_impl),
        }

        impl io::Read for $reader {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                match self {
                    Self::File(f) => f.read(buf),
                    Self::$std_enum(fd) => {
                        let mut lock = fd.lock();
                        lock.read(buf)
                    }
                }
            }
        }
    };
}

define_input_file_or_std!(FileOrStdin, FileOrStdinReader, FileOrStdin, Stdin, stdin);
