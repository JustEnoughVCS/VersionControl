#[derive(Debug, thiserror::Error)]
pub enum StorageIOError {
    #[error("IO error: {0}")]
    IOErr(#[from] std::io::Error),

    #[error("Hash too short")]
    HashTooShort,
}
