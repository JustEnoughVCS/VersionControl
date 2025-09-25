use std::{env::current_dir, path::PathBuf};

use cfg_file::config::ConfigFile;
use tokio::fs;

use crate::{
    constants::{CLIENT_FILE_README, CLIENT_FILE_WORKSPACE},
    current::{current_local_path, find_local_path},
    workspace::local::config::LocalConfig,
};

pub mod config;

pub struct LocalWorkspace {
    config: LocalConfig,
    local_path: PathBuf,
}

impl LocalWorkspace {
    /// Get the path of the local workspace.
    pub fn local_path(&self) -> &PathBuf {
        &self.local_path
    }

    /// Initialize local workspace.
    pub fn init(config: LocalConfig, local_path: impl Into<PathBuf>) -> Option<Self> {
        let Some(local_path) = find_local_path(local_path) else {
            return None;
        };
        Some(Self { config, local_path })
    }

    /// Initialize local workspace in the current directory.
    pub fn init_current_dir(config: LocalConfig) -> Option<Self> {
        let Some(local_path) = current_local_path() else {
            return None;
        };
        Some(Self { config, local_path })
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
        let readme_content = "\
# JustEnoughVCS Local Workspace

This directory is a **Local Workspace** managed by `JustEnoughVCS`. All files and subdirectories within this scope can be version-controlled using the `JustEnoughVCS` CLI or GUI tools, with the following exceptions:

- The `.jv` directory
- Any files or directories excluded via `.jgnore` or `.gitignore`

> ⚠️ **Warning**
>
> Files in this workspace will be uploaded to the upstream server. Please ensure you fully trust this server before proceeding.

## Access Requirements

To use `JustEnoughVCS` with this workspace, you must have:

- **A registered user ID** with the upstream server
- **Your private key** properly configured locally
- **Your public key** stored in the server's public key directory

Without these credentials, the server will reject all access requests.

## Support

- **Permission or access issues?** → Contact your server administrator
- **Tooling problems or bugs?** → Reach out to the development team via [GitHub Issues](https://github.com/JustEnoughVCS/VersionControl/issues)
- **Documentation**: Visit our repository for full documentation

------

*Thank you for using JustEnoughVCS!*
".to_string()
        .trim()
        .to_string();
        fs::write(local_path.join(CLIENT_FILE_README), readme_content).await?;

        Ok(())
    }

    /// Setup local workspace in current directory
    pub async fn setup_local_workspacecurrent_dir() -> Result<(), std::io::Error> {
        Self::setup_local_workspace(current_dir()?).await?;
        Ok(())
    }
}
