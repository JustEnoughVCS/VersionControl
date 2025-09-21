use cfg_file::ConfigFile;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use crate::constants::CLIENT_FILE_WORKSPACE;
use crate::constants::PORT;

#[derive(Serialize, Deserialize, ConfigFile)]
#[cfg_file(path = CLIENT_FILE_WORKSPACE)]
pub struct LocalConfig {
    /// The vault address, representing the upstream address of the local workspace,
    /// to facilitate timely retrieval of new updates from the upstream source.
    vault_addr: SocketAddr,
}

impl Default for LocalConfig {
    fn default() -> Self {
        Self {
            vault_addr: SocketAddr::V4(std::net::SocketAddrV4::new(
                std::net::Ipv4Addr::new(127, 0, 0, 1),
                PORT,
            )),
        }
    }
}

impl LocalConfig {
    /// Set the vault address.
    pub fn set_vault_addr(&mut self, addr: SocketAddr) {
        self.vault_addr = addr;
    }

    /// Get the vault address.
    pub fn vault_addr(&self) -> SocketAddr {
        self.vault_addr
    }
}
