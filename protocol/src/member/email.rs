use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::member::error::EMailAddressParseError;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq)]
pub struct EMailAddress {
    account: String,
    domain: String,
}

impl std::hash::Hash for EMailAddress {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.account.hash(state);
        self.domain.hash(state);
    }
}

impl PartialEq for EMailAddress {
    fn eq(&self, other: &Self) -> bool {
        self.account == other.account && self.domain == other.domain
    }
}

impl Display for EMailAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.account, self.domain)
    }
}

impl TryFrom<&str> for EMailAddress {
    type Error = EMailAddressParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let trimmed_value = value.trim();
        let parts: Vec<&str> = trimmed_value.split('@').collect();
        if parts.len() != 2 {
            return Err(EMailAddressParseError::InvalidFormat);
        }
        let account = parts[0].trim().to_string();
        let domain = parts[1].trim().to_string();
        if account.is_empty() || domain.is_empty() {
            return Err(EMailAddressParseError::EmptyPart);
        }
        Ok(Self { account, domain })
    }
}

impl TryFrom<String> for EMailAddress {
    type Error = EMailAddressParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl Serialize for EMailAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for EMailAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        EMailAddress::try_from(s).map_err(serde::de::Error::custom)
    }
}
