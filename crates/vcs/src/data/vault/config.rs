use std::net::{IpAddr, Ipv4Addr};

use cfg_file::ConfigFile;
use serde::{Deserialize, Serialize};

use crate::constants::{PORT, SERVER_FILE_VAULT};
use crate::data::member::{Member, MemberId};

#[derive(Serialize, Deserialize, ConfigFile)]
#[cfg_file(path = SERVER_FILE_VAULT)]
pub struct VaultConfig {
    /// Vault name, which can be used as the project name and generally serves as a hint
    vault_name: String,

    /// Vault admin id, a list of member id representing administrator identities
    vault_admin_list: Vec<MemberId>,

    /// Vault server configuration, which will be loaded when connecting to the server
    server_config: VaultServerConfig,
}

#[derive(Serialize, Deserialize)]
pub struct VaultServerConfig {
    /// Local IP address to bind to when the server starts
    local_bind: IpAddr,

    /// TCP port to bind to when the server starts
    port: u16,

    /// Whether to enable LAN discovery, allowing members on the same LAN to more easily find the upstream server
    lan_discovery: bool,

    /// Authentication strength level
    /// 0: Weakest - Anyone can claim any identity, fastest speed
    /// 1: Basic - Any device can claim any registered identity, slightly faster
    /// 2: Advanced - Uses asymmetric encryption, multiple devices can use key authentication to log in simultaneously, slightly slower
    /// 3: Secure - Uses asymmetric encryption, only one device can use key for authentication at a time, much slower
    /// Default is "Advanced", if using a lower security policy, ensure your server is only accessible by trusted devices
    auth_strength: u8,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            vault_name: "JustEnoughVault".to_string(),
            vault_admin_list: Vec::new(),
            server_config: VaultServerConfig {
                local_bind: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                port: PORT,
                lan_discovery: false,
                auth_strength: 2,
            },
        }
    }
}

/// Vault Management
impl VaultConfig {
    // Change name of the vault.
    pub fn change_name(&mut self, name: impl Into<String>) {
        self.vault_name = name.into()
    }

    // Add admin
    pub fn add_admin(&mut self, member: &Member) {
        let uuid = member.id();
        if !self.vault_admin_list.contains(&uuid) {
            self.vault_admin_list.push(uuid);
        }
    }

    // Remove admin
    pub fn remove_admin(&mut self, member: &Member) {
        let id = member.id();
        self.vault_admin_list.retain(|x| x != &id);
    }
}
