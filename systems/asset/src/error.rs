use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum DataReadError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    ParseError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum DataWriteError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum DataApplyError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Asset file not found: {0}")]
    AssetFileNotFound(PathBuf),

    #[error("Pre-check failed: {0}")]
    PrecheckFailed(#[from] PrecheckFailed),
}

#[derive(Debug, thiserror::Error)]
pub enum PrecheckFailed {
    /// Lock file does not exist
    /// Means trying to apply a modification that cannot be applied
    #[error("Lock not found")]
    LockNotFound,

    /// Asset file does not exist
    /// Apply phase will fail due to this condition
    #[error("Asset not found")]
    AssetNotFound,

    // Note: !writed is allowed,
    // but writed without a TEMP_FILE is not allowed
    /// Handle produced a write
    /// but the temporary file does not exist, indicating an abnormal write operation
    #[error("Temp not found")]
    WritedButTempNotFound,

    /// Asset path is invalid
    /// This is not an issue that should arise from normal Handle creation flow
    #[error("Asset path invalid")]
    AssetPathInvalid,

    /// Temporary file cannot be moved
    #[error("Temp file not moveable")]
    TempNotMoveable,

    /// Asset file cannot be moved
    #[error("Asset file not writable")]
    AssetNotWritable,

    /// Handle is processing a cross-directory operation
    /// This is not atomic
    #[error("Handle is cross-directory")]
    HandleIsCrossDirectory,

    /// Asset file has no parent directory
    /// This is not a valid path for file operations
    #[error("Asset file has no parent directory")]
    HandleFileIsNoParent,

    /// A handle with the same path already exists
    /// This operation will cause a conflict
    #[error("Handle with same path exists")]
    HasSamePath,

    #[error("Lock on lock file")]
    LockOnLockFile,

    #[error("Temp file for temp file")]
    TempForTempFile,

    #[error("Asset path cannot be formatted")]
    FormatPathFailed,
}

#[derive(Debug, thiserror::Error)]
pub enum HandleLockError {
    #[error("Parse path failed")]
    ParsePathFailed,

    #[error("Asset file not found")]
    AssetFileNotFound(PathBuf),

    #[error("Read file name failed")]
    ReadFileNameFailed,

    #[error("Asset locked")]
    AssetLocked,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Pre-check failed: {0}")]
    PrecheckFailed(#[from] PrecheckFailed),
}
