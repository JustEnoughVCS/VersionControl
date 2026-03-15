use thiserror::Error;

#[derive(Error, Debug)]
pub enum IDAliasError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid alias file: {0}")]
    InvalidAliasFile(String),

    #[error("Alias not found for ID: {0}")]
    AliasNotFound(u32),

    #[error("Invalid file offset: {0}")]
    InvalidOffset(u64),

    #[error("File size mismatch: expected {0}, got {1}")]
    FileSizeMismatch(u64, u64),
}
