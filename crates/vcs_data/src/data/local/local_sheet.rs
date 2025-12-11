use std::{collections::HashMap, io::Error, path::PathBuf, time::SystemTime};

use ::serde::{Deserialize, Serialize};
use cfg_file::{ConfigFile, config::ConfigFile};
use string_proc::format_path::format_path;

use crate::{
    constants::CLIENT_FILE_LOCAL_SHEET_NOSET,
    data::{
        local::LocalWorkspace,
        member::MemberId,
        sheet::SheetName,
        vault::virtual_file::{VirtualFileId, VirtualFileVersion, VirtualFileVersionDescription},
    },
};

pub type LocalFilePathBuf = PathBuf;
pub type LocalSheetPathBuf = PathBuf;

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

#[derive(Debug, Default, Serialize, Deserialize, ConfigFile, Clone)]
#[cfg_file(path = CLIENT_FILE_LOCAL_SHEET_NOSET)] // Do not use LocalSheet::write or LocalSheet::read
pub struct LocalSheetData {
    /// Local file path to metadata mapping.
    #[serde(rename = "mapping")]
    pub(crate) mapping: HashMap<LocalFilePathBuf, LocalMappingMetadata>,

    pub(crate) vfs: HashMap<VirtualFileId, LocalFilePathBuf>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalMappingMetadata {
    /// Hash value generated immediately after the file is downloaded to the local workspace
    #[serde(rename = "hash")]
    pub(crate) hash_when_updated: String,

    /// Time when the file was downloaded to the local workspace
    #[serde(rename = "time")]
    pub(crate) time_when_updated: SystemTime,

    /// Size of the file when downloaded to the local workspace
    #[serde(rename = "size")]
    pub(crate) size_when_updated: u64,

    /// Version description when the file was downloaded to the local workspace
    #[serde(rename = "desc")]
    pub(crate) version_desc_when_updated: VirtualFileVersionDescription,

    /// Version when the file was downloaded to the local workspace
    #[serde(rename = "version")]
    pub(crate) version_when_updated: VirtualFileVersion,

    /// Virtual file ID corresponding to the local path
    #[serde(rename = "id")]
    pub(crate) mapping_vfid: VirtualFileId,

    /// Latest modifiy check time
    #[serde(rename = "check_time")]
    pub(crate) last_modifiy_check_time: SystemTime,

    /// Latest modifiy check result
    #[serde(rename = "modified")]
    pub(crate) last_modifiy_check_result: bool,

    /// Latest modifiy check hash result
    #[serde(rename = "current_hash")]
    pub(crate) last_modifiy_check_hash: Option<String>,
}

impl LocalSheetData {
    /// Wrap LocalSheetData into LocalSheet with workspace, member, and sheet name
    pub fn wrap_to_local_sheet<'a>(
        self,
        workspace: &'a LocalWorkspace,
        member: MemberId,
        sheet_name: SheetName,
    ) -> LocalSheet<'a> {
        LocalSheet {
            local_workspace: workspace,
            member,
            sheet_name,
            data: self,
        }
    }
}

impl LocalMappingMetadata {
    /// Create a new MappingMetaData instance
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        hash_when_updated: String,
        time_when_updated: SystemTime,
        size_when_updated: u64,
        version_desc_when_updated: VirtualFileVersionDescription,
        version_when_updated: VirtualFileVersion,
        mapping_vfid: VirtualFileId,
        last_modifiy_check_time: SystemTime,
        last_modifiy_check_result: bool,
    ) -> Self {
        Self {
            hash_when_updated,
            time_when_updated,
            size_when_updated,
            version_desc_when_updated,
            version_when_updated,
            mapping_vfid,
            last_modifiy_check_time,
            last_modifiy_check_result,
            last_modifiy_check_hash: None,
        }
    }

    /// Getter for hash_when_updated
    pub fn hash_when_updated(&self) -> &String {
        &self.hash_when_updated
    }

    /// Setter for hash_when_updated
    pub fn set_hash_when_updated(&mut self, hash: String) {
        self.hash_when_updated = hash;
    }

    /// Getter for date_when_updated
    pub fn time_when_updated(&self) -> &SystemTime {
        &self.time_when_updated
    }

    /// Setter for time_when_updated
    pub fn set_time_when_updated(&mut self, time: SystemTime) {
        self.time_when_updated = time;
    }

    /// Getter for size_when_updated
    pub fn size_when_updated(&self) -> u64 {
        self.size_when_updated
    }

    /// Setter for size_when_updated
    pub fn set_size_when_updated(&mut self, size: u64) {
        self.size_when_updated = size;
    }

    /// Getter for version_desc_when_updated
    pub fn version_desc_when_updated(&self) -> &VirtualFileVersionDescription {
        &self.version_desc_when_updated
    }

    /// Setter for version_desc_when_updated
    pub fn set_version_desc_when_updated(&mut self, version_desc: VirtualFileVersionDescription) {
        self.version_desc_when_updated = version_desc;
    }

    /// Getter for version_when_updated
    pub fn version_when_updated(&self) -> &VirtualFileVersion {
        &self.version_when_updated
    }

    /// Setter for version_when_updated
    pub fn set_version_when_updated(&mut self, version: VirtualFileVersion) {
        self.version_when_updated = version;
    }

    /// Getter for mapping_vfid
    pub fn mapping_vfid(&self) -> &VirtualFileId {
        &self.mapping_vfid
    }

    /// Setter for mapping_vfid
    pub fn set_mapping_vfid(&mut self, vfid: VirtualFileId) {
        self.mapping_vfid = vfid;
    }

    /// Getter for last_modifiy_check_time
    pub fn last_modifiy_check_time(&self) -> &SystemTime {
        &self.last_modifiy_check_time
    }

    /// Setter for last_modifiy_check_time
    pub fn set_last_modifiy_check_time(&mut self, time: SystemTime) {
        self.last_modifiy_check_time = time;
    }

    /// Getter for last_modifiy_check_result
    pub fn last_modifiy_check_result(&self) -> bool {
        self.last_modifiy_check_result
    }

    /// Setter for last_modifiy_check_result
    pub fn set_last_modifiy_check_result(&mut self, result: bool) {
        self.last_modifiy_check_result = result;
    }
}

impl Default for LocalMappingMetadata {
    fn default() -> Self {
        Self {
            hash_when_updated: Default::default(),
            time_when_updated: SystemTime::now(),
            size_when_updated: Default::default(),
            version_desc_when_updated: Default::default(),
            version_when_updated: Default::default(),
            mapping_vfid: Default::default(),
            last_modifiy_check_time: SystemTime::now(),
            last_modifiy_check_result: false,
            last_modifiy_check_hash: None,
        }
    }
}

mod instant_serde {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use tokio::time::Instant;

    pub fn serialize<S>(instant: &Instant, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(instant.elapsed().as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Instant, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Instant::now() - std::time::Duration::from_secs(secs))
    }
}

impl<'a> From<&'a LocalSheet<'a>> for &'a LocalSheetData {
    fn from(sheet: &'a LocalSheet<'a>) -> Self {
        &sheet.data
    }
}

impl<'a> LocalSheet<'a> {
    /// Add mapping to local sheet data
    pub fn add_mapping(
        &mut self,
        path: &LocalFilePathBuf,
        mapping: LocalMappingMetadata,
    ) -> Result<(), std::io::Error> {
        let path = format_path(path)?;
        if self.data.mapping.contains_key(&path)
            || self.data.vfs.contains_key(&mapping.mapping_vfid)
        {
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
        from: &LocalFilePathBuf,
        to: &LocalFilePathBuf,
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

    /// Get immutable mapping data
    pub fn mapping_data(
        &self,
        path: &LocalFilePathBuf,
    ) -> Result<&LocalMappingMetadata, std::io::Error> {
        let path = format_path(path)?;
        let Some(data) = self.data.mapping.get(&path) else {
            return Err(Error::new(
                std::io::ErrorKind::NotFound,
                "Path is not found.",
            ));
        };
        Ok(data)
    }

    /// Get muttable mapping data
    pub fn mapping_data_mut(
        &mut self,
        path: &LocalFilePathBuf,
    ) -> Result<&mut LocalMappingMetadata, std::io::Error> {
        let path = format_path(path)?;
        let Some(data) = self.data.mapping.get_mut(&path) else {
            return Err(Error::new(
                std::io::ErrorKind::NotFound,
                "Path is not found.",
            ));
        };
        Ok(data)
    }

    /// Write the sheet to disk
    pub async fn write(&mut self) -> Result<(), std::io::Error> {
        let path = self
            .local_workspace
            .local_sheet_path(&self.member, &self.sheet_name);
        self.write_to_path(path).await
    }

    /// Write the sheet to custom path
    pub async fn write_to_path(&mut self, path: impl Into<PathBuf>) -> Result<(), std::io::Error> {
        let path = path.into();

        self.data.vfs = HashMap::new();
        for (path, mapping) in self.data.mapping.iter() {
            self.data
                .vfs
                .insert(mapping.mapping_vfid.clone(), path.clone());
        }

        LocalSheetData::write_to(&self.data, path).await?;
        Ok(())
    }

    /// Get path by VirtualFileId
    pub fn path_by_id(&self, vfid: &VirtualFileId) -> Option<&PathBuf> {
        self.data.vfs.get(vfid)
    }
}
