use crate::sheet::SheetEditItem;

#[derive(Debug, thiserror::Error)]
pub enum SheetEditError {
    #[error("Edit `{0}` Failed: Node already exists: `{1}`")]
    NodeAlreadyExist(SheetEditItem, String),

    #[error("Edit `{0}` Failed: Node not found: `{1}`")]
    NodeNotFound(SheetEditItem, String),
}

#[derive(Debug, thiserror::Error)]
pub enum ReadSheetDataError {
    #[error("IO error: {0}")]
    IOErr(#[from] std::io::Error),
}
