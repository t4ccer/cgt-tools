use std::{
    fmt::Display,
    fs::File,
    io::{BufReader, Read},
};

#[derive(Debug)]
pub enum ToFromFileError {
    DecodeError(Box<bincode::ErrorKind>),
    FileError(std::io::Error),
}

impl Display for ToFromFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToFromFileError::DecodeError(_) => write!(f, "Could not decode the file"),
            ToFromFileError::FileError(_) => write!(f, "Could not access the file"),
        }
    }
}

impl std::error::Error for ToFromFileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ToFromFileError::DecodeError(e) => Some(e),
            ToFromFileError::FileError(e) => Some(e),
        }
    }
}

impl From<Box<bincode::ErrorKind>> for ToFromFileError {
    fn from(value: Box<bincode::ErrorKind>) -> Self {
        Self::DecodeError(value)
    }
}

impl From<std::io::Error> for ToFromFileError {
    fn from(value: std::io::Error) -> Self {
        Self::FileError(value)
    }
}

pub trait ToFromFile {
    fn save_to_file(&self, file_path: &str) -> Result<(), ToFromFileError>;
    fn load_from_file(file_path: &str) -> Result<Self, ToFromFileError>
    where
        Self: Sized;
}

impl<'de, T> ToFromFile for T
where
    T: serde::Serialize + serde::de::DeserializeOwned + Sized,
{
    fn save_to_file(&self, file_path: &str) -> Result<(), ToFromFileError> {
        let serialized = bincode::serialize(self)?;
        let mut f = File::create(file_path)?;
        std::io::Write::write_all(&mut f, &serialized)?;
        Ok(())
    }

    fn load_from_file(file_path: &str) -> Result<Self, ToFromFileError> {
        let f = File::open(file_path)?;
        let mut reader = BufReader::new(f);
        let mut buffer: Vec<u8> = Vec::new();
        reader.read_to_end(&mut buffer)?;
        let res = bincode::deserialize::<Self>(&buffer)?;
        Ok(res)
    }
}
