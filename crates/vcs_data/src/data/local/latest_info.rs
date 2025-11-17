use std::time::{SystemTime, UNIX_EPOCH};

use cfg_file::ConfigFile;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tokio::time::Instant;

use crate::{
    constants::CLIENT_FILE_LATEST_INFO,
    data::{
        member::{Member, MemberId},
        sheet::{SheetData, SheetName},
    },
};

/// # Latest Info
/// Locally cached latest information,
/// used to cache personal information from upstream for querying and quickly retrieving member information.
#[derive(Default, Serialize, Deserialize, ConfigFile)]
#[cfg_file(path = CLIENT_FILE_LATEST_INFO)]
pub struct LatestInfo {
    // Sheets
    /// My sheets, indicating which sheets I can edit
    pub my_sheets: Vec<SheetName>,
    /// Other sheets, indicating which sheets I can export files to (these sheets are not readable to me)
    pub other_sheets: Vec<SheetInfo>,
    /// Reference sheet data, indicating what files I can get from the reference sheet
    pub ref_sheet_content: SheetData,

    /// Update instant
    #[serde(
        serialize_with = "serialize_instant",
        deserialize_with = "deserialize_instant"
    )]
    pub update_instant: Option<Instant>,

    // Members
    /// All member information of the vault, allowing me to contact them more conveniently
    pub vault_members: Vec<Member>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct SheetInfo {
    pub sheet_name: SheetName,
    pub holder_name: Option<MemberId>,
}

fn serialize_instant<S>(instant: &Option<Instant>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let system_now = SystemTime::now();
    let instant_now = Instant::now();
    let duration_since_epoch = instant
        .as_ref()
        .and_then(|i| i.checked_duration_since(instant_now))
        .map(|d| system_now.checked_add(d))
        .unwrap_or(Some(system_now))
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .unwrap_or_else(|| SystemTime::now().duration_since(UNIX_EPOCH).unwrap());

    serializer.serialize_u64(duration_since_epoch.as_millis() as u64)
}

fn deserialize_instant<'de, D>(deserializer: D) -> Result<Option<Instant>, D::Error>
where
    D: Deserializer<'de>,
{
    let millis = u64::deserialize(deserializer)?;
    let duration_since_epoch = std::time::Duration::from_millis(millis);
    let system_time = UNIX_EPOCH + duration_since_epoch;
    let now_system = SystemTime::now();
    let now_instant = Instant::now();

    if let Ok(elapsed) = system_time.elapsed() {
        Ok(Some(now_instant - elapsed))
    } else if let Ok(duration_until) = system_time.duration_since(now_system) {
        Ok(Some(now_instant + duration_until))
    } else {
        Ok(Some(now_instant))
    }
}
