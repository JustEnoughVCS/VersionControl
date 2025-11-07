use std::{collections::HashMap, io::Error, path::PathBuf};

use ::serde::{Deserialize, Serialize};
use cfg_file::{ConfigFile, config::ConfigFile};
use chrono::NaiveDate;
use string_proc::format_path::format_path;

use crate::{
    constants::CLIENT_FILE_LOCAL_SHEET_NOSET,
    data::{
        local::LocalWorkspace,
        member::MemberId,
        vault::virtual_file::{VirtualFileId, VirtualFileVersionDescription},
    },
};

pub type LocalFilePathBuf = PathBuf;

/// # Local Sheet
/// Local sheet information, used to record metadata of actual local files,
/// to compare with upstream information for more optimized file submission,
/// and to determine whether files need to be updated or submitted.
pub struct LocalSheet<'a> {
    pub(crate) local_workspace: &'a LocalWorkspace,
    pub(crate) member: MemberId,
    pub(crate) sheet_name: String,
    pub(crate) data: LocalSheetData,
}

#[derive(Debug, Default, Serialize, Deserialize, ConfigFile)]
#[cfg_file(path = CLIENT_FILE_LOCAL_SHEET_NOSET)] // Do not use LocalSheet::write or LocalSheet::read
pub struct LocalSheetData {
    /// Local file path to virtual file ID mapping.
    #[serde(rename = "mapping")]
    pub(crate) mapping: HashMap<LocalFilePathBuf, MappingMetaData>, // Path to VFID
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MappingMetaData {
    /// Hash value generated immediately after the file is downloaded to the local workspace
    #[serde(rename = "hash")]
    pub(crate) hash_when_updated: String,

    /// Time when the file was downloaded to the local workspace
    #[serde(rename = "date", with = "naive_date_serde")]
    pub(crate) date_when_updated: NaiveDate,

    /// Size of the file when downloaded to the local workspace
    #[serde(rename = "size")]
    pub(crate) size_when_updated: u64,

    /// Version description when the file was downloaded to the local workspace
    #[serde(rename = "version")]
    pub(crate) version_desc_when_updated: VirtualFileVersionDescription,

    /// Virtual file ID corresponding to the local path
    #[serde(rename = "id")]
    pub(crate) mapping_vfid: VirtualFileId,
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

impl<'a> LocalSheet<'a> {
    /// Add mapping to local sheet data
    pub fn add_mapping(
        &mut self,
        path: LocalFilePathBuf,
        mapping: MappingMetaData,
    ) -> Result<(), std::io::Error> {
        let path = format_path(path)?;
        if self.data.mapping.contains_key(&path) {
            return Err(Error::new(
                std::io::ErrorKind::AlreadyExists,
                "Mapping already exists",
            ));
        }

        self.data.mapping.insert(path, mapping);
        Ok(())
    }

    /// Move mapping to other path
    pub fn move_mapping(
        &mut self,
        from: LocalFilePathBuf,
        to: LocalFilePathBuf,
    ) -> Result<(), std::io::Error> {
        let from = format_path(from)?;
        let to = format_path(to)?;
        if self.data.mapping.contains_key(&to) {
            return Err(Error::new(
                std::io::ErrorKind::AlreadyExists,
                "To path already exists.",
            ));
        }

        let Some(old_value) = self.data.mapping.remove(&from) else {
            return Err(Error::new(
                std::io::ErrorKind::NotFound,
                "From path is not found.",
            ));
        };

        self.data.mapping.insert(to, old_value);

        Ok(())
    }

    /// Get muttable mapping data
    pub fn mapping_data_mut(
        &mut self,
        path: LocalFilePathBuf,
    ) -> Result<&mut MappingMetaData, std::io::Error> {
        let path = format_path(path)?;
        let Some(data) = self.data.mapping.get_mut(&path) else {
            return Err(Error::new(
                std::io::ErrorKind::NotFound,
                "Path is not found.",
            ));
        };
        Ok(data)
    }

    /// Persist the sheet to disk
    pub async fn persist(&mut self) -> Result<(), std::io::Error> {
        let _path = self
            .local_workspace
            .local_sheet_path(&self.member, &self.sheet_name);
        LocalSheetData::write_to(&self.data, &self.sheet_name).await?;
        Ok(())
    }
}
