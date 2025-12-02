use std::net::{IpAddr, Ipv4Addr};

use cfg_file::ConfigFile;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::constants::{PORT, SERVER_FILE_VAULT};
use crate::data::member::{Member, MemberId};

pub type VaultName = String;
pub type VaultUuid = Uuid;

#[derive(Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AuthMode {
    /// Use asymmetric keys: both client and server need to register keys, after which they can connect
    Key,

    /// Use password: the password stays on the server, and the client needs to set the password locally for connection
    #[default]
    Password,

    /// No authentication: generally used in a strongly secure environment, skipping verification directly
    NoAuth,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LoggerLevel {
    Debug,
    Trace,

    #[default]
    Info,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ServiceEnabled {
    Enable,

    #[default]
    Disable,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum BehaviourEnabled {
    Yes,

    #[default]
    No,
}

impl Into<bool> for ServiceEnabled {
    fn into(self) -> bool {
        match self {
            ServiceEnabled::Enable => true,
            ServiceEnabled::Disable => false,
        }
    }
}

impl Into<bool> for BehaviourEnabled {
    fn into(self) -> bool {
        match self {
            BehaviourEnabled::Yes => true,
            BehaviourEnabled::No => false,
        }
    }
}

#[derive(Serialize, Deserialize, ConfigFile)]
#[cfg_file(path = SERVER_FILE_VAULT)]
pub struct VaultConfig {
    /// Vault uuid, unique identifier for the vault
    #[serde(rename = "uuid")]
    vault_uuid: VaultUuid,

    /// Vault name, which can be used as the project name and generally serves as a hint
    #[serde(rename = "name")]
    vault_name: VaultName,

    /// Vault admin id, a list of member id representing administrator identities
    #[serde(rename = "admin")]
    vault_admin_list: Vec<MemberId>,

    /// Vault server configuration, which will be loaded when connecting to the server
    #[serde(rename = "profile")]
    server_config: VaultServerConfig,
}

#[derive(Serialize, Deserialize)]
pub struct VaultServerConfig {
    /// Local IP address to bind to when the server starts
    #[serde(rename = "bind")]
    local_bind: IpAddr,

    /// TCP port to bind to when the server starts
    #[serde(rename = "port")]
    port: u16,

    /// Enable logging
    #[serde(rename = "logger")]
    logger: Option<BehaviourEnabled>,

    /// Logger Level
    #[serde(rename = "logger_level")]
    logger_level: Option<LoggerLevel>,

    /// Whether to enable LAN discovery, allowing members on the same LAN to more easily find the upstream server
    #[serde(rename = "lan_discovery")]
    lan_discovery: Option<ServiceEnabled>, // TODO

    /// Authentication mode for the vault server
    /// key: Use asymmetric keys for authentication
    /// password: Use a password for authentication
    /// noauth: No authentication required, requires a strongly secure environment
    #[serde(rename = "auth_mode")]
    auth_mode: Option<AuthMode>, // TODO
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
                logger: Some(BehaviourEnabled::default()),
                logger_level: Some(LoggerLevel::default()),
                lan_discovery: Some(ServiceEnabled::default()),
                auth_mode: Some(AuthMode::Key),
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

    /// Get port
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Check if LAN discovery is enabled
    pub fn is_lan_discovery_enabled(&self) -> bool {
        self.lan_discovery.clone().unwrap_or_default().into()
    }

    /// Get logger enabled status
    pub fn is_logger_enabled(&self) -> bool {
        self.logger.clone().unwrap_or_default().into()
    }

    /// Get logger level
    pub fn logger_level(&self) -> LoggerLevel {
        self.logger_level.clone().unwrap_or_default()
    }

    /// Get authentication mode
    pub fn auth_mode(&self) -> AuthMode {
        self.auth_mode.clone().unwrap_or_default()
    }
}
