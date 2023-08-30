use std::io;

pub enum FileStreamOpenError {
    FromIo(io::Error),
    FromSymphonia(symphonia::core::errors::Error),
    NoTrackFound,
}

impl From<symphonia::core::errors::Error> for FileStreamOpenError {
    fn from(value: symphonia::core::errors::Error) -> Self {
        FileStreamOpenError::FromSymphonia(value)
    }
}

impl From<io::Error> for FileStreamOpenError {
    fn from(value: io::Error) -> Self {
        FileStreamOpenError::FromIo(value)
    }
}
