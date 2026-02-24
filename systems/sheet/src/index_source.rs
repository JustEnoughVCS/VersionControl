/// IndexSource
/// Points to a unique resource address in Vault
#[derive(Debug, Clone, Copy)]
pub struct IndexSource {
    /// The index ID of the resource
    id: u32,

    /// The index version of the resource
    ver: u16,
}

// Implement construction and querying for IndexSource

impl IndexSource {
    /// Create IndexSource
    pub fn new(id: u32, ver: u16) -> Self {
        IndexSource { id, ver }
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
        &self.id == &other.id && &self.ver == &other.ver
    }
}

impl Eq for IndexSource {}

// Implement hashing for IndexSource and IndexSourceBuf

impl std::hash::Hash for IndexSource {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.ver.hash(state);
    }
}

// Implement construction of IndexSource from strings

impl<'a> TryFrom<&'a str> for IndexSource {
    type Error = &'static str;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let parts: Vec<&str> = value.split('/').collect();
        if parts.len() != 2 {
            return Err("Invalid format: expected 'id/version'");
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

        // Check for overflow (though parsing already validates range)
        // Additional bounds checks can be added here if needed
        Ok(Self { id, ver })
    }
}

impl TryFrom<String> for IndexSource {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl From<IndexSource> for (u32, u16) {
    fn from(src: IndexSource) -> Self {
        (src.id, src.ver)
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
}

// Implement Display for IndexSourceBuf and IndexSource

impl std::fmt::Display for IndexSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.id.to_string(), self.ver.to_string())
    }
}
