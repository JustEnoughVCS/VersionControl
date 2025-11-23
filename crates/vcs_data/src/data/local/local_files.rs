use std::path::{Path, PathBuf};

use string_proc::format_path::format_path;
use tokio::fs;

use crate::constants::CLIENT_FOLDER_WORKSPACE_ROOT_NAME;

pub struct RelativeFiles {
    pub(crate) files: Vec<PathBuf>,
}

impl IntoIterator for RelativeFiles {
    type Item = PathBuf;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.files.into_iter()
    }
}

impl RelativeFiles {
    pub fn iter(&self) -> std::slice::Iter<'_, PathBuf> {
        self.files.iter()
    }
}

/// Read the relative paths within the project from the input file list
pub async fn get_relative_paths(local_path: &PathBuf, paths: &[PathBuf]) -> Option<RelativeFiles> {
    // Get Relative Paths
    let Ok(paths) = format_input_paths_and_ignore_outside_paths(local_path, paths).await else {
        return None;
    };
    let files: Vec<PathBuf> = abs_paths_to_abs_files(paths).await;
    let Ok(files) = parse_to_relative(local_path, files) else {
        return None;
    };
    Some(RelativeFiles { files })
}

/// Normalize the input paths
async fn format_input_paths(
    local_path: &Path,
    track_files: &[PathBuf],
) -> Result<Vec<PathBuf>, std::io::Error> {
    let current_dir = local_path;

    let mut real_paths = Vec::new();
    for file in track_files {
        let path = current_dir.join(file);

        // Skip paths that contain .jv directories
        if path.components().any(|component| {
            if let std::path::Component::Normal(name) = component {
                name.to_str() == Some(CLIENT_FOLDER_WORKSPACE_ROOT_NAME)
            } else {
                false
            }
        }) {
            continue;
        }

        match format_path(path) {
            Ok(path) => real_paths.push(path),
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to format path: {}", e),
                ));
            }
        }
    }

    Ok(real_paths)
}

/// Ignore files outside the workspace
async fn format_input_paths_and_ignore_outside_paths(
    local_path: &PathBuf,
    files: &[PathBuf],
) -> Result<Vec<PathBuf>, std::io::Error> {
    let result = format_input_paths(local_path, files).await?;
    let result: Vec<PathBuf> = result
        .into_iter()
        .filter(|path| path.starts_with(local_path))
        .collect();
    Ok(result)
}

/// Normalize the input paths to relative paths
fn parse_to_relative(
    local_dir: &PathBuf,
    files: Vec<PathBuf>,
) -> Result<Vec<PathBuf>, std::io::Error> {
    let result: Result<Vec<PathBuf>, _> = files
        .iter()
        .map(|p| {
            p.strip_prefix(local_dir)
                .map(|relative| relative.to_path_buf())
                .map_err(|_| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Path prefix stripping failed",
                    )
                })
        })
        .collect();

    result
}

/// Convert absolute paths to absolute file paths, expanding directories to their contained files
async fn abs_paths_to_abs_files(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for path in paths {
        if !path.exists() {
            continue;
        }

        let metadata = match fs::metadata(&path).await {
            Ok(meta) => meta,
            Err(_) => continue,
        };

        if metadata.is_file() {
            files.push(path);
        } else if metadata.is_dir() {
            let walker = walkdir::WalkDir::new(&path);
            for entry in walker.into_iter().filter_map(|e| e.ok()) {
                if entry.path().components().any(|component| {
                    if let std::path::Component::Normal(name) = component {
                        name == CLIENT_FOLDER_WORKSPACE_ROOT_NAME
                    } else {
                        false
                    }
                }) {
                    continue;
                }

                if entry.file_type().is_file() {
                    files.push(entry.path().to_path_buf());
                }
            }
        }
    }

    files
}
