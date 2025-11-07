use cfg_file::ConfigFile;
use serde::{Deserialize, Serialize};

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

    // Members
    /// All member information of the vault, allowing me to contact them more conveniently
    pub vault_members: Vec<Member>,
}

impl LatestInfo {}

#[derive(Default, Serialize, Deserialize)]
pub struct SheetInfo {
    pub sheet_name: SheetName,
    pub holder_name: Option<MemberId>,
}
