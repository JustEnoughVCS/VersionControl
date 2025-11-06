use std::{env::current_dir, path::PathBuf, sync::Arc};

use cfg_file::config::ConfigFile;
use tokio::{fs, sync::Mutex};
use vcs_docs::docs::READMES_LOCAL_WORKSPACE_TODOLIST;

use crate::{
    constants::{CLIENT_FILE_TODOLIST, CLIENT_FILE_WORKSPACE},
    current::{current_local_path, find_local_path},
    data::local::config::LocalConfig,
};

pub mod cached_sheet;
pub mod config;
pub mod latest_info;
pub mod local_sheet;
pub mod member_held;

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

        // 2. Setup README.md
        let readme_content = READMES_LOCAL_WORKSPACE_TODOLIST.trim().to_string();
        fs::write(local_path.join(CLIENT_FILE_TODOLIST), readme_content).await?;

        // On Windows, set the .jv directory as hidden
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::fs::MetadataExt;
            use winapi_util::file::set_hidden;

            let jv_dir = local_path.join(".jv");
            if jv_dir.exists() {
                if let Err(e) = set_hidden(&jv_dir, true) {
                    eprintln!("Warning: Failed to set .jv directory as hidden: {}", e);
                }
            }
        }

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
}
