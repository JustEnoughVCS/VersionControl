use asset_system::error::{DataApplyError, DataReadError, DataWriteError, HandleLockError};
use framework::space::error::SpaceError;
use sheet_system::index_source::error::IDAliasError;

#[derive(thiserror::Error, Debug)]
pub enum WorkspaceOperationError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),

    #[error("Configuration not found")]
    ConfigNotFound,

    #[error("Workspace not found")]
    WorkspaceNotFound,

    #[error("Handle lock error: {0}")]
    HandleLock(#[from] HandleLockError),

    #[error("Data read error: {0}")]
    DataRead(#[from] DataReadError),

    #[error("Data write error: {0}")]
    DataWrite(#[from] DataWriteError),

    #[error("Data apply error: {0}")]
    DataApply(#[from] DataApplyError),

    #[error("ID alias error: {0}")]
    IDAliasError(#[from] IDAliasError),
}

impl From<SpaceError> for WorkspaceOperationError {
    fn from(value: SpaceError) -> Self {
        match value {
            SpaceError::SpaceNotFound => WorkspaceOperationError::WorkspaceNotFound,
            SpaceError::Io(error) => WorkspaceOperationError::Io(error),
            SpaceError::Other(e) => Self::Other(e),
            _ => Self::Other(value.to_string()),
        }
    }
}
