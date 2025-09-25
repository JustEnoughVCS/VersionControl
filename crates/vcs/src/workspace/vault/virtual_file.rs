use std::{
    collections::HashMap,
    io::{Error, ErrorKind},
    path::PathBuf,
};

use cfg_file::{ConfigFile, config::ConfigFile};
use serde::{Deserialize, Serialize};
use string_proc::snake_case;
use tcp_connection::instance::ConnectionInstance;
use tokio::fs;
use uuid::Uuid;

use crate::{
    constants::{
        SERVER_FILE_VF_META, SERVER_FILE_VF_VERSION_INSTANCE, SERVER_PATH_VF_ROOT,
        SERVER_PATH_VF_STORAGE, SERVER_PATH_VF_TEMP,
    },
    workspace::vault::{MemberId, Vault},
};

pub type VirtualFileId = String;
pub type VirtualFileVersion = String;

const VF_PREFIX: &str = "vf_";
const ID_PARAM: &str = "{vf_id}";
const ID_INDEX: &str = "{vf_index}";
const VERSION_PARAM: &str = "{vf_version}";
const TEMP_NAME: &str = "{temp_name}";

pub struct VirtualFile<'a> {
    /// Unique identifier for the virtual file
    id: VirtualFileId,

    /// Reference of Vault
    current_vault: &'a Vault,
}

#[derive(Default, Clone, Serialize, Deserialize, ConfigFile)]
pub struct VirtualFileMeta {
    /// Current version of the virtual file
    current_version: VirtualFileVersion,

    /// The member who holds the edit right of the file
    hold_member: MemberId,

    /// Description of each version
    version_description: HashMap<VirtualFileVersion, VirtualFileVersionDescription>,

    /// Histories
    histories: Vec<VirtualFileVersion>,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct VirtualFileVersionDescription {
    /// The member who created this version
    pub creator: MemberId,

    /// The description of this version
    pub description: String,
}

impl VirtualFileVersionDescription {
    /// Create a new version description
    pub fn new(creator: MemberId, description: String) -> Self {
        Self {
            creator,
            description,
        }
    }
}

impl Vault {
    /// Generate a temporary path for receiving
    pub fn virtual_file_temp_path(&self) -> PathBuf {
        let random_receive_name = format!("{}", uuid::Uuid::new_v4());
        self.vault_path
            .join(SERVER_PATH_VF_TEMP.replace(TEMP_NAME, &random_receive_name))
    }

    /// Get the directory where virtual files are stored
    pub fn virtual_file_storage_dir(&self) -> PathBuf {
        self.vault_path().join(SERVER_PATH_VF_ROOT)
    }

    /// Get the directory where a specific virtual file is stored
    pub fn virtual_file_dir(&self, id: &VirtualFileId) -> Result<PathBuf, std::io::Error> {
        Ok(self.vault_path().join(
            SERVER_PATH_VF_STORAGE
                .replace(ID_PARAM, &id.to_string())
                .replace(ID_INDEX, &Self::vf_index(id)?),
        ))
    }

    // Generate index path of virtual file
    fn vf_index(id: &VirtualFileId) -> Result<String, std::io::Error> {
        // Remove VF_PREFIX if present
        let id_str = if id.starts_with(VF_PREFIX) {
            &id[VF_PREFIX.len()..]
        } else {
            id
        };

        // Extract the first part before the first hyphen
        let first_part = id_str.split('-').next().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid virtual file ID format: no hyphen found",
            )
        })?;

        // Ensure the first part has exactly 8 characters
        if first_part.len() != 8 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid virtual file ID format: first part must be 8 characters",
            ))?;
        }

        // Split into 2-character chunks and join with path separator
        let mut path = String::new();
        for i in (0..first_part.len()).step_by(2) {
            if i > 0 {
                path.push('/');
            }
            path.push_str(&first_part[i..i + 2]);
        }

        Ok(path)
    }

    /// Get the directory where a specific virtual file's metadata is stored
    pub fn virtual_file_real_path(
        &self,
        id: &VirtualFileId,
        version: &VirtualFileVersion,
    ) -> PathBuf {
        self.vault_path().join(
            SERVER_FILE_VF_VERSION_INSTANCE
                .replace(ID_PARAM, &id.to_string())
                .replace(ID_INDEX, &version.to_string()),
        )
    }

    /// Get the directory where a specific virtual file's metadata is stored
    pub fn virtual_file_meta_path(&self, id: &VirtualFileId) -> PathBuf {
        self.vault_path()
            .join(SERVER_FILE_VF_META.replace(ID_PARAM, &id.to_string()))
    }

    /// Get the virtual file with the given ID
    pub fn virtual_file(&self, id: &VirtualFileId) -> Result<VirtualFile<'_>, std::io::Error> {
        let dir = self.virtual_file_dir(id);
        if dir?.exists() {
            Ok(VirtualFile {
                id: id.clone(),
                current_vault: self,
            })
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Cannot found virtual file!",
            ))
        }
    }

    /// Get the meta data of the virtual file with the given ID
    pub async fn virtual_file_meta(
        &self,
        id: &VirtualFileId,
    ) -> Result<VirtualFileMeta, std::io::Error> {
        let dir = self.virtual_file_meta_path(id);
        let metadata = VirtualFileMeta::read_from(dir).await?;
        Ok(metadata)
    }

    /// Write the meta data of the virtual file with the given ID
    pub async fn write_virtual_file_meta(
        &self,
        id: &VirtualFileId,
        meta: &VirtualFileMeta,
    ) -> Result<(), std::io::Error> {
        let dir = self.virtual_file_meta_path(id);
        VirtualFileMeta::write_to(meta, dir).await?;
        Ok(())
    }

    /// Create a virtual file from a connection instance
    ///
    /// It's the only way to create virtual files!
    ///
    /// When the target machine executes `write_file`, use this function instead of `read_file`,
    ///    and provide the member ID of the transmitting member.
    ///
    /// The system will automatically receive the file and
    ///    create the virtual file.
    pub async fn create_virtual_file_from_connection(
        &self,
        instance: &mut ConnectionInstance,
        member_id: &MemberId,
    ) -> Result<VirtualFileId, std::io::Error> {
        const FIRST_VERSION: &str = "0";
        let receive_path = self.virtual_file_temp_path();
        let new_id = format!("{}{}", VF_PREFIX, Uuid::new_v4());
        let move_path = self.virtual_file_real_path(&new_id, &FIRST_VERSION.to_string());

        match instance.read_file(receive_path.clone()).await {
            Ok(_) => {
                // Read successful, create virtual file
                // Create default version description
                let mut version_description =
                    HashMap::<VirtualFileVersion, VirtualFileVersionDescription>::new();
                version_description.insert(
                    FIRST_VERSION.to_string(),
                    VirtualFileVersionDescription {
                        creator: member_id.clone(),
                        description: "Track".to_string(),
                    },
                );
                // Create metadata
                let mut meta = VirtualFileMeta {
                    current_version: FIRST_VERSION.to_string(),
                    hold_member: String::default(),
                    version_description,
                    histories: Vec::default(),
                };

                // Add first version
                meta.histories.push(FIRST_VERSION.to_string());

                // Write metadata to file
                VirtualFileMeta::write_to(&meta, self.virtual_file_meta_path(&new_id)).await?;

                // Move temp file to virtual file directory
                if let Some(parent) = move_path.parent() {
                    if !parent.exists() {
                        fs::create_dir_all(parent).await?;
                    }
                }
                fs::rename(receive_path, move_path).await?;

                Ok(new_id)
            }
            Err(e) => {
                // Read failed, remove temp file.
                if receive_path.exists() {
                    fs::remove_file(receive_path).await?;
                }

                Err(Error::new(ErrorKind::Other, e))
            }
        }
    }

    /// Update a virtual file from a connection instance
    ///
    /// It's the only way to update virtual files!
    /// When the target machine executes `write_file`, use this function instead of `read_file`,
    ///    and provide the member ID of the transmitting member.
    ///
    /// The system will automatically receive the file and
    ///    update the virtual file.
    ///
    /// Note: The specified member must hold the edit right of the file,
    ///    otherwise the file reception will not be allowed.
    ///
    /// Make sure to obtain the edit right of the file before calling this function.
    pub async fn update_virtual_file_from_connection(
        &self,
        instance: &mut ConnectionInstance,
        member: &MemberId,
        virtual_file_id: &VirtualFileId,
        new_version: &VirtualFileVersion,
        description: VirtualFileVersionDescription,
    ) -> Result<(), std::io::Error> {
        let new_version = snake_case!(new_version.clone());
        let mut meta = self.virtual_file_meta(virtual_file_id).await?;

        // Check if the member has edit right
        self.check_virtual_file_edit_right(member, virtual_file_id)
            .await?;

        // Check if the new version already exists
        if meta.version_description.contains_key(&new_version) {
            return Err(Error::new(
                ErrorKind::AlreadyExists,
                format!(
                    "Version `{}` already exists for virtual file `{}`",
                    new_version, virtual_file_id
                ),
            ));
        }

        // Verify success
        let receive_path = self.virtual_file_temp_path();
        let move_path = self.virtual_file_real_path(virtual_file_id, &new_version);

        match instance.read_file(receive_path.clone()).await {
            Ok(_) => {
                // Read success, move temp file to real path.
                fs::rename(receive_path, move_path).await?;

                // Update metadata
                meta.current_version = new_version.clone();
                meta.version_description
                    .insert(new_version.clone(), description);
                meta.histories.push(new_version);
                VirtualFileMeta::write_to(&meta, self.virtual_file_meta_path(virtual_file_id))
                    .await?;

                return Ok(());
            }
            Err(e) => {
                // Read failed, remove temp file.
                if receive_path.exists() {
                    fs::remove_file(receive_path).await?;
                }

                return Err(Error::new(ErrorKind::Other, e));
            }
        }
    }

    /// Update virtual file from existing version
    ///
    /// This operation creates a new version based on the specified old version file instance.
    /// The new version will retain the same version name as the old version, but use a different version number.
    /// After the update, this version will be considered newer than the original version when comparing versions.
    pub async fn update_virtual_file_from_exist_version(
        &self,
        member: &MemberId,
        virtual_file_id: &VirtualFileId,
        old_version: &VirtualFileVersion,
    ) -> Result<(), std::io::Error> {
        let old_version = snake_case!(old_version.clone());
        let mut meta = self.virtual_file_meta(virtual_file_id).await?;

        // Check if the member has edit right
        self.check_virtual_file_edit_right(member, virtual_file_id)
            .await?;

        // Ensure virtual file exist
        let Ok(_) = self.virtual_file(virtual_file_id) else {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Virtual file `{}` not found!", virtual_file_id),
            ));
        };

        // Ensure version exist
        if !meta.version_exists(&old_version) {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Version `{}` not found!", old_version),
            ));
        }

        // Ok, Create new version
        meta.current_version = old_version.clone();
        meta.histories.push(old_version);
        VirtualFileMeta::write_to(&meta, self.virtual_file_meta_path(virtual_file_id)).await?;

        Ok(())
    }

    /// Grant a member the edit right for a virtual file
    /// This operation takes effect immediately upon success
    pub async fn grant_virtual_file_edit_right(
        &self,
        member_id: &MemberId,
        virtual_file_id: &VirtualFileId,
    ) -> Result<(), std::io::Error> {
        let mut meta = self.virtual_file_meta(virtual_file_id).await?;
        meta.hold_member = member_id.clone();
        self.write_virtual_file_meta(virtual_file_id, &meta).await
    }

    /// Check if a member has the edit right for a virtual file
    pub async fn has_virtual_file_edit_right(
        &self,
        member_id: &MemberId,
        virtual_file_id: &VirtualFileId,
    ) -> Result<bool, std::io::Error> {
        let meta = self.virtual_file_meta(virtual_file_id).await?;
        Ok(meta.hold_member.eq(member_id))
    }

    /// Check if a member has the edit right for a virtual file and return Result
    /// Returns Ok(()) if the member has edit right, otherwise returns PermissionDenied error
    pub async fn check_virtual_file_edit_right(
        &self,
        member_id: &MemberId,
        virtual_file_id: &VirtualFileId,
    ) -> Result<(), std::io::Error> {
        if !self
            .has_virtual_file_edit_right(member_id, virtual_file_id)
            .await?
        {
            return Err(Error::new(
                ErrorKind::PermissionDenied,
                format!(
                    "Member `{}` not allowed to update virtual file `{}`",
                    member_id, virtual_file_id
                ),
            ));
        }
        Ok(())
    }

    /// Revoke the edit right for a virtual file from the current holder
    /// This operation takes effect immediately upon success
    pub async fn revoke_virtual_file_edit_right(
        &self,
        virtual_file_id: &VirtualFileId,
    ) -> Result<(), std::io::Error> {
        let mut meta = self.virtual_file_meta(virtual_file_id).await?;
        meta.hold_member = String::default();
        self.write_virtual_file_meta(virtual_file_id, &meta).await
    }
}

impl<'a> VirtualFile<'a> {
    /// Get id of VirtualFile
    pub fn id(&self) -> VirtualFileId {
        self.id.clone()
    }

    /// Read metadata of VirtualFile
    pub async fn read_meta(&self) -> Result<VirtualFileMeta, std::io::Error> {
        self.current_vault.virtual_file_meta(&self.id).await
    }
}

impl VirtualFileMeta {
    /// Get all versions of the virtual file
    pub fn versions(&self) -> &Vec<VirtualFileVersion> {
        &self.histories
    }

    /// Get the total number of versions for this virtual file
    pub fn version_len(&self) -> i32 {
        self.histories.len() as i32
    }

    /// Check if a specific version exists
    /// Returns true if the version exists, false otherwise
    pub fn version_exists(&self, version: &VirtualFileVersion) -> bool {
        self.versions().iter().any(|v| v == version)
    }

    /// Get the version number (index) for a given version name
    /// Returns None if the version doesn't exist
    pub fn version_num(&self, version: &VirtualFileVersion) -> Option<i32> {
        self.histories
            .iter()
            .rev()
            .position(|v| v == version)
            .map(|pos| (self.histories.len() - 1 - pos) as i32)
    }

    /// Get the version name for a given version number (index)
    /// Returns None if the version number is out of range
    pub fn version_name(&self, version_num: i32) -> Option<VirtualFileVersion> {
        self.histories.get(version_num as usize).cloned()
    }
}
