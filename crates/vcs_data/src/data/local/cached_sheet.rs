use std::{io::Error, path::PathBuf};

use cfg_file::config::ConfigFile;
use string_proc::{format_path::format_path, snake_case};
use tokio::fs;

use crate::{
    constants::{
        CLIENT_FILE_CACHED_SHEET, CLIENT_PATH_CACHED_SHEET, CLIENT_SUFFIX_CACHED_SHEET_FILE,
    },
    current::current_local_path,
    data::sheet::{SheetData, SheetName},
};

pub type CachedSheetPathBuf = PathBuf;

const SHEET_NAME: &str = "{sheet_name}";
const ACCOUNT_NAME: &str = "{account}";

/// # Cached Sheet
/// The cached sheet is a read-only version cloned from the upstream repository to the local environment,
/// automatically generated during update operations,
/// which records the latest Sheet information stored locally to accelerate data access and reduce network requests.
pub struct CachedSheet;

impl CachedSheet {
    /// Read the cached sheet data.
    pub async fn cached_sheet_data(sheet_name: &SheetName) -> Result<SheetData, std::io::Error> {
        let sheet_name = snake_case!(sheet_name.clone());

        let Some(path) = Self::cached_sheet_path(sheet_name) else {
            return Err(Error::new(
                std::io::ErrorKind::NotFound,
                "Local workspace not found!",
            ));
        };
        let data = SheetData::read_from(path).await?;
        Ok(data)
    }

    /// Get the path to the cached sheet file.
    pub fn cached_sheet_path(sheet_name: SheetName) -> Option<PathBuf> {
        let current_workspace = current_local_path()?;
        Some(
            current_workspace
                .join(CLIENT_FILE_CACHED_SHEET.replace(SHEET_NAME, &sheet_name.to_string())),
        )
    }

    /// Get all cached sheet names
    pub async fn cached_sheet_names() -> Result<Vec<SheetName>, std::io::Error> {
        let mut dir = fs::read_dir(CLIENT_PATH_CACHED_SHEET).await?;
        let mut sheet_names = Vec::new();

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();

            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.ends_with(CLIENT_SUFFIX_CACHED_SHEET_FILE) {
                        let name_without_ext = file_name
                            .trim_end_matches(CLIENT_SUFFIX_CACHED_SHEET_FILE)
                            .to_string();
                        sheet_names.push(name_without_ext);
                    }
                }
            }
        }

        Ok(sheet_names)
    }

    /// Get all cached sheet paths
    pub async fn cached_sheet_paths() -> Result<Vec<CachedSheetPathBuf>, std::io::Error> {
        let mut dir = fs::read_dir(CLIENT_PATH_CACHED_SHEET).await?;
        let mut sheet_paths = Vec::new();
        let Some(workspace_path) = current_local_path() else {
            return Err(Error::new(
                std::io::ErrorKind::NotFound,
                "Local workspace not found!",
            ));
        };

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();

            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.ends_with(CLIENT_SUFFIX_CACHED_SHEET_FILE) {
                        sheet_paths.push(format_path(workspace_path.join(path))?);
                    }
                }
            }
        }

        Ok(sheet_paths)
    }
}
