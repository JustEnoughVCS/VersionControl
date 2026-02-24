#[derive(Debug, thiserror::Error)]
pub enum ParseMappingError {
    #[error("Mapping information is invalid and cannot be safely converted to LocalMapping")]
    InvalidMapping,
}
