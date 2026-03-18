use framework::space::error::SpaceError;

#[derive(thiserror::Error, Debug)]
pub enum MakeSheetError {
    #[error("Sheet already exists")]
    SheetAlreadyExists,

    #[error("Sheet not found")]
    SheetNotFound,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}

impl From<SpaceError> for MakeSheetError {
    fn from(value: SpaceError) -> Self {
        match value {
            SpaceError::SpaceNotFound => Self::SheetNotFound,
            SpaceError::PathFormatError(path_format_error) => {
                Self::Other(format!("PathFormatError: {}", path_format_error))
            }
            SpaceError::Io(error) => Self::Io(error),
            SpaceError::Other(msg) => Self::Other(msg),
        }
    }
}
