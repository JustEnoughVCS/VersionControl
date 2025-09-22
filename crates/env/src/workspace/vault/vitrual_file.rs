use std::{
    collections::HashMap,
    io::{Error, ErrorKind},
    path::PathBuf,
};

use cfg_file::{ConfigFile, config::ConfigFile};
use serde::{Deserialize, Serialize};
use tcp_connection::instance::ConnectionInstance;
use tokio::fs;
use uuid::Uuid;

use crate::{
    constants::{
        SERVER_FILE_VIRTUAL_FILE_META, SERVER_FILE_VIRTUAL_FILE_VERSION_INSTANCE,
        SERVER_PATH_VIRTUAL_FILE_ROOT, SERVER_PATH_VIRTUAL_FILE_STORAGE,
        SERVER_PATH_VIRTUAL_FILE_TEMP,
    },
    workspace::vault::{MemberId, Vault},
};

pub type VirtualFileId = String;
pub type VirtualFileVersion = String;

const ID_PARAM: &str = "{vf_id}";
const VERSION_PARAM: &str = "{vf_version}";
const TEMP_NAME: &str = "{temp_name}";

pub struct VirtualFile {
    /// Unique identifier for the virtual file
    id: VirtualFileId,

    /// Versions of the virtual file
    versions: Vec<VirtualFileVersion>,
}

#[derive(Default, Clone, Serialize, Deserialize, ConfigFile)]
pub struct VirtualFileMeta {
    /// Current version of the virtual file
    current_version: VirtualFileVersion,

    /// The member who holds the edit right of the file
    hold_member: MemberId,

    /// Description of each version
    version_description: HashMap<VirtualFileVersion, VirtualFileVersionDesciption>,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct VirtualFileVersionDesciption {
    /// The member who created this version
    pub creator: MemberId,

    /// The description of this version
    pub description: String,
}

impl VirtualFileVersionDesciption {
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
            .join(SERVER_PATH_VIRTUAL_FILE_TEMP.replace(TEMP_NAME, &random_receive_name))
    }

    /// Get the directory where virtual files are stored
    pub fn virtual_file_storage_dir(&self) -> PathBuf {
        self.vault_path().join(SERVER_PATH_VIRTUAL_FILE_ROOT)
    }

    /// Get the directory where a specific virtual file is stored
    pub fn virtual_file_dir(&self, id: VirtualFileId) -> PathBuf {
        self.vault_path()
            .join(SERVER_PATH_VIRTUAL_FILE_STORAGE.replace(ID_PARAM, &id.to_string()))
    }

    /// Get the directory where a specific virtual file's metadata is stored
    pub fn virtual_file_real_path(
        &self,
        id: VirtualFileId,
        version: VirtualFileVersion,
    ) -> PathBuf {
        self.vault_path().join(
            SERVER_FILE_VIRTUAL_FILE_VERSION_INSTANCE
                .replace(ID_PARAM, &id.to_string())
                .replace(VERSION_PARAM, &version.to_string()),
        )
    }

    /// Get the directory where a specific virtual file's metadata is stored
    pub fn virtual_file_meta_path(&self, id: VirtualFileId) -> PathBuf {
        self.vault_path()
            .join(SERVER_FILE_VIRTUAL_FILE_META.replace(ID_PARAM, &id.to_string()))
    }

    /// Get the virtual file with the given ID
    pub fn virtual_file(&self, id: VirtualFileId) -> Option<VirtualFile> {
        let dir = self.virtual_file_dir(id.clone());
        if dir.exists() {
            Some(VirtualFile {
                id,
                versions: std::fs::read_dir(&dir)
                    .ok()?
                    .filter_map(|entry| {
                        let entry = entry.ok()?;
                        let path = entry.path();
                        if path.is_file() && path.extension()?.to_str()? == "rf" {
                            path.file_stem()?.to_str().map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect(),
            })
        } else {
            None
        }
    }

    /// Get the meta data of the virtual file with the given ID
    pub async fn virtual_file_meta(
        &self,
        id: VirtualFileId,
    ) -> Result<VirtualFileMeta, std::io::Error> {
        let dir = self.virtual_file_meta_path(id);
        let metadata = VirtualFileMeta::read_from(dir).await?;
        Ok(metadata)
    }

    /// Write the meta data of the virtual file with the given ID
    pub async fn write_virtual_file_meta(
        &self,
        id: VirtualFileId,
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
        member: MemberId,
    ) -> Result<VirtualFileId, std::io::Error> {
        const FIRST_VERSION: &str = "0";
        let receive_path = self.virtual_file_temp_path();
        let new_id = format!("vf_{}", Uuid::new_v4());
        let move_path = self.virtual_file_real_path(new_id.clone(), FIRST_VERSION.to_string());

        match instance.read_file(receive_path.clone()).await {
            Ok(_) => {
                // Read successful, create virtual file
                // Create default version description
                let mut version_description =
                    HashMap::<VirtualFileVersion, VirtualFileVersionDesciption>::new();
                version_description.insert(
                    FIRST_VERSION.to_string(),
                    VirtualFileVersionDesciption {
                        creator: member,
                        description: "Track".to_string(),
                    },
                );
                // Create metadata
                let meta = VirtualFileMeta {
                    current_version: FIRST_VERSION.to_string(),
                    hold_member: String::default(),
                    version_description,
                };
                // Write metadata to file
                VirtualFileMeta::write_to(&meta, self.virtual_file_meta_path(new_id.clone()))
                    .await?;

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
        member: MemberId,
        virtual_file_id: VirtualFileId,
        new_version: VirtualFileVersion,
        description: VirtualFileVersionDesciption,
    ) -> Result<(), std::io::Error> {
        // Check if the member has edit right
        let mut meta = self.virtual_file_meta(virtual_file_id.clone()).await?;
        if !meta.hold_member.eq(&member) {
            return Err(Error::new(
                ErrorKind::PermissionDenied,
                format!(
                    "Member `{}` not allowed to update virtual file `{}`",
                    member, virtual_file_id
                ),
            ));
        }

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
        let move_path = self.virtual_file_real_path(virtual_file_id.clone(), new_version.clone());

        match instance.read_file(receive_path.clone()).await {
            Ok(_) => {
                // Read success, move temp file to real path.
                fs::rename(receive_path, move_path).await?;

                // Update metadata
                meta.version_description.insert(new_version, description);
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

    /// Grant a member the edit right for a virtual file
    pub async fn grant_virtual_file_edit_right(
        &self,
        member_id: MemberId,
        virtual_file_id: VirtualFileId,
    ) -> Result<(), std::io::Error> {
        let mut meta = self.virtual_file_meta(virtual_file_id.clone()).await?;
        meta.hold_member = member_id;
        self.write_virtual_file_meta(virtual_file_id, &meta).await
    }

    /// Check if a member has the edit right for a virtual file
    pub async fn has_virtual_file_edit_right(
        &self,
        member_id: MemberId,
        virtual_file_id: VirtualFileId,
    ) -> Result<bool, std::io::Error> {
        let meta = self.virtual_file_meta(virtual_file_id).await?;
        Ok(meta.hold_member == member_id)
    }

    /// Revoke the edit right for a virtual file from the current holder
    pub async fn revoke_virtual_file_edit_right(
        &self,
        virtual_file_id: VirtualFileId,
    ) -> Result<(), std::io::Error> {
        let mut meta = self.virtual_file_meta(virtual_file_id.clone()).await?;
        meta.hold_member = String::default();
        self.write_virtual_file_meta(virtual_file_id, &meta).await
    }
}
