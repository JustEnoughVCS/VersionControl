use std::io;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum ConnectionError {
    #[error("I/O error: {0}")]
    Io(String),
}

impl From<io::Error> for ConnectionError {
    fn from(error: io::Error) -> Self {
        ConnectionError::Io(error.to_string())
    }
}
