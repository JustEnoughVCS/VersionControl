use std::{collections::HashMap, path::PathBuf};

use cfg_file::{ConfigFile, config::ConfigFile};
use serde::{Deserialize, Serialize};

use crate::{
    constants::SERVER_FILE_SHEET,
    data::{
        member::MemberId,
        vault::{
            Vault,
            virtual_file::{VirtualFileId, VirtualFileVersion},
        },
    },
};

pub type SheetName = String;
pub type SheetPathBuf = PathBuf;

const SHEET_NAME: &str = "{sheet_name}";

pub struct Sheet<'a> {
    /// The name of the current sheet
    pub(crate) name: SheetName,

    /// Sheet data
    pub(crate) data: SheetData,

    /// Sheet path
    pub(crate) vault_reference: &'a Vault,
}

#[derive(Default, Serialize, Deserialize, ConfigFile, Clone)]
pub struct SheetData {
    /// The write count of the current sheet
    #[serde(rename = "v")]
    pub(crate) write_count: i32,

    /// The holder of the current sheet, who has full operation rights to the sheet mapping
    #[serde(rename = "holder")]
    pub(crate) holder: Option<MemberId>,

    /// Mapping of sheet paths to virtual file IDs
    #[serde(rename = "map")]
    pub(crate) mapping: HashMap<SheetPathBuf, SheetMappingMetadata>,

    /// Mapping of virtual file Ids to sheet paths
    #[serde(rename = "id_map")]
    pub(crate) id_mapping: Option<HashMap<VirtualFileId, SheetPathBuf>>,
}

#[derive(Debug, Default, Serialize, Deserialize, ConfigFile, Clone, Eq, PartialEq)]
pub struct SheetMappingMetadata {
    #[serde(rename = "id")]
    pub id: VirtualFileId,
    #[serde(rename = "ver")]
    pub version: VirtualFileVersion,
}

impl<'a> Sheet<'a> {
    pub fn name(&self) -> &SheetName {
        &self.name
    }

    /// Get the holder of this sheet
    pub fn holder(&self) -> Option<&MemberId> {
        self.data.holder.as_ref()
    }

    /// Get the mapping of this sheet
    pub fn mapping(&self) -> &HashMap<SheetPathBuf, SheetMappingMetadata> {
        &self.data.mapping
    }

    /// Get the muttable mapping of this sheet
    pub fn mapping_mut(&mut self) -> &mut HashMap<SheetPathBuf, SheetMappingMetadata> {
        &mut self.data.mapping
    }

    /// Get the id_mapping of this sheet data
    pub fn id_mapping(&self) -> &Option<HashMap<VirtualFileId, SheetPathBuf>> {
        &self.data.id_mapping
    }

    /// Get the write count of this sheet
    pub fn write_count(&self) -> i32 {
        self.data.write_count
    }

    /// Forget the holder of this sheet
    pub fn forget_holder(&mut self) {
        self.data.holder = None;
    }

    /// Set the holder of this sheet
    pub fn set_holder(&mut self, holder: MemberId) {
        self.data.holder = Some(holder);
    }

    /// Add (or Edit) a mapping entry to the sheet
    ///
    /// This operation performs safety checks to ensure the member has the right to add the mapping:
    /// 1. The sheet must have a holder (member) to perform this operation
    /// 2. If the virtual file ID doesn't exist in the vault, the mapping is added directly
    /// 3. If the virtual file exists, the mapping is added regardless of member edit rights
    ///
    /// Note: Full validation adds overhead - avoid frequent calls
    pub async fn add_mapping(
        &mut self,
        sheet_path: SheetPathBuf,
        virtual_file_id: VirtualFileId,
        version: VirtualFileVersion,
    ) -> Result<(), std::io::Error> {
        // Check if the virtual file exists in the vault
        if self.vault_reference.virtual_file(&virtual_file_id).is_err() {
            // Virtual file doesn't exist, add the mapping directly
            self.data.mapping.insert(
                sheet_path,
                SheetMappingMetadata {
                    id: virtual_file_id,
                    version,
                },
            );
            return Ok(());
        }

        // Check if the sheet has a holder
        let Some(_) = self.holder() else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "This sheet has no holder",
            ));
        };

        self.data.mapping.insert(
            sheet_path,
            SheetMappingMetadata {
                id: virtual_file_id,
                version,
            },
        );

        Ok(())
    }

    /// Remove a mapping entry from the sheet
    ///
    /// This operation performs safety checks to ensure the member has the right to remove the mapping:
    /// 1. The sheet must have a holder (member) to perform this operation
    /// 2. Member must NOT have edit rights to the virtual file to release it (ensuring clear ownership)
    /// 3. If the virtual file doesn't exist, the mapping is removed but no ID is returned
    /// 4. If member has no edit rights and the file exists, returns the removed virtual file ID
    ///
    /// Note: Full validation adds overhead - avoid frequent calls
    pub async fn remove_mapping(
        &mut self,
        sheet_path: &SheetPathBuf,
    ) -> Option<SheetMappingMetadata> {
        let virtual_file_meta = match self.data.mapping.get(sheet_path) {
            Some(id) => id,
            None => {
                // The mapping entry doesn't exist, nothing to remove
                return None;
            }
        };

        // Check if the virtual file exists in the vault
        if self
            .vault_reference
            .virtual_file(&virtual_file_meta.id)
            .is_err()
        {
            // Virtual file doesn't exist, remove the mapping and return None
            self.data.mapping.remove(sheet_path);
            return None;
        }

        // Check if the sheet has a holder
        let holder = self.holder()?;

        // Check if the holder has edit rights to the virtual file
        match self
            .vault_reference
            .has_virtual_file_edit_right(holder, &virtual_file_meta.id)
            .await
        {
            Ok(false) => {
                // Holder doesn't have rights, remove and return the virtual file ID
                self.data.mapping.remove(sheet_path)
            }
            Ok(true) => {
                // Holder has edit rights, don't remove the mapping
                None
            }
            Err(_) => {
                // Error checking rights, don't remove the mapping
                None
            }
        }
    }

    /// Persist the sheet to disk
    ///
    /// Why not use a reference?
    /// Because I don't want a second instance of the sheet to be kept in memory.
    /// If needed, please deserialize and reload it.
    pub async fn persist(mut self) -> Result<(), std::io::Error> {
        self.data.write_count += 1;

        // Update id mapping
        self.data.id_mapping = Some(HashMap::new());
        for map in self.data.mapping.iter() {
            self.data
                .id_mapping
                .as_mut()
                .unwrap()
                .insert(map.1.id.clone(), map.0.clone());
        }

        // Add write count
        if self.data.write_count >= i32::MAX - 1 {
            self.data.write_count = 0;
        }
        SheetData::write_to(&self.data, self.sheet_path()).await
    }

    /// Get the path to the sheet file
    pub fn sheet_path(&self) -> PathBuf {
        Sheet::sheet_path_with_name(self.vault_reference, &self.name)
    }

    /// Get the path to the sheet file with the given name
    pub fn sheet_path_with_name(vault: &Vault, name: impl AsRef<str>) -> PathBuf {
        vault
            .vault_path()
            .join(SERVER_FILE_SHEET.replace(SHEET_NAME, name.as_ref()))
    }

    /// Clone the data of the sheet
    pub fn clone_data(&self) -> SheetData {
        self.data.clone()
    }

    /// Convert the sheet into its data representation
    pub fn to_data(self) -> SheetData {
        self.data
    }
}

impl SheetData {
    /// Get the write count of this sheet data
    pub fn write_count(&self) -> i32 {
        self.write_count
    }

    /// Get the holder of this sheet data
    pub fn holder(&self) -> Option<&MemberId> {
        self.holder.as_ref()
    }

    /// Get the mapping of this sheet data
    pub fn mapping(&self) -> &HashMap<SheetPathBuf, SheetMappingMetadata> {
        &self.mapping
    }

    /// Get the muttable mapping of this sheet data
    pub fn mapping_mut(&mut self) -> &mut HashMap<SheetPathBuf, SheetMappingMetadata> {
        &mut self.mapping
    }

    /// Get the id_mapping of this sheet data
    pub fn id_mapping(&self) -> &Option<HashMap<VirtualFileId, SheetPathBuf>> {
        &self.id_mapping
    }

    /// Get the muttable id_mapping of this sheet data
    pub fn id_mapping_mut(&mut self) -> &mut Option<HashMap<VirtualFileId, SheetPathBuf>> {
        &mut self.id_mapping
    }
}
