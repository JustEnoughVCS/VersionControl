use std::{
    collections::HashSet,
    fs::File,
    path::{Path, PathBuf},
};

use memmap2::Mmap;
use tokio::fs;

use crate::{
    index_source::IndexSource,
    mapping::{LocalMapping, LocalMappingForward, Mapping, MappingBuf},
    sheet::{
        error::{ReadSheetDataError, SheetEditError},
        reader::{read_mapping, read_sheet_data},
        writer::convert_sheet_data_to_bytes,
    },
};

pub mod constants;
pub mod error;
pub mod reader;
pub mod writer;

#[cfg(test)]
pub mod test;

#[derive(Default, Debug, Clone, PartialEq)]
pub struct Sheet {
    /// Sheet Name
    name: String,

    /// Data in the sheet
    data: SheetData,

    /// Edit information
    edit: SheetEdit,
}

/// Full Sheet information
///
/// Used to wrap as a Sheet object for editing and persistence
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct SheetData {
    /// All local mappings
    mappings: HashSet<LocalMapping>,
}

/// Mmap for SheetData
pub struct SheetDataMmap {
    mmap: Mmap,
}

/// Editing state of the Sheet
///
/// Stored in the Sheet, records the editing operations that **will** be performed on its SheetData
/// The content will be cleared after the edits are applied
#[derive(Default, Debug, Clone, PartialEq)]
pub struct SheetEdit {
    /// Edit history
    list: Vec<SheetEditItem>,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub enum SheetEditItem {
    /// Do nothing, this entry is not included in checksum and audit
    #[default]
    DoNothing,

    /// Move a Mapping's Node to another Node
    Move {
        from_node: Vec<String>,
        to_node: Vec<String>,
    },

    /// Swap the positions of two Mapping Nodes
    Swap {
        node_a: Vec<String>,
        node_b: Vec<String>,
    },

    /// Erase the Mapping pointed to by a Node
    EraseMapping { node: Vec<String> },

    /// Insert a new Mapping
    InsertMapping { mapping: LocalMapping },

    /// Replace the IndexSource of a Mapping pointed to by a Node
    ReplaceSource {
        node: Vec<String>,
        source: IndexSource,
    },

    /// Update the LocalMappingForward of a Mapping pointed to by a Node
    UpdateForward {
        node: Vec<String>,
        forward: LocalMappingForward,
    },
}

impl std::fmt::Display for SheetEditItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SheetEditItem::DoNothing => write!(f, ""),
            SheetEditItem::Move { from_node, to_node } => {
                write!(
                    f,
                    "Move \"{}\" -> \"{}\"",
                    display_node_helper(from_node),
                    display_node_helper(to_node),
                )
            }
            SheetEditItem::Swap { node_a, node_b } => {
                write!(
                    f,
                    "Swap \"{}\" <-> \"{}\"",
                    display_node_helper(node_a),
                    display_node_helper(node_b),
                )
            }
            SheetEditItem::EraseMapping { node } => {
                write!(f, "Earse \"{}\"", display_node_helper(node),)
            }
            SheetEditItem::InsertMapping { mapping } => {
                write!(f, "Insert {}", mapping.to_string())
            }
            SheetEditItem::ReplaceSource { node, source } => {
                write!(
                    f,
                    "Replace \"{}\" => \"{}\"",
                    display_node_helper(node),
                    source.to_string()
                )
            }
            SheetEditItem::UpdateForward { node, forward } => match forward {
                LocalMappingForward::Latest => {
                    write!(f, "Update \"{}\" => Latest", display_node_helper(node))
                }
                LocalMappingForward::Ref { sheet_name } => {
                    write!(
                        f,
                        "Update \"{}\" => Ref(\"{}\")",
                        display_node_helper(node),
                        sheet_name
                    )
                }
                LocalMappingForward::Version {
                    version: version_name,
                } => {
                    write!(
                        f,
                        "Update \"{}\" => Ver(\"{}\")",
                        display_node_helper(node),
                        version_name
                    )
                }
            },
        }
    }
}

#[inline(always)]
fn display_node_helper(n: &Vec<String>) -> String {
    n.join("/")
}

impl SheetData {
    /// Create an empty SheetData
    pub fn empty() -> Self {
        Self {
            mappings: HashSet::new(),
        }
    }

    /// Read SheetData completely from the workspace
    pub async fn full_read(
        &mut self,
        sheet_file: impl Into<PathBuf>,
    ) -> Result<(), ReadSheetDataError> {
        let file_data = fs::read(sheet_file.into()).await?;
        let sheet_data = read_sheet_data(file_data.as_slice())?;
        self.mappings = sheet_data.mappings;
        Ok(())
    }

    /// Load MMAP from a Sheet file
    pub fn mmap<'a>(sheet_file: impl AsRef<Path>) -> std::io::Result<SheetDataMmap> {
        let file = File::open(sheet_file.as_ref())?;

        // SAFETY: The file has been successfully opened and is managed by the SheetDataMmap wrapper
        let mmap = unsafe { Mmap::map(&file)? };
        Ok(SheetDataMmap { mmap })
    }

    /// Check if a mapping exists in SheetData
    pub fn contains_mapping(&self, value: &Vec<String>) -> bool {
        self.mappings.contains(value)
    }

    /// Read local mapping information from SheetData
    pub fn read_local_mapping(&self, value: &Vec<String>) -> Option<&LocalMapping> {
        self.mappings.get(value)
    }

    /// Wrap SheetData into a Sheet
    pub fn pack(self, sheet_name: impl Into<String>) -> Sheet {
        Sheet {
            name: sheet_name.into(),
            data: self,
            edit: SheetEdit { list: Vec::new() },
        }
    }
}

impl SheetDataMmap {
    /// Load mapping information from MMAP at high speed
    pub fn mp<'a>(
        &'a self,
        node: &[&str],
    ) -> Result<Option<(Mapping<'a>, LocalMappingForward)>, ReadSheetDataError> {
        read_mapping(&self.mmap[..], node)
    }

    /// Load mapping information from Sheet file at high speed and copy into LocalMapping
    pub fn mp_c<'a>(&self, node: &[&str]) -> Result<Option<LocalMapping>, ReadSheetDataError> {
        match self.mp(node)? {
            Some((mapping, forward)) => {
                // Note:
                // Regarding the `unwrap()` here:
                // Data is read from the original SheetData, it cannot produce values longer than `u8::MAX`
                // It cannot trigger local_mapping's validity check, so it can be safely unwrapped
                let local_mapping = mapping.to_local_mapping(forward).unwrap();

                Ok(Some(local_mapping))
            }
            None => Ok(None),
        }
    }
}

impl Sheet {
    /// Unpack Sheet into pure data
    pub fn unpack(self) -> SheetData {
        self.data
    }

    /// Check if a mapping exists in the Sheet
    pub fn contains_mapping(&self, value: &Vec<String>) -> bool {
        self.data.contains_mapping(value)
    }

    /// Read local mapping information from Sheet data
    pub fn read_local_mapping(&self, value: &Vec<String>) -> Option<&LocalMapping> {
        self.data.read_local_mapping(value)
    }

    /// Read from Sheet data and clone into MappingBuf
    pub fn read_mapping_buf(&self, value: &Vec<String>) -> Option<MappingBuf> {
        match self.read_local_mapping(value) {
            Some(v) => Some(v.to_mapping_buf_cloned(&self.name)),
            None => None,
        }
    }

    /// Insert mapping modification
    pub fn insert_mapping(
        &mut self,
        mapping: impl Into<LocalMapping>,
    ) -> Result<(), SheetEditError> {
        self.edit.list.push(SheetEditItem::InsertMapping {
            mapping: mapping.into(),
        });
        Ok(())
    }

    /// Insert mapping erasure
    pub fn earse_mapping(&mut self, node: Vec<String>) -> Result<(), SheetEditError> {
        self.edit.list.push(SheetEditItem::EraseMapping { node });
        Ok(())
    }

    /// Insert mapping swap
    pub fn swap_mapping(
        &mut self,
        node_a: Vec<String>,
        node_b: Vec<String>,
    ) -> Result<(), SheetEditError> {
        self.edit.list.push(SheetEditItem::Swap { node_a, node_b });
        Ok(())
    }

    /// Insert mapping move
    pub fn move_mapping(
        &mut self,
        from_node: Vec<String>,
        to_node: Vec<String>,
    ) -> Result<(), SheetEditError> {
        self.edit
            .list
            .push(SheetEditItem::Move { from_node, to_node });
        Ok(())
    }

    /// Replace source
    pub fn replace_source(
        &mut self,
        node: Vec<String>,
        source: IndexSource,
    ) -> Result<(), SheetEditError> {
        self.edit
            .list
            .push(SheetEditItem::ReplaceSource { node, source });
        Ok(())
    }

    /// Update forward
    pub fn update_forward(
        &mut self,
        node: Vec<String>,
        forward: LocalMappingForward,
    ) -> Result<(), SheetEditError> {
        self.edit
            .list
            .push(SheetEditItem::UpdateForward { node, forward });
        Ok(())
    }

    /// Apply changes
    pub fn apply(&mut self) {
        // Logic for applying changes
        todo!();

        // Clear the edit list
        #[allow(unreachable_code)] // Note: Remove after todo!() is completed
        self.edit.list.clear();
    }
}

// Implement the as_bytes function for SheetData

impl SheetData {
    /// Convert SheetData to byte data for storage in the file system
    pub fn as_bytes(self) -> Vec<u8> {
        convert_sheet_data_to_bytes(self)
    }
}

impl From<SheetData> for Vec<u8> {
    fn from(value: SheetData) -> Self {
        value.as_bytes()
    }
}

impl From<&SheetData> for Vec<u8> {
    fn from(value: &SheetData) -> Self {
        value.clone().as_bytes()
    }
}

impl TryFrom<Vec<u8>> for SheetData {
    type Error = ReadSheetDataError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        read_sheet_data(value.as_slice())
    }
}

impl TryFrom<&[u8]> for SheetData {
    type Error = ReadSheetDataError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        read_sheet_data(value)
    }
}
