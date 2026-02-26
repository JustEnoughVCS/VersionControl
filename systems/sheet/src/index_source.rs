use serde::{Deserialize, Serialize};

/// IndexSource
/// Points to a unique resource address in Vault
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct IndexSource {
    remote: bool,

    /// The index ID of the resource
    id: u32,

    /// The index version of the resource
    ver: u16,
}

// Implement construction and querying for IndexSource

impl IndexSource {
    /// Create IndexSource
    pub fn new(remote: bool, id: u32, ver: u16) -> Self {
        IndexSource { remote, id, ver }
    }

    /// Create IndexSource To Local Namespace
    pub fn new_local(id: u32, ver: u16) -> Self {
        IndexSource {
            remote: false,
            id,
            ver,
        }
    }

    /// Create IndexSource To Remote Namespace
    pub fn new_remote(id: u32, ver: u16) -> Self {
        IndexSource {
            remote: true,
            id,
            ver,
        }
    }

    /// Check if the IndexSource points to a remote namespace
    pub fn is_remote(&self) -> bool {
        self.remote
    }

    /// Get index ID from IndexSource
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get index version from IndexSource
    pub fn version(&self) -> u16 {
        self.ver
    }
}

// Implement comparison for IndexSource

impl PartialEq for IndexSource {
    fn eq(&self, other: &Self) -> bool {
        &self.remote == &other.remote && &self.id == &other.id && &self.ver == &other.ver
    }
}

impl Eq for IndexSource {}

// Implement hashing for IndexSource and IndexSourceBuf

impl std::hash::Hash for IndexSource {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.remote.hash(state);
        self.id.hash(state);
        self.ver.hash(state);
    }
}

// Implement construction of IndexSource from strings

impl<'a> TryFrom<&'a str> for IndexSource {
    type Error = &'static str;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let trimmed = value.trim();
        let remote = !trimmed.starts_with('~');
        let value_without_tilde = if remote { trimmed } else { &trimmed[1..] };

        let parts: Vec<&str> = value_without_tilde.split('/').collect();
        if parts.len() != 2 {
            return Err("Invalid format: expected '[~]id/version'");
        }

        let id_str = parts[0].trim();
        let ver_str = parts[1].trim();

        if id_str.is_empty() || ver_str.is_empty() {
            return Err("ID or version cannot be empty");
        }

        let id = id_str
            .parse::<u32>()
            .map_err(|_| "ID must be a valid u32")?;
        let ver = ver_str
            .parse::<u16>()
            .map_err(|_| "Version must be a valid u16")?;

        Ok(Self { remote, id, ver })
    }
}

impl TryFrom<String> for IndexSource {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl From<IndexSource> for (bool, u32, u16) {
    fn from(src: IndexSource) -> Self {
        (src.remote, src.id, src.ver)
    }
}

// Implement modifications for IndexSource
impl IndexSource {
    /// Set the index ID of IndexSource
    pub fn set_id(&mut self, index_id: u32) {
        self.id = index_id;
    }

    /// Set the index version of IndexSource
    pub fn set_version(&mut self, version: u16) {
        self.ver = version;
    }

    /// Set the remote flag of IndexSource
    pub fn set_is_remote(&mut self, remote: bool) {
        self.remote = remote;
    }

    /// Convert IndexSource to local namespace
    pub fn to_local(&mut self) {
        self.remote = false;
    }

    /// Convert IndexSource to remote namespace
    pub fn to_remote(&mut self) {
        self.remote = true;
    }
}

// Implement Display for IndexSourceBuf and IndexSource

impl std::fmt::Display for IndexSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let local_symbol = if self.remote { "" } else { "~" };
        write!(
            f,
            "{}{}/{}",
            local_symbol,
            self.id.to_string(),
            self.ver.to_string()
        )
    }
}
