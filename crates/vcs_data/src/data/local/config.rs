use cfg_file::ConfigFile;
use cfg_file::config::ConfigFile;
use serde::{Deserialize, Serialize};
use std::io::Error;
use std::net::SocketAddr;
use std::path::Path;
use std::path::PathBuf;
use string_proc::snake_case;

use crate::constants::CLIENT_FILE_WORKSPACE;
use crate::constants::CLIENT_FOLDER_WORKSPACE_ROOT_NAME;
use crate::constants::CLIENT_PATH_LOCAL_DRAFT;
use crate::constants::CLIENT_PATH_WORKSPACE_ROOT;
use crate::constants::PORT;
use crate::current::current_local_path;
use crate::data::local::latest_info::LatestInfo;
use crate::data::member::MemberId;
use crate::data::sheet::SheetName;
use crate::data::vault::config::VaultUuid;

const ACCOUNT: &str = "{account}";
const SHEET_NAME: &str = "{sheet_name}";

#[derive(Serialize, Deserialize, ConfigFile, Clone)]
#[cfg_file(path = CLIENT_FILE_WORKSPACE)]
pub struct LocalConfig {
    /// The upstream address, representing the upstream address of the local workspace,
    /// to facilitate timely retrieval of new updates from the upstream source.
    #[serde(rename = "addr")]
    upstream_addr: SocketAddr,

    /// The member ID used by the current local workspace.
    /// This ID will be used to verify access permissions when connecting to the upstream server.
    #[serde(rename = "as")]
    using_account: MemberId,

    /// Whether the current member is interacting as a host.
    /// In host mode, full Vault operation permissions are available except for adding new content.
    #[serde(rename = "host")]
    using_host_mode: bool,

    /// Whether the local workspace is stained.
    ///
    /// If stained, it can only set an upstream server with the same identifier.
    ///
    /// If the value is None, it means not stained;
    /// otherwise, it contains the stain identifier (i.e., the upstream vault's unique ID)
    #[serde(rename = "up_uid")]
    stained_uuid: Option<VaultUuid>,

    /// The name of the sheet currently in use.
    #[serde(rename = "use")]
    sheet_in_use: Option<SheetName>,
}

impl Default for LocalConfig {
    fn default() -> Self {
        Self {
            upstream_addr: SocketAddr::V4(std::net::SocketAddrV4::new(
                std::net::Ipv4Addr::new(127, 0, 0, 1),
                PORT,
            )),
            using_account: "unknown".to_string(),
            using_host_mode: false,
            stained_uuid: None,
            sheet_in_use: None,
        }
    }
}

impl LocalConfig {
    /// Set the vault address.
    pub fn set_vault_addr(&mut self, addr: SocketAddr) {
        self.upstream_addr = addr;
    }

    /// Get the vault address.
    pub fn vault_addr(&self) -> SocketAddr {
        self.upstream_addr
    }

    /// Set the currently used account
    pub fn set_current_account(&mut self, account: MemberId) -> Result<(), std::io::Error> {
        if self.sheet_in_use().is_some() {
            return Err(Error::new(
                std::io::ErrorKind::DirectoryNotEmpty,
                "Please exit the current sheet before switching accounts",
            ));
        }
        self.using_account = account;
        Ok(())
    }

    /// Set the host mode
    pub fn set_host_mode(&mut self, host_mode: bool) {
        self.using_host_mode = host_mode;
    }

    /// Set the currently used sheet
    pub async fn use_sheet(&mut self, sheet: SheetName) -> Result<(), std::io::Error> {
        let sheet = snake_case!(sheet);

        // Check if the sheet is already in use
        if self.sheet_in_use().is_some() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "Sheet already in use",
            ));
        };

        // Check if the local path exists
        let local_path = self.get_local_path().await?;

        // Get latest info
        let Ok(latest_info) = LatestInfo::read_from(LatestInfo::latest_info_path(
            &local_path,
            &self.current_account(),
        ))
        .await
        else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No latest info found",
            ));
        };

        // Check if the sheet exists
        if !latest_info.visible_sheets.contains(&sheet) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Sheet not found",
            ));
        }

        // Check if there are any files or folders other than .jv
        self.check_local_path_empty(&local_path).await?;

        // Get the draft folder path
        let draft_folder = self.draft_folder(&self.using_account, &sheet, &local_path);

        if draft_folder.exists() {
            // Exists
            // Move the contents of the draft folder to the local path with rollback support
            self.move_draft_to_local(&draft_folder, &local_path).await?;
        }

        self.sheet_in_use = Some(sheet);
        LocalConfig::write(self).await?;

        Ok(())
    }

    /// Exit the currently used sheet
    pub async fn exit_sheet(&mut self) -> Result<(), std::io::Error> {
        // Check if the sheet is already in use
        if self.sheet_in_use().is_none() {
            return Ok(());
        }

        // Check if the local path exists
        let local_path = self.get_local_path().await?;

        // Get the current sheet name
        let sheet_name = self.sheet_in_use().as_ref().unwrap().clone();

        // Get the draft folder path
        let draft_folder = self.draft_folder(&self.using_account, &sheet_name, &local_path);

        // Create the draft folder if it doesn't exist
        if !draft_folder.exists() {
            std::fs::create_dir_all(&draft_folder).map_err(std::io::Error::other)?;
        }

        // Move all files and folders (except .jv folder) to the draft folder with rollback support
        self.move_local_to_draft(&local_path, &draft_folder).await?;

        // Clear the sheet in use
        self.sheet_in_use = None;
        LocalConfig::write(self).await?;

        Ok(())
    }

    /// Get local path or return error
    async fn get_local_path(&self) -> Result<PathBuf, std::io::Error> {
        current_local_path().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Fail to get local path")
        })
    }

    /// Check if local path is empty (except for .jv folder)
    async fn check_local_path_empty(&self, local_path: &Path) -> Result<(), std::io::Error> {
        let jv_folder = local_path.join(CLIENT_PATH_WORKSPACE_ROOT);
        let mut entries = std::fs::read_dir(local_path).map_err(std::io::Error::other)?;

        if entries.any(|entry| {
            if let Ok(entry) = entry {
                let path = entry.path();
                path != jv_folder
                    && path.file_name().and_then(|s| s.to_str())
                        != Some(CLIENT_FOLDER_WORKSPACE_ROOT_NAME)
            } else {
                false
            }
        }) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::DirectoryNotEmpty,
                "Local path is not empty!",
            ));
        }

        Ok(())
    }

    /// Move contents from draft folder to local path with rollback support
    async fn move_draft_to_local(
        &self,
        draft_folder: &Path,
        local_path: &Path,
    ) -> Result<(), std::io::Error> {
        let draft_entries: Vec<_> = std::fs::read_dir(draft_folder)
            .map_err(std::io::Error::other)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(std::io::Error::other)?;

        let mut moved_items: Vec<MovedItem> = Vec::new();

        for entry in &draft_entries {
            let entry_path = entry.path();
            let target_path = local_path.join(entry_path.file_name().unwrap());

            // Move each file/directory from draft folder to local path
            std::fs::rename(&entry_path, &target_path).map_err(|e| {
                // Rollback all previously moved items
                for moved_item in &moved_items {
                    let _ = std::fs::rename(&moved_item.target, &moved_item.source);
                }
                std::io::Error::other(e)
            })?;

            moved_items.push(MovedItem {
                source: entry_path.clone(),
                target: target_path.clone(),
            });
        }

        // Remove the now-empty draft folder
        std::fs::remove_dir(draft_folder).map_err(|e| {
            // Rollback all moved items if folder removal fails
            for moved_item in &moved_items {
                let _ = std::fs::rename(&moved_item.target, &moved_item.source);
            }
            std::io::Error::other(e)
        })?;

        Ok(())
    }

    /// Move contents from local path to draft folder with rollback support (except .jv folder)
    async fn move_local_to_draft(
        &self,
        local_path: &Path,
        draft_folder: &Path,
    ) -> Result<(), std::io::Error> {
        let jv_folder = local_path.join(CLIENT_PATH_WORKSPACE_ROOT);
        let entries: Vec<_> = std::fs::read_dir(local_path)
            .map_err(std::io::Error::other)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(std::io::Error::other)?;

        let mut moved_items: Vec<MovedItem> = Vec::new();

        for entry in &entries {
            let entry_path = entry.path();

            // Skip the .jv folder
            if entry_path == jv_folder
                || entry_path.file_name().and_then(|s| s.to_str())
                    == Some(CLIENT_FOLDER_WORKSPACE_ROOT_NAME)
            {
                continue;
            }

            let target_path = draft_folder.join(entry_path.file_name().unwrap());

            // Move each file/directory from local path to draft folder
            std::fs::rename(&entry_path, &target_path).map_err(|e| {
                // Rollback all previously moved items
                for moved_item in &moved_items {
                    let _ = std::fs::rename(&moved_item.target, &moved_item.source);
                }
                std::io::Error::other(e)
            })?;

            moved_items.push(MovedItem {
                source: entry_path.clone(),
                target: target_path.clone(),
            });
        }

        Ok(())
    }

    /// Get the currently used account
    pub fn current_account(&self) -> MemberId {
        self.using_account.clone()
    }

    /// Check if the current member is interacting as a host.
    pub fn is_host_mode(&self) -> bool {
        self.using_host_mode
    }

    /// Check if the local workspace is stained.
    pub fn stained(&self) -> bool {
        self.stained_uuid.is_some()
    }

    /// Get the UUID of the vault that the local workspace is stained with.
    pub fn stained_uuid(&self) -> Option<VaultUuid> {
        self.stained_uuid
    }

    /// Stain the local workspace with the given UUID.
    pub fn stain(&mut self, uuid: VaultUuid) {
        self.stained_uuid = Some(uuid);
    }

    /// Unstain the local workspace.
    pub fn unstain(&mut self) {
        self.stained_uuid = None;
    }

    /// Get the upstream address.
    pub fn upstream_addr(&self) -> SocketAddr {
        self.upstream_addr
    }

    /// Get the currently used sheet
    pub fn sheet_in_use(&self) -> &Option<SheetName> {
        &self.sheet_in_use
    }

    /// Get draft folder
    pub fn draft_folder(
        &self,
        account: &MemberId,
        sheet_name: &SheetName,
        local_workspace_path: impl Into<PathBuf>,
    ) -> PathBuf {
        let account_str = snake_case!(account.as_str());
        let sheet_name_str = snake_case!(sheet_name.as_str());
        let draft_path = CLIENT_PATH_LOCAL_DRAFT
            .replace(ACCOUNT, &account_str)
            .replace(SHEET_NAME, &sheet_name_str);
        local_workspace_path.into().join(draft_path)
    }

    /// Get current draft folder
    pub fn current_draft_folder(&self) -> Option<PathBuf> {
        let Some(sheet_name) = self.sheet_in_use() else {
            return None;
        };

        let current_dir = current_local_path()?;

        Some(self.draft_folder(&self.using_account, sheet_name, current_dir))
    }
}

#[derive(Clone)]
struct MovedItem {
    source: PathBuf,
    target: PathBuf,
}
