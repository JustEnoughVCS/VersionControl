use crate::current::current_doc_dir;
use std::path::PathBuf;

pub mod accounts;

pub struct UserDirectory {
    local_path: PathBuf,
}

impl UserDirectory {
    /// Create a user ditectory struct from the current system's document directory
    pub fn current_doc_dir() -> Option<Self> {
        Some(UserDirectory {
            local_path: current_doc_dir()?,
        })
    }

    /// Create a user directory struct from a specified directory path
    /// Returns None if the directory does not exist
    pub fn from_path<P: Into<PathBuf>>(path: P) -> Option<Self> {
        let local_path = path.into();
        if local_path.exists() {
            Some(UserDirectory { local_path })
        } else {
            None
        }
    }
}
