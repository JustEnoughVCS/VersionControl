use just_fmt::fmt_path::{PathFormatConfig, fmt_path_str, fmt_path_str_custom};

use crate::{index_source::IndexSource, mapping::error::ParseMappingError};

pub mod error;

// Validation rules for LocalMapping
// LocalMapping is a key component for writing and reading SheetData
// According to the SheetData protocol specification, all variable-length fields store length information using the u8 type
// Therefore, the lengths of the index_id, version, mapping_value, and forward fields must not exceed `u8::MAX`

/// Local mapping
/// It is stored inside a Sheet and will be exposed externally as Mapping or MappingBuf
#[derive(Debug, Clone)]
pub struct LocalMapping {
    /// The value of the local mapping
    val: Vec<String>,

    /// Index source
    source: IndexSource,

    /// The version direction of the local mapping
    forward: LocalMappingForward,
}

/// The forward direction of the current Mapping
/// It indicates the expected asset update method for the current Mapping
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum LocalMappingForward {
    /// Expect the current index version to be the latest
    #[default]
    Latest,

    /// Expect the current index version to point to a specific Ref
    /// Note: When the Ref points to a Sheet that does not have this index,
    ///       its Forward will become `Version(current_version)`
    Ref { sheet_name: String },

    /// Expect the current index version to point to a specific version
    Version { version: u16 },
}

/// Mapping
/// It stores basic mapping information and only participates in comparison and parsing
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Mapping<'a> {
    sheet_name: &'a str,
    val: &'a str,
    source: IndexSource,
}

/// MappingBuf
/// It stores complete mapping information and participates in complex mapping editing operations like storage and modification
#[derive(Debug, PartialEq, Clone)]
pub struct MappingBuf {
    sheet_name: String,
    val: Vec<String>,
    val_joined: String,
    source: IndexSource,
}

// Implement conversions for LocalMappingForward

impl LocalMappingForward {
    /// Check if the forward direction length is valid
    pub fn is_len_valid(&self) -> bool {
        match self {
            LocalMappingForward::Latest => true,
            LocalMappingForward::Ref { sheet_name } => sheet_name.len() <= u8::MAX as usize,
            LocalMappingForward::Version { version: _ } => true,
        }
    }

    /// Get the forward direction type ID
    pub fn forward_type_id(&self) -> u8 {
        match self {
            LocalMappingForward::Latest => 0,
            LocalMappingForward::Ref { sheet_name: _ } => 1,
            LocalMappingForward::Version { version: _ } => 2,
        }
    }

    /// Unpack into raw information
    /// (id, len, bytes)
    pub fn unpack(&self) -> (u8, u8, Vec<u8>) {
        let b = match self {
            LocalMappingForward::Latest => vec![],
            LocalMappingForward::Ref { sheet_name } => sheet_name.as_bytes().to_vec(),
            LocalMappingForward::Version {
                version: version_name,
            } => version_name.to_be_bytes().to_vec(),
        };
        (self.forward_type_id(), b.len() as u8, b)
    }

    /// Reconstruct into a forward direction
    pub fn pack(id: u8, bytes: &[u8]) -> Option<Self> {
        if bytes.len() > u8::MAX as usize {
            return None;
        }
        match id {
            0 => Some(Self::Latest),
            1 => Some(Self::Ref {
                sheet_name: String::from_utf8(bytes.to_vec()).ok()?,
            }),
            2 => Some(Self::Version {
                version: u16::from_be_bytes(bytes.try_into().ok()?),
            }),
            _ => None,
        }
    }
}

// Implement creation and mutual conversion for MappingBuf, LocalMapping and Mapping

impl LocalMapping {
    /// Create a new LocalMapping
    pub fn new(
        val: Vec<String>,
        source: IndexSource,
        forward: LocalMappingForward,
    ) -> Option<Self> {
        // Note:
        // LocalMapping will be stored in SheetData
        // Strict validity checks are required; the lengths of forward, and even val must not exceed limits
        // Otherwise, errors will occur that prevent correct writing to SheetData
        let valid = forward.is_len_valid() && val.join("/").len() <= u8::MAX as usize;

        if valid {
            Some(Self {
                val,
                source,
                forward,
            })
        } else {
            None
        }
    }

    /// Get the path value of LocalMapping
    pub fn value(&self) -> &Vec<String> {
        &self.val
    }

    /// Get the IndexSource of LocalMapping
    pub fn index_source(&self) -> IndexSource {
        self.source
    }

    /// Get the mapped index ID of LocalMapping
    pub fn mapped_id(&self) -> u32 {
        self.source.id()
    }

    /// Get the mapped index version of LocalMapping
    pub fn mapped_version(&self) -> u16 {
        self.source.version()
    }

    /// Get the forward direction of LocalMapping
    pub fn forward(&self) -> &LocalMappingForward {
        &self.forward
    }

    /// Clone and generate a MappingBuf from LocalMapping
    pub fn to_mapping_buf_cloned(&self, sheet_name: impl Into<String>) -> MappingBuf {
        MappingBuf::new(sheet_name.into(), self.val.clone(), self.source.clone())
    }

    /// Generate a MappingBuf from LocalMapping
    pub fn to_mapping_buf(self, sheet_name: impl Into<String>) -> MappingBuf {
        MappingBuf::new(sheet_name.into(), self.val, self.source)
    }
}

impl MappingBuf {
    /// Create a new MappingBuf
    pub fn new(sheet_name: impl Into<String>, val: Vec<String>, source: IndexSource) -> Self {
        let val_joined = val.join("/");
        Self {
            sheet_name: sheet_name.into(),
            val,
            val_joined,
            source,
        }
    }

    /// Get the sheet name of MappingBuf
    pub fn sheet_name(&self) -> &String {
        &self.sheet_name
    }

    /// Get the path value of MappingBuf
    pub fn value(&self) -> &Vec<String> {
        &self.val
    }

    /// Get the path value string of MappingBuf
    pub fn value_str(&self) -> &String {
        &self.val_joined
    }

    /// Get the IndexSource of MappingBuf
    pub fn index_source(&self) -> IndexSource {
        self.source
    }

    /// Get the mapped index ID of MappingBuf
    pub fn mapped_id(&self) -> u32 {
        self.source.id()
    }

    /// Get the mapped index version of MappingBuf
    pub fn mapped_version(&self) -> u16 {
        self.source.version()
    }

    /// Generate a Mapping from MappingBuf
    pub fn as_mapping(&self) -> Mapping<'_> {
        Mapping::new(&self.sheet_name, &self.val_joined, self.source)
    }

    /// Clone and generate a LocalMapping from MappingBuf
    ///
    /// If any of the following conditions exist in MappingBuf or its members,
    /// the conversion will be invalid and return None
    /// - The final length of the recorded mapping value exceeds `u8::MAX`
    ///
    /// Additionally, if the length of the given forward value exceeds `u8::MAX`, it will also return None
    pub fn to_local_mapping_cloned(&self, forward: &LocalMappingForward) -> Option<LocalMapping> {
        LocalMapping::new(self.val.clone(), self.source.clone(), forward.clone())
    }

    /// Generate a LocalMapping from MappingBuf
    ///
    /// If any of the following conditions exist in MappingBuf or its members,
    /// the conversion will be invalid and return None
    /// - The final length of the recorded mapping value exceeds `u8::MAX`
    ///
    /// Additionally, if the length of the given forward value exceeds `u8::MAX`, it will also return None
    pub fn to_local_mapping(self, forward: LocalMappingForward) -> Option<LocalMapping> {
        LocalMapping::new(self.val, self.source, forward)
    }
}

impl<'a> Mapping<'a> {
    /// Create a new Mapping
    pub fn new(sheet_name: &'a str, val: &'a str, source: IndexSource) -> Self {
        Self {
            sheet_name,
            val,
            source,
        }
    }

    /// Get the sheet name of Mapping
    pub fn sheet_name(&self) -> &str {
        &self.sheet_name
    }

    /// Build a Vec of Mapping values from the stored address
    pub fn value(&self) -> Vec<String> {
        fmt_path_str(self.val.to_string())
            .unwrap_or_default()
            .split("/")
            .map(|s| s.to_string())
            .collect()
    }

    /// Get the value str of Mapping
    pub fn value_str(&self) -> &str {
        &self.val
    }

    /// Get the IndexSource of Mapping
    pub fn index_source(&self) -> IndexSource {
        self.source
    }

    /// Get the mapped index ID of Mapping
    pub fn mapped_id(&self) -> u32 {
        self.source.id()
    }

    /// Get the mapped index version of Mapping
    pub fn mapped_version(&self) -> u16 {
        self.source.version()
    }

    /// Generate a MappingBuf from Mapping
    pub fn to_mapping_buf(&self) -> MappingBuf {
        MappingBuf::new(
            self.sheet_name.to_string(),
            fmt_path_str(self.val)
                .unwrap_or_default()
                .split('/')
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            self.source,
        )
    }

    /// Generate a LocalMapping from Mapping
    ///
    /// If any of the following conditions exist in Mapping or its members,
    /// the conversion will be invalid and return None
    /// - The final length of the recorded mapping value exceeds `u8::MAX`
    ///
    /// Additionally, if the length of the given forward value exceeds `u8::MAX`, it will also return None
    pub fn to_local_mapping(self, forward: LocalMappingForward) -> Option<LocalMapping> {
        LocalMapping::new(
            fmt_path_str(self.val)
                .unwrap_or_default()
                .split("/")
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            self.source,
            forward,
        )
    }
}

impl<'a> From<Mapping<'a>> for Vec<String> {
    fn from(mapping: Mapping<'a>) -> Vec<String> {
        mapping.value()
    }
}

impl<'a> From<Mapping<'a>> for MappingBuf {
    fn from(mapping: Mapping<'a>) -> Self {
        mapping.to_mapping_buf()
    }
}

impl<'a> TryFrom<Mapping<'a>> for LocalMapping {
    type Error = ParseMappingError;

    fn try_from(value: Mapping<'a>) -> Result<Self, Self::Error> {
        match value.to_local_mapping(LocalMappingForward::Latest) {
            Some(m) => Ok(m),
            None => Err(ParseMappingError::InvalidMapping),
        }
    }
}

impl TryFrom<MappingBuf> for LocalMapping {
    type Error = ParseMappingError;

    fn try_from(value: MappingBuf) -> Result<Self, Self::Error> {
        match value.to_local_mapping(LocalMappingForward::Latest) {
            Some(m) => Ok(m),
            None => Err(ParseMappingError::InvalidMapping),
        }
    }
}

// Implement the Display trait for Mapping, LocalMapping and MappingBuf for formatted output.
//
// The Display implementation only shows path information, not the complete structure information.
// Why?
//
// Because Display is primarily used for user-friendly presentation, not for internal program use.
// When presenting, only the snake_case converted sheet_name and the path formed by joining val are shown.

macro_rules! fmt_mapping {
    ($f:expr, $sheet_name:expr, $val:expr, $source:expr) => {
        write!(
            $f,
            "\"{}:/{}\" => \"{}\"",
            just_fmt::snake_case!($sheet_name),
            just_fmt::fmt_path::fmt_path_str($val).unwrap_or_default(),
            $source
        )
    };
}

impl<'a> std::fmt::Display for Mapping<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt_mapping!(f, self.sheet_name, self.val, self.source.to_string())
    }
}

impl std::fmt::Display for MappingBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt_mapping!(
            f,
            self.sheet_name.to_string(),
            &self.val.join("/"),
            self.source.to_string()
        )
    }
}

impl std::fmt::Display for LocalMapping {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.forward {
            LocalMappingForward::Latest => {
                write!(
                    f,
                    "\"{}\" => \"{}\"",
                    self.val.join("/"),
                    self.source.to_string()
                )
            }
            LocalMappingForward::Ref { sheet_name } => {
                write!(
                    f,
                    "\"{}\" => \"{}\" => \"{}\"",
                    self.val.join("/"),
                    self.source.to_string(),
                    sheet_name
                )
            }
            LocalMappingForward::Version { version } => {
                if &self.mapped_version() == version {
                    write!(
                        f,
                        "\"{}\" == \"{}\"",
                        self.val.join("/"),
                        self.source.to_string(),
                    )
                } else {
                    write!(
                        f,
                        "\"{}\" => \"{}\" == \"{}\"",
                        self.val.join("/"),
                        self.source.to_string(),
                        version
                    )
                }
            }
        }
    }
}

// Implement editing functionality for MappingBuf and LocalMapping

impl MappingBuf {
    /// Append new nodes to the end of MappingBuf to modify the path
    pub fn join(mut self, nodes: impl Into<String>) -> Self {
        let nodes = nodes.into();
        let mapping_buf_val = join_helper(nodes, self.val);
        self.val_joined = mapping_buf_val.join("/");
        self.val = mapping_buf_val;
        self
    }

    /// Set the sheet name of the current MappingBuf
    pub fn set_sheet_name(&mut self, sheet_name: impl Into<String>) {
        self.sheet_name = sheet_name.into();
    }

    /// Set the value of the current MappingBuf
    pub fn set_value(&mut self, val: Vec<String>) {
        self.val = val;
        self.val_joined = self.val.join("/");
    }

    /// Replace the current IndexSource
    pub fn replace_source(&mut self, new_source: IndexSource) {
        self.source = new_source;
    }

    /// Set the mapped index ID of the current MappingBuf
    pub fn set_mapped_id(&mut self, id: u32) {
        self.source.set_id(id);
    }

    /// Set the mapped index version of the current MappingBuf
    pub fn set_mapped_version(&mut self, version: u16) {
        self.source.set_version(version);
    }
}

impl LocalMapping {
    /// Append new nodes to the end of MappingBuf to modify the path
    pub fn join(mut self, nodes: impl Into<String>) -> Self {
        let nodes = nodes.into();
        let mapping_buf_val = join_helper(nodes, self.val);
        self.val = mapping_buf_val;
        self
    }

    /// Set the value of the current LocalMapping
    pub fn set_value(&mut self, val: Vec<String>) {
        self.val = val;
    }

    /// Replace the current IndexSource
    pub fn replace_source(&mut self, new_source: IndexSource) {
        self.source = new_source;
    }

    /// Set the mapped index ID of the current LocalMapping
    pub fn set_mapped_id(&mut self, id: u32) {
        self.source.set_id(id);
    }

    /// Set the mapped index version of the current LocalMapping
    pub fn set_mapped_version(&mut self, version: u16) {
        self.source.set_version(version);
    }

    /// Set the forward direction of the current LocalMapping
    pub fn set_forward(&mut self, forward: &LocalMappingForward) {
        self.forward = forward.clone();
    }

    /// Replace the forward direction of the current LocalMapping
    pub fn replace_forward(&mut self, forward: LocalMappingForward) {
        self.forward = forward;
    }
}

#[inline(always)]
fn join_helper(nodes: String, mut mapping_buf_val: Vec<String>) -> Vec<String> {
    let formatted = fmt_path_str_custom(
        nodes,
        &PathFormatConfig {
            // Do not process ".." because it is used to go up one level
            resolve_parent_dirs: false,
            ..Default::default()
        },
    )
    .unwrap_or_default();
    let sliced_nodes = formatted.split('/');
    for node in sliced_nodes.into_iter() {
        match node {
            "." => continue,
            ".." => {
                // If the length of Mapping is greater than 1, remove the last item
                if mapping_buf_val.len() > 1 {
                    let _ = mapping_buf_val.remove(mapping_buf_val.len() - 1);
                }
            }
            _ => {
                mapping_buf_val.push(node.to_string());
            }
        }
    }

    return mapping_buf_val;
}

// Implement mutual comparison for MappingBuf and Mapping
//
// Note:
// When either side's ID or Version is None, it indicates an invalid Source
// The comparison result is false

impl<'a> PartialEq<MappingBuf> for Mapping<'a> {
    fn eq(&self, other: &MappingBuf) -> bool {
        self.val == other.val_joined
            && self.source.id() == other.source.id()
            && self.source.version() == other.source.version()
    }
}

// Implement comparison between LocalMappings
//
// Note:
// LocalMappings are considered equal as long as their val (Node) values are the same

impl PartialEq for LocalMapping {
    fn eq(&self, other: &Self) -> bool {
        self.val == other.val
    }
}

impl PartialEq<Vec<String>> for LocalMapping {
    fn eq(&self, other: &Vec<String>) -> bool {
        &self.val == other
    }
}

impl Eq for LocalMapping {}

impl std::hash::Hash for LocalMapping {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.val.hash(state);
    }
}

// Implement borrowing for LocalMapping and MappingBuf

impl std::borrow::Borrow<Vec<String>> for LocalMapping {
    fn borrow(&self) -> &Vec<String> {
        &self.val
    }
}

impl std::borrow::Borrow<Vec<String>> for MappingBuf {
    fn borrow(&self) -> &Vec<String> {
        &self.val
    }
}

/// Convert a string into a Node suitable for Mapping
///
/// # Examples
///
/// ```
/// # use sheet_system::mapping::node;
/// assert_eq!(
///     node("./home/path1/"),
///     vec!["home".to_string(), "path1".to_string(), "".to_string()]
/// );
/// assert_eq!(
///     node("./home/path2"),
///     vec!["home".to_string(), "path2".to_string()]
/// );
/// assert_eq!(
///     node(".\\home\\path3\\"),
///     vec!["home".to_string(), "path3".to_string(), "".to_string()]
/// );
/// assert_eq!(
///     node("home/path4"),
///     vec!["home".to_string(), "path4".to_string()]
/// );
/// ```
pub fn node(str: impl Into<String>) -> Vec<String> {
    let str = just_fmt::fmt_path::fmt_path_str(str).unwrap_or_default();
    str.split("/").into_iter().map(|f| f.to_string()).collect()
}
