use crate::mapping::error::ParseMappingError;

#[derive(Debug, thiserror::Error)]
pub enum SheetEditError {
    #[error("Edit Failed: Node already exists: `{0}`")]
    NodeAlreadyExist(String),

    #[error("Edit Failed: Node not found: `{0}`")]
    NodeNotFound(String),
}

#[derive(Debug, thiserror::Error)]
pub enum SheetApplyError {
    #[error("Node already exists: `{0}` (Unexpected)")]
    UnexpectedAlreadyExist(String),

    #[error("Unexpected error: Node not found: `{0}` (Unexpected)")]
    UnexpectedNotFound(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ParseSheetError {
    #[error("Parse mapping error: {0}")]
    ParseMappingError(#[from] ParseMappingError),

    #[error("Sheet edit error: {0}")]
    SheetEditError(#[from] SheetEditError),

    #[error("Sheet apply error: {0}")]
    SheetApplyError(#[from] SheetApplyError),
}

#[derive(Debug, thiserror::Error)]
pub enum ReadSheetDataError {
    #[error("IO error: {0}")]
    IOErr(#[from] std::io::Error),
}
