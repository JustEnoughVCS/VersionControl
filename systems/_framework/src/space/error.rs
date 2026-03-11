#[derive(thiserror::Error, Debug)]
pub enum SpaceError {
    #[error("Space not found")]
    SpaceNotFound,

    #[error("Path format error: {0}")]
    PathFormatError(#[from] just_fmt::fmt_path::PathFormatError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl PartialEq for SpaceError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Io(_), Self::Io(_)) => true,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}
