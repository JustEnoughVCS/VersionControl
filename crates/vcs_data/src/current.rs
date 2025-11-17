use crate::constants::*;
use std::io::{self, Error};
use std::{env::set_current_dir, path::PathBuf};

/// Find the nearest vault or local workspace and correct the `current_dir` to it
pub fn correct_current_dir() -> Result<(), io::Error> {
    if let Some(local_workspace) = current_local_path() {
        set_current_dir(local_workspace)?;
        return Ok(());
    }
    if let Some(vault) = current_vault_path() {
        set_current_dir(vault)?;
        return Ok(());
    }
    Err(Error::new(
        io::ErrorKind::NotFound,
        "Could not find any vault or local workspace!",
    ))
}

/// Get the nearest Vault directory from `current_dir`
pub fn current_vault_path() -> Option<PathBuf> {
    let current_dir = std::env::current_dir().ok()?;
    find_vault_path(current_dir)
}

/// Get the nearest local workspace from `current_dir`
pub fn current_local_path() -> Option<PathBuf> {
    let current_dir = std::env::current_dir().ok()?;
    find_local_path(current_dir)
}

/// Get the nearest Vault directory from the specified path
pub fn find_vault_path(path: impl Into<PathBuf>) -> Option<PathBuf> {
    let mut current_path = path.into();
    let vault_file = SERVER_FILE_VAULT;

    loop {
        let vault_toml_path = current_path.join(vault_file);
        if vault_toml_path.exists() {
            return Some(current_path);
        }

        if let Some(parent) = current_path.parent() {
            current_path = parent.to_path_buf();
        } else {
            break;
        }
    }

    None
}

/// Get the nearest local workspace from the specified path
pub fn find_local_path(path: impl Into<PathBuf>) -> Option<PathBuf> {
    let mut current_path = path.into();
    let workspace_dir = CLIENT_PATH_WORKSPACE_ROOT;

    loop {
        let jvc_path = current_path.join(workspace_dir);
        if jvc_path.exists() {
            return Some(current_path);
        }

        if let Some(parent) = current_path.parent() {
            current_path = parent.to_path_buf();
        } else {
            break;
        }
    }

    None
}

/// Get the system's document directory and join with .just_enough_vcs
pub fn current_doc_dir() -> Option<PathBuf> {
    dirs::config_local_dir().map(|path| path.join("jvcs"))
}
