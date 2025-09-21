use cfg_file::ConfigFile;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::constants::SERVER_FILE_VAULT;
use crate::member::Member;

pub type MemberUuid = Uuid;

#[derive(Default, Serialize, Deserialize, ConfigFile)]
#[cfg_file(path = SERVER_FILE_VAULT)]
pub struct VaultConfig {
    /// Vault name, which can be used as the project name and generally serves as a hint
    vault_name: String,

    /// Vault admin Uuids, a list of member Uuids representing administrator identities
    vault_admin_list: Vec<MemberUuid>,
}

impl VaultConfig {
    // Change name of the vault.
    pub fn change_name(&mut self, name: impl Into<String>) {
        self.vault_name = name.into()
    }

    // Add admin
    pub fn add_admin(&mut self, member: &Member) {
        let uuid = member.uuid();
        if !self.vault_admin_list.contains(&uuid) {
            self.vault_admin_list.push(uuid);
        }
    }

    // Remove admin
    pub fn remove_admin(&mut self, member: &Member) {
        let uuid = member.uuid();
        self.vault_admin_list.retain(|&x| x != uuid);
    }
}
