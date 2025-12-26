use std::{collections::HashMap, io::Error, path::PathBuf};

use cfg_file::{ConfigFile, config::ConfigFile};
use rand::{Rng, rng};
use serde::{Deserialize, Serialize};
use string_proc::{format_path, snake_case};
use tokio::fs;

use crate::{
    constants::{SERVER_FILE_SHEET_SHARE, SERVER_PATH_SHARES, SERVER_SUFFIX_SHEET_FILE_NO_DOT},
    data::{
        member::MemberId,
        sheet::{Sheet, SheetMappingMetadata, SheetName, SheetPathBuf},
        vault::Vault,
    },
};

pub type SheetShareId = String;

const SHEET_NAME: &str = "{sheet_name}";
const SHARE_ID: &str = "{share_id}";

#[derive(Default, Serialize, Deserialize, ConfigFile, Clone, Debug)]
pub struct Share {
    /// Sharer: the member who created this share item
    pub sharer: MemberId,

    /// Description of the share item
    pub description: String,

    /// Metadata path
    #[serde(skip)]
    pub path: Option<PathBuf>,

    /// From: which sheet the member exported the file from
    pub from_sheet: SheetName,

    /// Mappings: the sheet mappings contained in the share item
    pub mappings: HashMap<SheetPathBuf, SheetMappingMetadata>,
}

#[derive(Default, Serialize, Deserialize, ConfigFile, Clone, PartialEq, Eq)]
pub enum ShareMergeMode {
    /// If a path or file already exists during merge, prioritize the incoming share
    /// Path conflict: replace the mapping content at the local path with the incoming content
    /// File conflict: delete the original file mapping and create a new one
    Overwrite,

    /// If a path or file already exists during merge, skip overwriting this entry
    Skip,

    /// Pre-check for conflicts, prohibit merging if any conflicts are found
    #[default]
    Safe,
}

#[derive(Default, Serialize, Deserialize, ConfigFile, Clone)]
pub struct ShareMergeConflict {
    /// Duplicate mappings exist
    pub duplicate_mapping: Vec<PathBuf>,

    /// Duplicate files exist
    pub duplicate_file: Vec<PathBuf>,
}

impl ShareMergeConflict {
    /// Check if there are no conflicts
    pub fn ok(&self) -> bool {
        self.duplicate_mapping.is_empty() && self.duplicate_file.is_empty()
    }
}

impl Vault {
    /// Get the path of a share item in a sheet
    pub fn share_file_path(&self, sheet_name: &SheetName, share_id: &SheetShareId) -> PathBuf {
        let sheet_name = snake_case!(sheet_name.clone());
        let share_id = share_id.clone();

        // Format the path to remove "./" prefix and normalize it
        let path_str = SERVER_FILE_SHEET_SHARE
            .replace(SHEET_NAME, &sheet_name)
            .replace(SHARE_ID, &share_id);

        // Use format_path to normalize the path
        match format_path::format_path_str(&path_str) {
            Ok(normalized_path) => self.vault_path().join(normalized_path),
            Err(_) => {
                // Fallback to original behavior if formatting fails
                self.vault_path().join(path_str)
            }
        }
    }

    /// Get the actual paths of all share items in a sheet
    pub async fn share_file_paths(&self, sheet_name: &SheetName) -> Vec<PathBuf> {
        let sheet_name = snake_case!(sheet_name.clone());
        let shares_dir = self
            .vault_path()
            .join(SERVER_PATH_SHARES.replace(SHEET_NAME, &sheet_name));

        let mut result = Vec::new();
        if let Ok(mut entries) = fs::read_dir(shares_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.is_file()
                    && path.extension().and_then(|s| s.to_str())
                        == Some(SERVER_SUFFIX_SHEET_FILE_NO_DOT)
                {
                    result.push(path);
                }
            }
        }
        result
    }
}

impl<'a> Sheet<'a> {
    /// Get the shares of a sheet
    pub async fn get_shares(&self) -> Result<Vec<Share>, std::io::Error> {
        let paths = self.vault_reference.share_file_paths(&self.name).await;
        let mut shares = Vec::new();

        for path in paths {
            match Share::read_from(&path).await {
                Ok(mut share) => {
                    share.path = Some(path);
                    shares.push(share);
                }
                Err(e) => return Err(e),
            }
        }

        Ok(shares)
    }

    /// Get a share of a sheet
    pub async fn get_share(&self, share_id: &SheetShareId) -> Result<Share, std::io::Error> {
        let path = self.vault_reference.share_file_path(&self.name, share_id);
        let mut share = Share::read_from(&path).await?;
        share.path = Some(path);
        Ok(share)
    }

    /// Import a share of a sheet by its ID
    pub async fn merge_share_by_id(
        self,
        share_id: &SheetShareId,
        share_merge_mode: ShareMergeMode,
    ) -> Result<(), std::io::Error> {
        let share = self.get_share(share_id).await?;
        self.merge_share(share, share_merge_mode).await
    }

    /// Import a share of a sheet
    pub async fn merge_share(
        mut self,
        share: Share,
        share_merge_mode: ShareMergeMode,
    ) -> Result<(), std::io::Error> {
        // Backup original data and edit based on this backup
        let mut copy_share = share.clone();
        let mut copy_sheet = self.clone_data();

        // Pre-check
        let precheck = self.precheck(&copy_share);

        match share_merge_mode {
            // Safe mode: conflicts are not allowed
            ShareMergeMode::Safe => {
                // Conflicts found
                if !precheck.ok() {
                    // Do nothing, return Error
                    return Err(Error::new(
                        std::io::ErrorKind::AlreadyExists,
                        "Mappings or files already exist!",
                    ));
                }
            }
            // Overwrite mode: when conflicts occur, prioritize the share item
            ShareMergeMode::Overwrite => {
                // Handle duplicate mappings
                for path in precheck.duplicate_mapping {
                    // Get the share data
                    let Some(share_value) = copy_share.mappings.remove(&path) else {
                        return Err(Error::new(
                            std::io::ErrorKind::NotFound,
                            format!("Share value `{}` not found!", &path.display()),
                        ));
                    };
                    // Overwrite
                    copy_sheet.mapping_mut().insert(path, share_value);
                }

                // Handle duplicate IDs
                for path in precheck.duplicate_file {
                    // Get the share data
                    let Some(share_value) = copy_share.mappings.remove(&path) else {
                        return Err(Error::new(
                            std::io::ErrorKind::NotFound,
                            format!("Share value `{}` not found!", &path.display()),
                        ));
                    };

                    // Extract the file ID
                    let conflict_vfid = &share_value.id;

                    // Through the sheet's ID mapping
                    let Some(id_mapping) = copy_sheet.id_mapping_mut() else {
                        return Err(Error::new(
                            std::io::ErrorKind::NotFound,
                            "Id mapping not found!",
                        ));
                    };

                    // Get the original path from the ID mapping
                    let Some(raw_path) = id_mapping.remove(conflict_vfid) else {
                        return Err(Error::new(
                            std::io::ErrorKind::NotFound,
                            format!("The path of virtual file `{}' not found!", conflict_vfid),
                        ));
                    };

                    // Remove the original path mapping
                    if copy_sheet.mapping_mut().remove(&raw_path).is_none() {
                        return Err(Error::new(
                            std::io::ErrorKind::NotFound,
                            format!("Remove mapping `{}` failed!", &raw_path.display()),
                        ));
                    }
                    // Insert the new item
                    copy_sheet.mapping_mut().insert(path, share_value);
                }
            }
            // Skip mode: when conflicts occur, prioritize the local sheet
            ShareMergeMode::Skip => {
                // Directly remove conflicting items
                for path in precheck.duplicate_mapping {
                    copy_share.mappings.remove(&path);
                }
                for path in precheck.duplicate_file {
                    copy_share.mappings.remove(&path);
                }
            }
        }

        // Subsequent merging
        copy_sheet
            .mapping_mut()
            .extend(copy_share.mappings.into_iter());

        // Merge completed
        self.data = copy_sheet; // Write the result

        // Merge completed, consume the sheet
        self.persist().await.map_err(|err| {
            Error::new(
                std::io::ErrorKind::NotFound,
                format!("Write sheet failed: {}", err),
            )
        })?;

        // Persistence succeeded, continue to consume the share item
        share.remove().await.map_err(|err| {
            Error::new(
                std::io::ErrorKind::NotFound,
                format!("Remove share failed: {}", err.1),
            )
        })
    }

    // Pre-check whether the share can be imported into the current sheet without conflicts
    fn precheck(&self, share: &Share) -> ShareMergeConflict {
        let mut conflicts = ShareMergeConflict::default();

        for (mapping, metadata) in &share.mappings {
            // Check for duplicate mappings
            if self.mapping().contains_key(mapping.as_path()) {
                conflicts.duplicate_mapping.push(mapping.clone());
                continue;
            }

            // Check for duplicate IDs
            if let Some(id_mapping) = self.id_mapping() {
                if id_mapping.contains_key(&metadata.id) {
                    conflicts.duplicate_file.push(mapping.clone());
                    continue;
                }
            }
        }

        conflicts
    }

    /// Share mappings with another sheet
    pub async fn share_mappings(
        &self,
        other_sheet: &SheetName,
        mappings: Vec<PathBuf>,
        sharer: &MemberId,
        description: String,
    ) -> Result<Share, std::io::Error> {
        let other_sheet = snake_case!(other_sheet.clone());
        let sharer = snake_case!(sharer.clone());

        // Check if the sheet exists
        let sheet_names = self.vault_reference.sheet_names()?;
        if !sheet_names.contains(&other_sheet) {
            return Err(Error::new(
                std::io::ErrorKind::NotFound,
                format!("Sheet `{}` not found!", &other_sheet),
            ));
        }

        // Check if the target file exists, regenerate ID if path already exists, up to 20 attempts
        let target_path = {
            let mut id;
            let mut share_path;
            let mut attempts = 0;

            loop {
                id = Share::gen_share_id(&sharer);
                share_path = self.vault_reference.share_file_path(&other_sheet, &id);

                if !share_path.exists() {
                    break share_path;
                }

                attempts += 1;
                if attempts >= 20 {
                    return Err(Error::new(
                        std::io::ErrorKind::AlreadyExists,
                        "Failed to generate unique share ID after 20 attempts!",
                    ));
                }
            }
        };

        // Validate that the share is valid
        let mut share_mappings = HashMap::new();
        for mapping_path in &mappings {
            if let Some(metadata) = self.mapping().get(mapping_path) {
                share_mappings.insert(mapping_path.clone(), metadata.clone());
            } else {
                return Err(Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Mapping `{}` not found in sheet!", mapping_path.display()),
                ));
            }
        }

        // Build share data
        let share_data = Share {
            sharer,
            description,
            path: None, // This is only needed during merging (reading), no need to serialize now
            from_sheet: self.name.clone(),
            mappings: share_mappings,
        };

        // Write data
        Share::write_to(&share_data, target_path).await?;

        Ok(share_data)
    }
}

impl Share {
    /// Generate a share ID for a given sharer
    pub fn gen_share_id(sharer: &MemberId) -> String {
        let sharer_snake = snake_case!(sharer.clone());
        let random_part: String = rng()
            .sample_iter(&rand::distr::Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();
        format!("{}@{}", sharer_snake, random_part)
    }

    /// Delete a share (reject or remove the share item)
    /// If deletion succeeds, returns `Ok(())`;
    /// If deletion fails, returns `Err((self, std::io::Error))`, containing the original share object and the error information.
    pub async fn remove(self) -> Result<(), (Self, std::io::Error)> {
        let Some(path) = &self.path else {
            return Err((
                self,
                Error::new(std::io::ErrorKind::NotFound, "No share path recorded!"),
            ));
        };

        if !path.exists() {
            return Err((
                self,
                Error::new(std::io::ErrorKind::NotFound, "No share file exists!"),
            ));
        }

        match fs::remove_file(path).await {
            Err(err) => Err((
                self,
                Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to delete share file: {}", err),
                ),
            )),
            Ok(_) => Ok(()),
        }
    }
}
