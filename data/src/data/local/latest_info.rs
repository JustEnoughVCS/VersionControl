use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    time::SystemTime,
};

use cfg_file::ConfigFile;
use serde::{Deserialize, Serialize};

use crate::{
    constants::{CLIENT_FILE_LATEST_INFO, CLIENT_FILE_LATEST_INFO_NOSET},
    data::{
        member::{Member, MemberId},
        sheet::{SheetData, SheetName, SheetPathBuf},
        vault::{
            sheet_share::{Share, SheetShareId},
            virtual_file::VirtualFileId,
        },
    },
};

const ACCOUNT: &str = "{account}";

/// # Latest Info
/// Locally cached latest information,
/// used to cache personal information from upstream for querying and quickly retrieving member information.
#[derive(Default, Serialize, Deserialize, ConfigFile)]
#[cfg_file(path = CLIENT_FILE_LATEST_INFO_NOSET)]
pub struct LatestInfo {
    // Sheets
    /// Visible sheets,
    /// indicating which sheets I can edit
    #[serde(rename = "my")]
    pub visible_sheets: Vec<SheetName>,

    /// Invisible sheets,
    /// indicating which sheets I can export files to (these sheets are not readable to me)
    #[serde(rename = "others")]
    pub invisible_sheets: Vec<SheetInfo>,

    /// Reference sheets,
    /// indicating sheets owned by the host, visible to everyone,
    /// but only the host can modify or add mappings within them
    #[serde(rename = "refsheets")]
    pub reference_sheets: HashSet<SheetName>,

    /// Reference sheet data, indicating what files I can get from the reference sheet
    #[serde(rename = "ref")]
    pub ref_sheet_content: SheetData,

    /// Reverse mapping from virtual file IDs to actual paths in reference sheets
    #[serde(rename = "ref_vfs")]
    pub ref_sheet_vfs_mapping: HashMap<VirtualFileId, SheetPathBuf>,

    /// Shares in my sheets, indicating which external merge requests have entries that I can view
    #[serde(rename = "shares")]
    pub shares_in_my_sheets: HashMap<SheetName, HashMap<SheetShareId, Share>>,

    /// Update instant
    #[serde(rename = "update")]
    pub update_instant: Option<SystemTime>,

    // Members
    /// All member information of the vault, allowing me to contact them more conveniently
    #[serde(rename = "members")]
    pub vault_members: Vec<Member>,
}

impl LatestInfo {
    /// Get the path to the latest info file for a given workspace and member ID
    pub fn latest_info_path(local_workspace_path: &Path, member_id: &MemberId) -> PathBuf {
        local_workspace_path.join(CLIENT_FILE_LATEST_INFO.replace(ACCOUNT, member_id))
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct SheetInfo {
    #[serde(rename = "name")]
    pub sheet_name: SheetName,

    #[serde(rename = "holder")]
    pub holder_name: Option<MemberId>,
}
