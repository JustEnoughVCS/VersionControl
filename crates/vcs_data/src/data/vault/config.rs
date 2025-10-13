use std::net::{IpAddr, Ipv4Addr};

use cfg_file::ConfigFile;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::constants::{PORT, SERVER_FILE_VAULT};
use crate::data::member::{Member, MemberId};

pub type VaultName = String;
pub type VaultUuid = Uuid;

#[derive(Serialize, Deserialize, ConfigFile)]
#[cfg_file(path = SERVER_FILE_VAULT)]
pub struct VaultConfig {
    /// Vault uuid, unique identifier for the vault
    vault_uuid: VaultUuid,

    /// Vault name, which can be used as the project name and generally serves as a hint
    vault_name: VaultName,

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
    lan_discovery: bool, // TODO

    /// Authentication strength level
    /// 0: Weakest - Anyone can claim any identity, fastest speed
    /// 1: Basic - Any device can claim any registered identity, slightly faster
    /// 2: Advanced - Uses asymmetric encryption, multiple devices can use key authentication to log in simultaneously, slightly slower
    /// 3: Secure - Uses asymmetric encryption, only one device can use key for authentication at a time, much slower
    /// Default is "Advanced", if using a lower security policy, ensure your server is only accessible by trusted devices
    auth_strength: u8, // TODO
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            vault_uuid: Uuid::new_v4(),
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
    /// Change name of the vault.
    pub fn change_name(&mut self, name: impl Into<String>) {
        self.vault_name = name.into()
    }

    /// Add admin
    pub fn add_admin(&mut self, member: &Member) {
        let uuid = member.id();
        if !self.vault_admin_list.contains(&uuid) {
            self.vault_admin_list.push(uuid);
        }
    }

    /// Remove admin
    pub fn remove_admin(&mut self, member: &Member) {
        let id = member.id();
        self.vault_admin_list.retain(|x| x != &id);
    }

    /// Get vault UUID
    pub fn vault_uuid(&self) -> &VaultUuid {
        &self.vault_uuid
    }

    /// Set vault UUID
    pub fn set_vault_uuid(&mut self, vault_uuid: VaultUuid) {
        self.vault_uuid = vault_uuid;
    }

    /// Get vault name
    pub fn vault_name(&self) -> &VaultName {
        &self.vault_name
    }

    /// Set vault name
    pub fn set_vault_name(&mut self, vault_name: VaultName) {
        self.vault_name = vault_name;
    }

    /// Get vault admin list
    pub fn vault_admin_list(&self) -> &Vec<MemberId> {
        &self.vault_admin_list
    }

    /// Set vault admin list
    pub fn set_vault_admin_list(&mut self, vault_admin_list: Vec<MemberId>) {
        self.vault_admin_list = vault_admin_list;
    }

    /// Get server config
    pub fn server_config(&self) -> &VaultServerConfig {
        &self.server_config
    }

    /// Set server config
    pub fn set_server_config(&mut self, server_config: VaultServerConfig) {
        self.server_config = server_config;
    }
}

impl VaultServerConfig {
    /// Get local bind IP address
    pub fn local_bind(&self) -> &IpAddr {
        &self.local_bind
    }

    /// Set local bind IP address
    pub fn set_local_bind(&mut self, local_bind: IpAddr) {
        self.local_bind = local_bind;
    }

    /// Get port
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Set port
    pub fn set_port(&mut self, port: u16) {
        self.port = port;
    }
}
