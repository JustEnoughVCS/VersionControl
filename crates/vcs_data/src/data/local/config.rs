use cfg_file::ConfigFile;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use crate::constants::CLIENT_FILE_WORKSPACE;
use crate::constants::PORT;
use crate::data::member::MemberId;
use crate::data::vault::config::VaultUuid;

#[derive(Serialize, Deserialize, ConfigFile)]
#[cfg_file(path = CLIENT_FILE_WORKSPACE)]
pub struct LocalConfig {
    /// The upstream address, representing the upstream address of the local workspace,
    /// to facilitate timely retrieval of new updates from the upstream source.
    upstream_addr: SocketAddr,

    /// The member ID used by the current local workspace.
    /// This ID will be used to verify access permissions when connecting to the upstream server.
    using_account: MemberId,

    /// Whether the local workspace is stained.
    ///
    /// If stained, it can only set an upstream server with the same identifier.
    ///
    /// If the value is None, it means not stained;
    /// otherwise, it contains the stain identifier (i.e., the upstream vault's unique ID)
    stained_uuid: Option<VaultUuid>,
}

impl Default for LocalConfig {
    fn default() -> Self {
        Self {
            upstream_addr: SocketAddr::V4(std::net::SocketAddrV4::new(
                std::net::Ipv4Addr::new(127, 0, 0, 1),
                PORT,
            )),
            using_account: "unknown".to_string(),
            stained_uuid: None,
        }
    }
}

impl LocalConfig {
    /// Set the vault address.
    pub fn set_vault_addr(&mut self, addr: SocketAddr) {
        self.upstream_addr = addr;
    }

    /// Get the vault address.
    pub fn vault_addr(&self) -> SocketAddr {
        self.upstream_addr
    }

    /// Set the currently used account
    pub fn set_current_account(&mut self, account: MemberId) {
        self.using_account = account;
    }

    /// Get the currently used account
    pub fn current_account(&self) -> MemberId {
        self.using_account.clone()
    }

    /// Check if the local workspace is stained.
    pub fn stained(&self) -> bool {
        self.stained_uuid.is_some()
    }

    /// Stain the local workspace with the given UUID.
    pub fn stain(&mut self, uuid: VaultUuid) {
        self.stained_uuid = Some(uuid);
    }

    /// Unstain the local workspace.
    pub fn unstain(&mut self) {
        self.stained_uuid = None;
    }
}
