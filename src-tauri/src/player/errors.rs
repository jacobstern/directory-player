use std::{error::Error, fmt::Display, io};

#[derive(Debug)]
pub enum FileStreamOpenError {
    IoError(io::Error),
    SymphoniaError(symphonia::core::errors::Error),
    NoTrackFound,
}

impl Error for FileStreamOpenError {}

impl Display for FileStreamOpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl From<symphonia::core::errors::Error> for FileStreamOpenError {
    fn from(value: symphonia::core::errors::Error) -> Self {
        FileStreamOpenError::SymphoniaError(value)
    }
}

impl From<io::Error> for FileStreamOpenError {
    fn from(value: io::Error) -> Self {
        FileStreamOpenError::IoError(value)
    }
}
