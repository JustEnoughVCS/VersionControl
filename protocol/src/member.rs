use crate::member::email::EMailAddress;
use serde::{Deserialize, Serialize};

pub mod email;
pub mod error;

#[derive(Debug, Clone, Eq, Serialize, Deserialize)]
pub struct Member {
    name: String,
    mail: EMailAddress,
}

impl PartialEq for Member {
    fn eq(&self, other: &Self) -> bool {
        self.mail == other.mail
    }
}

impl std::hash::Hash for Member {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.mail.hash(state);
    }
}

impl std::fmt::Display for Member {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}
