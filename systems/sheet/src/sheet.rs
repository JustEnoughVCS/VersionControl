use std::{
    collections::HashSet,
    fs::File,
    mem::replace,
    path::{Path, PathBuf},
};

use memmap2::Mmap;
use tokio::fs;

use crate::{
    index_source::IndexSource,
    mapping::{LocalMapping, LocalMappingForward, Mapping, MappingBuf},
    sheet::{
        error::{ReadSheetDataError, SheetApplyError, SheetEditError},
        reader::{read_mapping, read_sheet_data},
        writer::convert_sheet_data_to_bytes,
    },
};

pub mod error;
pub mod reader;
pub mod writer;

// Format
pub mod v1;

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

    /// List of lost nodes
    lost_nodes: HashSet<Vec<String>>,

    /// List of filled nodes
    filled_nodes: HashSet<Vec<String>>,
}

impl SheetEdit {
    /// Clear current SheetEdit information
    pub fn clear(&mut self) {
        self.list.clear();
        self.lost_nodes.clear();
        self.filled_nodes.clear();
    }
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
                write!(f, "erase \"{}\"", display_node_helper(node),)
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
            edit: SheetEdit::default(),
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

    /// Check if a mapping exists in this sheet (including unapplied edits)
    /// The difference from `contains_mapping_actual` is:
    /// This method will consider edit operations that have not yet been `apply()`-ed
    pub fn contains_mapping(&self, value: &Vec<String>) -> bool {
        self.data.contains_mapping(value) && !self.edit.lost_nodes.contains(value)
            || self.edit.filled_nodes.contains(value)
    }

    /// Check if a mapping actually exists in this sheet
    pub fn contains_mapping_actual(&self, value: &Vec<String>) -> bool {
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

    /// Insert mapping move
    pub fn move_mapping(
        &mut self,
        from_node: Vec<String>,
        to_node: Vec<String>,
    ) -> Result<(), SheetEditError> {
        if !self.contains_mapping(&from_node) {
            return Err(SheetEditError::NodeNotFound(from_node.join("/")));
        }

        if self.contains_mapping(&to_node) {
            return Err(SheetEditError::NodeAlreadyExist(to_node.join("/")));
        }

        self.edit.filled_nodes.remove(&from_node);
        self.edit.lost_nodes.remove(&to_node);

        self.edit.lost_nodes.insert(from_node.clone());
        self.edit.filled_nodes.insert(to_node.clone());

        self.edit
            .list
            .push(SheetEditItem::Move { from_node, to_node });

        Ok(())
    }

    /// Insert mapping swap
    pub fn swap_mapping(
        &mut self,
        node_a: Vec<String>,
        node_b: Vec<String>,
    ) -> Result<(), SheetEditError> {
        if !self.contains_mapping(&node_a) {
            return Err(SheetEditError::NodeNotFound(node_a.join("/")));
        }

        if !self.contains_mapping(&node_b) {
            return Err(SheetEditError::NodeNotFound(node_b.join("/")));
        }

        self.edit.list.push(SheetEditItem::Swap { node_a, node_b });
        Ok(())
    }

    /// Insert mapping erasure
    pub fn erase_mapping(&mut self, node: Vec<String>) -> Result<(), SheetEditError> {
        if !self.contains_mapping(&node) {
            return Err(SheetEditError::NodeNotFound(node.join("/")));
        }

        self.edit.filled_nodes.remove(&node);
        self.edit.lost_nodes.insert(node.clone());

        self.edit.list.push(SheetEditItem::EraseMapping { node });
        Ok(())
    }

    /// Insert mapping modification
    pub fn insert_mapping(
        &mut self,
        mapping: impl Into<LocalMapping>,
    ) -> Result<(), SheetEditError> {
        let mapping = mapping.into();
        let node = mapping.value();

        if self.contains_mapping(node) {
            return Err(SheetEditError::NodeAlreadyExist(node.join("/")));
        }

        self.edit.lost_nodes.remove(node);
        self.edit.filled_nodes.insert(node.clone());

        self.edit
            .list
            .push(SheetEditItem::InsertMapping { mapping });
        Ok(())
    }

    /// Replace source
    pub fn replace_source(
        &mut self,
        node: Vec<String>,
        source: IndexSource,
    ) -> Result<(), SheetEditError> {
        if !self.contains_mapping(&node) {
            return Err(SheetEditError::NodeNotFound(node.join("/")));
        }

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
        if !self.contains_mapping(&node) {
            return Err(SheetEditError::NodeNotFound(node.join("/")));
        }

        self.edit
            .list
            .push(SheetEditItem::UpdateForward { node, forward });
        Ok(())
    }

    /// Apply changes
    pub fn apply(&mut self) -> Result<(), SheetApplyError> {
        let items = replace(&mut self.edit.list, Vec::new());
        for item in items {
            match item {
                SheetEditItem::DoNothing => continue,
                SheetEditItem::Move { from_node, to_node } => {
                    let mut moved =
                        self.data.mappings.take(&from_node).ok_or_else(|| {
                            SheetApplyError::UnexpectedNotFound(from_node.join("/"))
                        })?;
                    moved.set_value(to_node);
                    self.data.mappings.insert(moved);
                }
                SheetEditItem::Swap { node_a, node_b } => {
                    let mut a = self
                        .data
                        .mappings
                        .take(&node_a)
                        .ok_or_else(|| SheetApplyError::UnexpectedNotFound(node_a.join("/")))?;
                    let mut b = self
                        .data
                        .mappings
                        .take(&node_b)
                        .ok_or_else(|| SheetApplyError::UnexpectedNotFound(node_b.join("/")))?;
                    let val_a = a.value().clone();
                    let val_b = b.value().clone();
                    a.set_value(val_b);
                    b.set_value(val_a);
                }
                SheetEditItem::EraseMapping { node } => {
                    if !self.data.mappings.remove(&node) {
                        return Err(SheetApplyError::UnexpectedNotFound(node.join("/")));
                    }
                }
                SheetEditItem::InsertMapping { mapping } => {
                    let node = mapping.value().clone();
                    if !self.data.mappings.insert(mapping) {
                        return Err(SheetApplyError::UnexpectedAlreadyExist(node.join("/")));
                    }
                }
                SheetEditItem::ReplaceSource { node, source } => {
                    let mut target = self
                        .data
                        .mappings
                        .take(&node)
                        .ok_or_else(|| SheetApplyError::UnexpectedNotFound(node.join("/")))?;
                    target.replace_source(source);
                    self.data.mappings.insert(target);
                }
                SheetEditItem::UpdateForward { node, forward } => {
                    let mut target = self
                        .data
                        .mappings
                        .take(&node)
                        .ok_or_else(|| SheetApplyError::UnexpectedNotFound(node.join("/")))?;
                    target.replace_forward(forward);
                    self.data.mappings.insert(target);
                }
            }
        }

        // Clear the edit list
        self.edit.clear();
        Ok(())
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
