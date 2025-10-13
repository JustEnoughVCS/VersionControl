use std::{collections::HashMap, path::PathBuf};

use cfg_file::{ConfigFile, config::ConfigFile};
use serde::{Deserialize, Serialize};
use string_proc::simple_processer::sanitize_file_path;

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

#[derive(Debug, Clone, Serialize, Deserialize, Eq)]
pub struct InputPackage {
    /// Name of the input package
    pub name: InputName,

    /// The sheet from which this input package was created
    pub from: SheetName,

    /// Files in this input package with their relative paths and virtual file IDs
    pub files: Vec<(InputRelativePathBuf, VirtualFileId)>,
}

impl PartialEq for InputPackage {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
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

    /// Get the names of the inputs of this sheet
    pub fn input_names(&self) -> Vec<String> {
        self.data
            .inputs
            .iter()
            .map(|input| input.name.clone())
            .collect()
    }

    /// Get the mapping of this sheet
    pub fn mapping(&self) -> &HashMap<SheetPathBuf, VirtualFileId> {
        &self.data.mapping
    }

    /// Add an input package to the sheet
    pub fn add_input(&mut self, input_package: InputPackage) -> Result<(), std::io::Error> {
        if self.data.inputs.iter().any(|input| input == &input_package) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("Input package '{}' already exists", input_package.name),
            ));
        }
        self.data.inputs.push(input_package);
        Ok(())
    }

    /// Deny and remove an input package from the sheet
    pub fn deny_input(&mut self, input_name: &InputName) -> Option<InputPackage> {
        self.data
            .inputs
            .iter()
            .position(|input| input.name == *input_name)
            .map(|pos| self.data.inputs.remove(pos))
    }

    /// Accept an input package and insert to the sheet
    pub async fn accept_import(
        &mut self,
        input_name: &InputName,
        insert_to: &SheetPathBuf,
    ) -> Result<(), std::io::Error> {
        // Remove inputs
        let input = self
            .inputs()
            .iter()
            .position(|input| input.name == *input_name)
            .map(|pos| self.data.inputs.remove(pos));

        // Ensure input is not empty
        let Some(input) = input else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Empty inputs.",
            ));
        };

        // Insert to sheet
        for (relative_path, virtual_file_id) in input.files {
            self.add_mapping(insert_to.join(relative_path), virtual_file_id)
                .await?;
        }

        Ok(())
    }

    /// Add (or Edit) a mapping entry to the sheet
    ///
    /// This operation performs safety checks to ensure the member has the right to add the mapping:
    /// 1. If the virtual file ID doesn't exist in the vault, the mapping is added directly
    /// 2. If the virtual file exists, check if the member has edit rights to the virtual file
    /// 3. If member has edit rights, the mapping is not allowed to be modified and returns an error
    /// 4. If member doesn't have edit rights, the mapping is allowed (member is giving up the file)
    ///
    /// Note: Full validation adds overhead - avoid frequent calls
    pub async fn add_mapping(
        &mut self,
        sheet_path: SheetPathBuf,
        virtual_file_id: VirtualFileId,
    ) -> Result<(), std::io::Error> {
        // Check if the virtual file exists in the vault
        if self.vault_reference.virtual_file(&virtual_file_id).is_err() {
            // Virtual file doesn't exist, add the mapping directly
            self.data.mapping.insert(sheet_path, virtual_file_id);
            return Ok(());
        }

        // Check if the holder has edit rights to the virtual file
        match self
            .vault_reference
            .has_virtual_file_edit_right(self.holder(), &virtual_file_id)
            .await
        {
            Ok(false) => {
                // Holder doesn't have rights, add the mapping (member is giving up the file)
                self.data.mapping.insert(sheet_path, virtual_file_id);
                Ok(())
            }
            Ok(true) => {
                // Holder has edit rights, don't allow modifying the mapping
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "Member has edit rights to the virtual file, cannot modify mapping",
                ))
            }
            Err(_) => {
                // Error checking rights, don't allow modifying the mapping
                Err(std::io::Error::other(
                    "Failed to check virtual file edit rights",
                ))
            }
        }
    }

    /// Remove a mapping entry from the sheet
    ///
    /// This operation performs safety checks to ensure the member has the right to remove the mapping:
    /// 1. Member must NOT have edit rights to the virtual file to release it (ensuring clear ownership)
    /// 2. If the virtual file doesn't exist, the mapping is removed but no ID is returned
    /// 3. If member has no edit rights and the file exists, returns the removed virtual file ID
    ///
    /// Note: Full validation adds overhead - avoid frequent calls
    pub async fn remove_mapping(&mut self, sheet_path: &SheetPathBuf) -> Option<VirtualFileId> {
        let virtual_file_id = match self.data.mapping.get(sheet_path) {
            Some(id) => id,
            None => {
                // The mapping entry doesn't exist, nothing to remove
                return None;
            }
        };

        // Check if the virtual file exists in the vault
        if self.vault_reference.virtual_file(virtual_file_id).is_err() {
            // Virtual file doesn't exist, remove the mapping and return None
            self.data.mapping.remove(sheet_path);
            return None;
        }

        // Check if the holder has edit rights to the virtual file
        match self
            .vault_reference
            .has_virtual_file_edit_right(self.holder(), virtual_file_id)
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

    /// Export files from the current sheet as an InputPackage for importing into other sheets
    ///
    /// This is the recommended way to create InputPackages. It takes a list of sheet paths
    /// and generates an InputPackage with optimized relative paths by removing the longest
    /// common prefix from all provided paths, then placing the files under a directory
    /// named with the output_name.
    ///
    /// # Example
    /// Given paths:
    /// - `MyProject/Art/Character/Model/final.fbx`
    /// - `MyProject/Art/Character/Texture/final.png`
    /// - `MyProject/Art/Character/README.md`
    ///
    /// With output_name = "MyExport", the resulting package will contain:
    /// - `MyExport/Model/final.fbx`
    /// - `MyExport/Texture/final.png`
    /// - `MyExport/README.md`
    ///
    /// # Arguments
    /// * `output_name` - Name of the output package (will be used as the root directory)
    /// * `paths` - List of sheet paths to include in the package
    ///
    /// # Returns
    /// Returns an InputPackage containing the exported files with optimized paths,
    /// or an error if paths are empty or files are not found in the sheet mapping
    pub fn output_mappings(
        &self,
        output_name: InputName,
        paths: &[SheetPathBuf],
    ) -> Result<InputPackage, std::io::Error> {
        let output_name = sanitize_file_path(output_name);

        // Return error for empty paths since there's no need to generate an empty package
        if paths.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Cannot generate output package with empty paths",
            ));
        }

        // Find the longest common prefix among all paths
        let common_prefix = Self::find_longest_common_prefix(paths);

        // Create output files with optimized relative paths
        let files = paths
            .iter()
            .map(|path| {
                let relative_path = path.strip_prefix(&common_prefix).unwrap_or(path);
                let output_path = PathBuf::from(&output_name).join(relative_path);

                self.data
                    .mapping
                    .get(path)
                    .map(|vfid| (output_path, vfid.clone()))
                    .ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            format!("File not found: {:?}", path),
                        )
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(InputPackage {
            name: output_name,
            from: self.name.clone(),
            files,
        })
    }

    /// Helper function to find the longest common prefix among all paths
    fn find_longest_common_prefix(paths: &[SheetPathBuf]) -> PathBuf {
        if paths.is_empty() {
            return PathBuf::new();
        }

        let first_path = &paths[0];
        let mut common_components = Vec::new();

        for (component_idx, first_component) in first_path.components().enumerate() {
            for path in paths.iter().skip(1) {
                if let Some(component) = path.components().nth(component_idx) {
                    if component != first_component {
                        return common_components.into_iter().collect();
                    }
                } else {
                    return common_components.into_iter().collect();
                }
            }
            common_components.push(first_component);
        }

        common_components.into_iter().collect()
    }
}
