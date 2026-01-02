use std::{collections::HashMap, io::Error, path::PathBuf};

use cfg_file::ConfigFile;
use serde::{Deserialize, Serialize};

use crate::{
    constants::{CLIENT_FILE_LATEST_DATA, CLIENT_FILE_MEMBER_HELD_NOSET},
    current::current_local_path,
    data::{
        member::MemberId,
        vault::virtual_file::{VirtualFileId, VirtualFileVersion},
    },
};

const ACCOUNT: &str = "{account}";

/// # Latest file data
/// Records the file holder and the latest version for permission and update checks
#[derive(Debug, Default, Clone, Serialize, Deserialize, ConfigFile)]
#[cfg_file(path = CLIENT_FILE_MEMBER_HELD_NOSET)]
pub struct LatestFileData {
    /// File holding status
    #[serde(rename = "held")]
    held_status: HashMap<VirtualFileId, HeldStatus>,

    /// File version
    #[serde(rename = "ver")]
    versions: HashMap<VirtualFileId, VirtualFileVersion>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub enum HeldStatus {
    #[serde(rename = "Hold")]
    HeldWith(MemberId), // Held, status changes are sync to the client

    #[serde(rename = "None")]
    NotHeld, // Not held, status changes are sync to the client

    #[default]
    #[serde(rename = "Unknown")]
    WantedToKnow, // Holding status is unknown, notify server must inform client
}

impl LatestFileData {
    /// Get the path to the file holding the held status information for the given member.
    pub fn data_path(account: &MemberId) -> Result<PathBuf, std::io::Error> {
        let Some(local_path) = current_local_path() else {
            return Err(Error::new(
                std::io::ErrorKind::NotFound,
                "Workspace not found.",
            ));
        };
        Ok(local_path.join(CLIENT_FILE_LATEST_DATA.replace(ACCOUNT, account)))
    }

    /// Get the member who holds the file with the given ID.
    pub fn file_holder(&self, vfid: &VirtualFileId) -> Option<&MemberId> {
        self.held_status.get(vfid).and_then(|status| match status {
            HeldStatus::HeldWith(id) => Some(id),
            _ => None,
        })
    }

    /// Get the version of the file with the given ID.
    pub fn file_version(&self, vfid: &VirtualFileId) -> Option<&VirtualFileVersion> {
        self.versions.get(vfid)
    }

    /// Update the held status of the files.
    pub fn update_info(
        &mut self,
        map: HashMap<VirtualFileId, (Option<MemberId>, VirtualFileVersion)>,
    ) {
        for (vfid, (member_id, version)) in map {
            self.held_status.insert(
                vfid.clone(),
                match member_id {
                    Some(member_id) => HeldStatus::HeldWith(member_id),
                    None => HeldStatus::NotHeld,
                },
            );
            self.versions.insert(vfid, version);
        }
    }
}
