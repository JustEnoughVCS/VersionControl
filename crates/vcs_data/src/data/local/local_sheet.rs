use std::{collections::HashMap, path::PathBuf};

use ::serde::{Deserialize, Serialize};
use cfg_file::ConfigFile;
use chrono::NaiveDate;

use crate::{
    constants::CLIENT_FILE_LOCAL_SHEET_NOSET,
    data::vault::virtual_file::{VirtualFileId, VirtualFileVersionDescription},
};

pub type LocalFilePathBuf = PathBuf;

#[derive(Debug, Default, Serialize, Deserialize, ConfigFile)]
#[cfg_file(path = CLIENT_FILE_LOCAL_SHEET_NOSET)] // Do not use LocalSheet::write or LocalSheet::read
pub struct LocalSheet {
    /// Local file path to virtual file ID mapping.
    #[serde(rename = "mapping")]
    mapping: HashMap<LocalFilePathBuf, MappingMetaData>, // Path to VFID
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MappingMetaData {
    /// Hash value generated immediately after the file is downloaded to the local workspace
    #[serde(rename = "hash")]
    hash_when_updated: String,

    /// Time when the file was downloaded to the local workspace
    #[serde(rename = "date", with = "naive_date_serde")]
    date_when_updated: NaiveDate,

    /// Size of the file when downloaded to the local workspace
    #[serde(rename = "size")]
    size_when_updated: u64,

    /// Version description when the file was downloaded to the local workspace
    #[serde(rename = "version")]
    version_desc_when_updated: VirtualFileVersionDescription,

    /// Virtual file ID corresponding to the local path
    #[serde(rename = "id")]
    mapping_vfid: VirtualFileId,
}

mod naive_date_serde {
    use chrono::NaiveDate;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &NaiveDate, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&date.format("%Y-%m-%d").to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveDate::parse_from_str(&s, "%Y-%m-%d").map_err(serde::de::Error::custom)
    }
}
