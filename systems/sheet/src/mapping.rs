use string_proc::{
    format_path::{PathFormatConfig, format_path_str, format_path_str_with_config},
    snake_case,
};

/// Local mapping
/// It is stored inside a Sheet and will be exposed externally as Mapping or MappingBuf
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LocalMapping {
    /// The value of the local mapping
    val: Vec<String>,

    /// The ID of the local mapping
    id: String,

    /// The version of the local mapping
    ver: String,

    /// The version direction of the local mapping
    forward: LocalMappingForward,
}

/// The forward direction of the current Mapping
/// It indicates the expected asset update method for the current Mapping
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum LocalMappingForward {
    /// Expect the current index version to be the latest
    Latest,

    /// Expect the current index version to point to a specific Ref
    /// Note: When the Ref points to a Sheet that does not have this index,
    ///       its Forward will become `Version(current_version)`
    Ref { sheet_name: String },

    /// Expect the current index version to point to a specific version
    Version { version_name: String },
}

/// Mapping
/// It stores basic mapping information and only participates in comparison and parsing
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Mapping<'a> {
    sheet_name: &'a str,
    val: &'a str,
    id: &'a str,
    ver: &'a str,
}

/// MappingBuf
/// It stores complete mapping information and participates in complex mapping editing operations like storage and modification
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MappingBuf {
    sheet_name: String,
    val: Vec<String>,
    val_joined: String,
    id: String,
    ver: String,
}

// Implement creation and mutual conversion for MappingBuf, LocalMapping and Mapping

impl LocalMapping {
    /// Create a new LocalMapping
    pub fn new(
        val: Vec<String>,
        id: impl Into<String>,
        ver: impl Into<String>,
        forward: LocalMappingForward,
    ) -> Self {
        Self {
            val,
            id: id.into(),
            ver: ver.into(),
            forward,
        }
    }

    /// Get the path value of LocalMapping
    pub fn value(&self) -> &Vec<String> {
        &self.val
    }

    /// Get the mapped index ID of LocalMapping
    pub fn mapped_id(&self) -> &String {
        &self.id
    }

    /// Get the mapped index version of LocalMapping
    pub fn mapped_version(&self) -> &String {
        &self.ver
    }

    /// Get the forward direction of LocalMapping
    pub fn forward(&self) -> &LocalMappingForward {
        &self.forward
    }

    /// Clone and generate a MappingBuf from LocalMapping
    pub fn to_mapping_buf_cloned(&self, sheet_name: impl Into<String>) -> MappingBuf {
        MappingBuf::new(
            sheet_name.into(),
            self.val.clone(),
            self.id.clone(),
            self.ver.clone(),
        )
    }

    /// Generate a MappingBuf from LocalMapping
    pub fn to_mapping_buf(self, sheet_name: impl Into<String>) -> MappingBuf {
        MappingBuf::new(sheet_name.into(), self.val, self.id, self.ver)
    }
}

impl MappingBuf {
    /// Create a new MappingBuf
    pub fn new(
        sheet_name: impl Into<String>,
        val: Vec<String>,
        id: impl Into<String>,
        ver: impl Into<String>,
    ) -> Self {
        let val_joined = val.join("/");
        Self {
            sheet_name: sheet_name.into(),
            val,
            val_joined,
            id: id.into(),
            ver: ver.into(),
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

    /// Get the mapped index ID of MappingBuf
    pub fn mapped_id(&self) -> &String {
        &self.id
    }

    /// Get the mapped index version of MappingBuf
    pub fn mapped_version(&self) -> &String {
        &self.ver
    }

    /// Generate a Mapping from MappingBuf
    pub fn as_mapping(&self) -> Mapping<'_> {
        Mapping::new(&self.sheet_name, &self.val_joined, &self.id, &self.ver)
    }

    /// Clone and generate a LocalMapping from MappingBuf
    pub fn to_local_mapping_cloned(&self, forward: &LocalMappingForward) -> LocalMapping {
        LocalMapping::new(
            self.val.clone(),
            self.id.clone(),
            self.ver.clone(),
            forward.clone(),
        )
    }

    /// Generate a LocalMapping from MappingBuf
    pub fn to_local_mapping(self, forward: LocalMappingForward) -> LocalMapping {
        LocalMapping::new(self.val, self.id, self.ver, forward)
    }
}

impl<'a> Mapping<'a> {
    /// Create a new Mapping
    pub fn new(sheet_name: &'a str, val: &'a str, id: &'a str, ver: &'a str) -> Self {
        Self {
            sheet_name,
            val,
            id,
            ver,
        }
    }

    /// Get the sheet name of Mapping
    pub fn sheet_name(&self) -> &str {
        &self.sheet_name
    }

    /// Build a Vec of Mapping values from the stored address
    pub fn value(&self) -> Vec<String> {
        format_path_str(self.val.to_string())
            .unwrap_or_default()
            .split("/")
            .map(|s| s.to_string())
            .collect()
    }

    /// Get the value str of Mapping
    pub fn value_str(&self) -> &str {
        &self.val
    }

    /// Get the mapped index ID of Mapping
    pub fn mapped_id(&self) -> &str {
        &self.id
    }

    /// Get the mapped index version of Mapping
    pub fn mapped_version(&self) -> &str {
        &self.ver
    }

    /// Generate a MappingBuf from Mapping
    pub fn to_mapping_buf(&self) -> MappingBuf {
        MappingBuf::new(
            self.sheet_name.to_string(),
            format_path_str(self.val)
                .unwrap_or_default()
                .split('/')
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            self.id.to_string(),
            self.ver.to_string(),
        )
    }

    /// Generate a LocalMapping from MappingBuf
    pub fn to_local_mapping(self, forward: LocalMappingForward) -> LocalMapping {
        LocalMapping::new(
            format_path_str(self.val)
                .unwrap_or_default()
                .split("/")
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            self.id.to_string(),
            self.ver.to_string(),
            forward,
        )
    }
}

impl<'a> From<Mapping<'a>> for MappingBuf {
    fn from(mapping: Mapping<'a>) -> Self {
        mapping.to_mapping_buf()
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
    ($f:expr, $sheet_name:expr, $val:expr) => {
        write!(
            $f,
            "{}:/{}",
            snake_case!($sheet_name),
            format_path_str($val).unwrap_or_default()
        )
    };
}

impl<'a> std::fmt::Display for Mapping<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt_mapping!(f, self.sheet_name, self.val)
    }
}

impl std::fmt::Display for MappingBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt_mapping!(f, self.sheet_name.to_string(), &self.val.join("/"))
    }
}

impl std::fmt::Display for LocalMapping {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.val.join("/"))
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

    /// Set the mapped index ID of the current MappingBuf
    pub fn set_mapped_id(&mut self, id: impl Into<String>) {
        self.id = id.into();
    }

    /// Set the mapped index version of the current MappingBuf
    pub fn set_mapped_version(&mut self, version: impl Into<String>) {
        self.ver = version.into();
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

    /// Set the mapped index ID of the current LocalMapping
    pub fn set_mapped_id(&mut self, id: impl Into<String>) {
        self.id = id.into();
    }

    /// Set the mapped index version of the current LocalMapping
    pub fn set_mapped_version(&mut self, version: impl Into<String>) {
        self.ver = version.into();
    }

    /// Set the forward direction of the current LocalMapping
    pub fn set_forward(&mut self, forward: &LocalMappingForward) {
        self.forward = forward.clone();
    }
}

#[inline(always)]
fn join_helper(nodes: String, mut mapping_buf_val: Vec<String>) -> Vec<String> {
    let formatted = format_path_str_with_config(
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

// Implement mutual comparison for LocalMapping, MappingBuf, and Mapping

impl<'a> PartialEq<Mapping<'a>> for LocalMapping {
    fn eq(&self, other: &Mapping<'a>) -> bool {
        self.val.join("/") == other.val && self.id == other.id && self.ver == other.ver
    }
}

impl<'a> PartialEq<LocalMapping> for Mapping<'a> {
    fn eq(&self, other: &LocalMapping) -> bool {
        other == self
    }
}

impl PartialEq<MappingBuf> for LocalMapping {
    fn eq(&self, other: &MappingBuf) -> bool {
        self.val == other.val && self.id == other.id && self.ver == other.ver
    }
}

impl PartialEq<LocalMapping> for MappingBuf {
    fn eq(&self, other: &LocalMapping) -> bool {
        other == self
    }
}

impl<'a> PartialEq<MappingBuf> for Mapping<'a> {
    fn eq(&self, other: &MappingBuf) -> bool {
        self.val == other.val_joined && self.id == other.id && self.ver == other.ver
    }
}

impl<'a> PartialEq<Mapping<'a>> for MappingBuf {
    fn eq(&self, other: &Mapping<'a>) -> bool {
        other == self
    }
}
