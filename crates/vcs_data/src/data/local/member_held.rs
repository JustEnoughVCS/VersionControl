use std::collections::HashMap;

use cfg_file::ConfigFile;
use serde::{Deserialize, Serialize};

use crate::{
    constants::CLIENT_FILE_MEMBER_HELD_NOSET,
    data::{member::MemberId, vault::virtual_file::VirtualFileId},
};

#[derive(Debug, Default, Clone, Serialize, Deserialize, ConfigFile)]
#[cfg_file(path = CLIENT_FILE_MEMBER_HELD_NOSET)]
pub struct MemberHeld {
    /// File holding status
    held_status: HashMap<VirtualFileId, HeldStatus>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub enum HeldStatus {
    HeldWith(MemberId), // Held, status changes are sync to the client
    NotHeld,            // Not held, status changes are sync to the client

    #[default]
    WantedToKnow, // Holding status is unknown, notify server must inform client
}
