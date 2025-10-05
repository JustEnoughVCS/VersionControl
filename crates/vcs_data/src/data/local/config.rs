use cfg_file::ConfigFile;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use crate::constants::CLIENT_FILE_WORKSPACE;
use crate::constants::PORT;
use crate::data::member::MemberId;

#[derive(Serialize, Deserialize, ConfigFile)]
#[cfg_file(path = CLIENT_FILE_WORKSPACE)]
pub struct LocalConfig {
    /// The upstream address, representing the upstream address of the local workspace,
    /// to facilitate timely retrieval of new updates from the upstream source.
    upstream_addr: SocketAddr,

    /// The member ID used by the current local workspace.
    /// This ID will be used to verify access permissions when connecting to the upstream server.
    using_account: MemberId,
}

impl Default for LocalConfig {
    fn default() -> Self {
        Self {
            upstream_addr: SocketAddr::V4(std::net::SocketAddrV4::new(
                std::net::Ipv4Addr::new(127, 0, 0, 1),
                PORT,
            )),
            using_account: "unknown".to_string(),
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
}
