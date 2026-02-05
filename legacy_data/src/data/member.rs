use std::collections::HashMap;

use cfg_file::ConfigFile;
use serde::{Deserialize, Serialize};
use string_proc::snake_case;

pub type MemberId = String;

#[derive(Debug, Eq, Clone, ConfigFile, Serialize, Deserialize)]
pub struct Member {
    /// Member ID, the unique identifier of the member
    #[serde(rename = "id")]
    id: String,

    /// Member metadata
    #[serde(rename = "meta")]
    metadata: HashMap<String, String>,
}

impl Default for Member {
    fn default() -> Self {
        Self::new("default_user")
    }
}

impl PartialEq for Member {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl std::fmt::Display for Member {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl std::convert::AsRef<str> for Member {
    fn as_ref(&self) -> &str {
        &self.id
    }
}

impl Member {
    /// Create member struct by id
    pub fn new(new_id: impl Into<String>) -> Self {
        Self {
            id: snake_case!(new_id.into()),
            metadata: HashMap::new(),
        }
    }

    /// Get member id
    pub fn id(&self) -> String {
        self.id.clone()
    }

    /// Get metadata
    pub fn metadata(&self, key: impl Into<String>) -> Option<&String> {
        self.metadata.get(&key.into())
    }

    /// Set metadata
    pub fn set_metadata(
        &mut self,
        key: impl AsRef<str>,
        value: impl Into<String>,
    ) -> Option<String> {
        self.metadata.insert(key.as_ref().to_string(), value.into())
    }
}
