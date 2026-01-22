use std::io;
use thiserror::Error;
use vcs_data::data::member::MemberId;

#[derive(Error, Debug, Clone)]
pub enum ConnectionError {
    #[error("I/O error: {0}")]
    Io(String),
}

#[derive(Error, Debug, Clone)]
pub enum ProcessActionError {
    #[error("Action `{0}` not registered")]
    ActionNotRegistered(String),

    #[error("Authorize `{0}` failed")]
    AuthorizeFailed(MemberId),

    #[error("Authorize host `{0}` failed")]
    AuthorizeHostFailed(MemberId),
}

impl From<io::Error> for ConnectionError {
    fn from(error: io::Error) -> Self {
        ConnectionError::Io(error.to_string())
    }
}
