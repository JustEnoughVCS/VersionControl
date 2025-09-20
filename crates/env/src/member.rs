use cfg_file::ConfigFile;
use serde::{Deserialize, Serialize};
use string_proc::camel_case;
use uuid::Uuid;

#[derive(Debug, Eq, Clone, ConfigFile, Serialize, Deserialize)]
pub struct Member {
    id: String,
    uuid: Uuid,
}

impl Default for Member {
    fn default() -> Self {
        Self::new("default_user")
    }
}

impl PartialEq for Member {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
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
        let uuid = Uuid::new_v4();
        Self {
            id: camel_case!(new_id.into()),
            uuid,
        }
    }

    /// Get member id
    pub fn id(&self) -> String {
        self.id.clone()
    }

    /// Get member uuid
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }
}
