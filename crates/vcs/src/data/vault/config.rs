use cfg_file::ConfigFile;
use serde::{Deserialize, Serialize};

use crate::constants::SERVER_FILE_VAULT;
use crate::data::member::{Member, MemberId};

#[derive(Serialize, Deserialize, ConfigFile)]
#[cfg_file(path = SERVER_FILE_VAULT)]
pub struct VaultConfig {
    /// Vault name, which can be used as the project name and generally serves as a hint
    vault_name: String,

    /// Vault admin id, a list of member id representing administrator identities
    vault_admin_list: Vec<MemberId>,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            vault_name: "JustEnoughVault".to_string(),
            vault_admin_list: Vec::new(),
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
