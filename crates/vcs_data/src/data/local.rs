use std::{
    collections::HashMap,
    env::current_dir,
    path::{Path, PathBuf},
    sync::Arc,
};

use cfg_file::config::ConfigFile;
use string_proc::format_path::format_path;
use tokio::{fs, sync::Mutex};
use vcs_docs::docs::READMES_LOCAL_WORKSPACE_TODOLIST;

use crate::{
    constants::{
        CLIENT_CONTENT_GITIGNORE, CLIENT_FILE_GITIGNORE, CLIENT_FILE_LOCAL_SHEET,
        CLIENT_FILE_TODOLIST, CLIENT_FILE_WORKSPACE, CLIENT_FOLDER_WORKSPACE_ROOT_NAME,
        CLIENT_PATH_LOCAL_SHEET, CLIENT_SUFFIX_LOCAL_SHEET_FILE,
    },
    current::{current_local_path, find_local_path},
    data::{
        local::{
            config::LocalConfig,
            local_sheet::{LocalSheet, LocalSheetData, LocalSheetPathBuf},
        },
        member::MemberId,
        sheet::SheetName,
    },
};

pub mod align;
pub mod cached_sheet;
pub mod config;
pub mod file_status;
pub mod latest_file_data;
pub mod latest_info;
pub mod local_files;
pub mod local_sheet;

const SHEET_NAME: &str = "{sheet_name}";
const ACCOUNT_NAME: &str = "{account}";

pub struct LocalWorkspace {
    config: Arc<Mutex<LocalConfig>>,
    local_path: PathBuf,
}

impl LocalWorkspace {
    /// Get the path of the local workspace.
    pub fn local_path(&self) -> &PathBuf {
        &self.local_path
    }

    /// Initialize local workspace.
    pub fn init(config: LocalConfig, local_path: impl Into<PathBuf>) -> Option<Self> {
        let local_path = find_local_path(local_path)?;
        Some(Self {
            config: Arc::new(Mutex::new(config)),
            local_path,
        })
    }

    /// Initialize local workspace in the current directory.
    pub fn init_current_dir(config: LocalConfig) -> Option<Self> {
        let local_path = current_local_path()?;
        Some(Self {
            config: Arc::new(Mutex::new(config)),
            local_path,
        })
    }

    /// Setup local workspace
    pub async fn setup_local_workspace(
        local_path: impl Into<PathBuf>,
    ) -> Result<(), std::io::Error> {
        let local_path: PathBuf = local_path.into();

        // Ensure directory is empty
        if local_path.exists() && local_path.read_dir()?.next().is_some() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::DirectoryNotEmpty,
                "DirectoryNotEmpty",
            ));
        }

        // 1. Setup config
        let config = LocalConfig::default();
        LocalConfig::write_to(&config, local_path.join(CLIENT_FILE_WORKSPACE)).await?;

        // 2. Setup SETUP.md
        let readme_content = READMES_LOCAL_WORKSPACE_TODOLIST.trim().to_string();
        fs::write(local_path.join(CLIENT_FILE_TODOLIST), readme_content).await?;

        // 3. Setup .gitignore
        fs::write(
            local_path.join(CLIENT_FILE_GITIGNORE),
            CLIENT_CONTENT_GITIGNORE,
        )
        .await?;

        // On Windows, set the .jv directory as hidden
        let jv_dir = local_path.join(CLIENT_FOLDER_WORKSPACE_ROOT_NAME);
        let _ = hide_folder::hide_folder(&jv_dir);

        Ok(())
    }

    /// Get a reference to the local configuration.
    pub fn config(&self) -> Arc<Mutex<LocalConfig>> {
        self.config.clone()
    }

    /// Setup local workspace in current directory
    pub async fn setup_local_workspace_current_dir() -> Result<(), std::io::Error> {
        Self::setup_local_workspace(current_dir()?).await?;
        Ok(())
    }

    /// Get the path to a local sheet.
    pub fn local_sheet_path(&self, member: &MemberId, sheet: &SheetName) -> PathBuf {
        let result = self.local_path.join(
            CLIENT_FILE_LOCAL_SHEET
                .replace(ACCOUNT_NAME, member)
                .replace(SHEET_NAME, sheet),
        );
        result
    }

    /// Read or initialize a local sheet.
    pub async fn local_sheet(
        &self,
        member: &MemberId,
        sheet: &SheetName,
    ) -> Result<LocalSheet<'_>, std::io::Error> {
        let local_sheet_path = self.local_sheet_path(member, sheet);

        if !local_sheet_path.exists() {
            let sheet_data = LocalSheetData {
                mapping: HashMap::new(),
                vfs: HashMap::new(),
            };
            LocalSheetData::write_to(&sheet_data, local_sheet_path).await?;
            return Ok(LocalSheet {
                local_workspace: self,
                member: member.clone(),
                sheet_name: sheet.clone(),
                data: sheet_data,
            });
        }

        let data = LocalSheetData::read_from(&local_sheet_path).await?;
        let local_sheet = LocalSheet {
            local_workspace: self,
            member: member.clone(),
            sheet_name: sheet.clone(),
            data,
        };

        Ok(local_sheet)
    }

    /// Collect all theet names
    pub async fn local_sheet_paths(&self) -> Result<Vec<LocalSheetPathBuf>, std::io::Error> {
        let local_sheet_path = self.local_path.join(CLIENT_PATH_LOCAL_SHEET);
        let mut sheet_paths = Vec::new();

        async fn collect_sheet_paths(
            dir: &Path,
            suffix: &str,
            paths: &mut Vec<LocalSheetPathBuf>,
        ) -> Result<(), std::io::Error> {
            if dir.is_dir() {
                let mut entries = fs::read_dir(dir).await?;
                while let Some(entry) = entries.next_entry().await? {
                    let path = entry.path();

                    if path.is_dir() {
                        Box::pin(collect_sheet_paths(&path, suffix, paths)).await?;
                    } else if path.is_file() {
                        if let Some(extension) = path.extension() {
                            if extension == suffix.trim_start_matches('.') {
                                let formatted_path = format_path(path)?;
                                paths.push(formatted_path);
                            }
                        }
                    }
                }
            }
            Ok(())
        }

        collect_sheet_paths(
            &local_sheet_path,
            CLIENT_SUFFIX_LOCAL_SHEET_FILE,
            &mut sheet_paths,
        )
        .await?;
        Ok(sheet_paths)
    }
}

mod hide_folder {
    use std::io;
    use std::path::Path;

    #[cfg(windows)]
    use std::os::windows::ffi::OsStrExt;
    #[cfg(windows)]
    use winapi::um::fileapi::{GetFileAttributesW, INVALID_FILE_ATTRIBUTES, SetFileAttributesW};

    pub fn hide_folder(path: &Path) -> io::Result<()> {
        if !path.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Path must be a directory",
            ));
        }

        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            if !file_name.starts_with('.') {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Directory name must start with '.'",
                ));
            }
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid directory name",
            ));
        }

        hide_folder_impl(path)
    }

    #[cfg(windows)]
    fn hide_folder_impl(path: &Path) -> io::Result<()> {
        // Convert to Windows wide string format
        let path_str: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();

        // Get current attributes
        let attrs = unsafe { GetFileAttributesW(path_str.as_ptr()) };
        if attrs == INVALID_FILE_ATTRIBUTES {
            return Err(io::Error::last_os_error());
        }

        // Add hidden attribute flag
        let new_attrs = attrs | winapi::um::winnt::FILE_ATTRIBUTE_HIDDEN;

        // Set new attributes
        let success = unsafe { SetFileAttributesW(path_str.as_ptr(), new_attrs) };
        if success == 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    #[cfg(unix)]
    fn hide_folder_impl(_path: &Path) -> io::Result<()> {
        Ok(())
    }

    #[cfg(not(any(windows, unix)))]
    fn hide_folder_impl(_path: &Path) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Unsupported operating system",
        ))
    }
}
