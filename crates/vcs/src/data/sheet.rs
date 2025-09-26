use std::{collections::HashMap, path::PathBuf};

use cfg_file::{ConfigFile, config::ConfigFile};
use serde::{Deserialize, Serialize};

use crate::{
    constants::SERVER_FILE_SHEET,
    data::{
        member::MemberId,
        vault::{Vault, virtual_file::VirtualFileId},
    },
};

pub type SheetName = String;
pub type SheetPathBuf = PathBuf;
pub type InputName = String;
pub type InputRelativePathBuf = PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputPackage {
    /// Name of the input package
    pub name: InputName,
    /// Files in this input package with their relative paths and virtual file IDs
    pub files: Vec<(InputRelativePathBuf, VirtualFileId)>,
}

const SHEET_NAME: &str = "{sheet-name}";

pub struct Sheet<'a> {
    /// The name of the current sheet
    pub(crate) name: SheetName,

    /// Sheet data
    pub(crate) data: SheetData,

    /// Sheet path
    pub(crate) vault_reference: &'a Vault,
}

#[derive(Default, Serialize, Deserialize, ConfigFile)]
pub struct SheetData {
    /// The holder of the current sheet, who has full operation rights to the sheet mapping
    pub(crate) holder: MemberId,

    /// Inputs
    pub(crate) inputs: Vec<InputPackage>,

    /// Mapping of sheet paths to virtual file IDs
    pub(crate) mapping: HashMap<SheetPathBuf, VirtualFileId>,
}

impl<'a> Sheet<'a> {
    /// Get the holder of this sheet
    pub fn holder(&self) -> &MemberId {
        &self.data.holder
    }

    /// Get the inputs of this sheet
    pub fn inputs(&self) -> &Vec<InputPackage> {
        &self.data.inputs
    }

    /// Get the mapping of this sheet
    pub fn mapping(&self) -> &HashMap<SheetPathBuf, VirtualFileId> {
        &self.data.mapping
    }

    /// Add an input package to the sheet
    pub fn add_input(
        &mut self,
        input_name: InputName,
        files: Vec<(InputRelativePathBuf, VirtualFileId)>,
    ) {
        self.data.inputs.push(InputPackage {
            name: input_name,
            files,
        });
    }

    /// Remove an input package from the sheet
    pub fn remove_input(&mut self, input_name: &InputName) -> Option<InputPackage> {
        self.data
            .inputs
            .iter()
            .position(|input| input.name == *input_name)
            .map(|pos| self.data.inputs.remove(pos))
    }

    /// Add a mapping entry to the sheet
    pub fn add_mapping(&mut self, sheet_path: SheetPathBuf, virtual_file_id: VirtualFileId) {
        self.data.mapping.insert(sheet_path, virtual_file_id);
    }

    /// Remove a mapping entry from the sheet
    pub fn remove_mapping(&mut self, sheet_path: &SheetPathBuf) -> Option<VirtualFileId> {
        self.data.mapping.remove(sheet_path)
    }

    /// Persist the sheet to disk
    ///
    /// Why not use a reference?
    /// Because I don't want a second instance of the sheet to be kept in memory.
    /// If needed, please deserialize and reload it.
    pub async fn persist(self) -> Result<(), std::io::Error> {
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
}
