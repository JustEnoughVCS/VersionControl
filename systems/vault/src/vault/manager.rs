use asset_system::asset::ReadOnlyAsset;
use constants::vault::files::vault_file_config;
use framework::space::Space;

use crate::vault::{Vault, config::VaultConfig, error::VaultOperationError};

pub struct VaultManager {
    space: Space<Vault>,
}

impl VaultManager {
    pub fn new() -> Self {
        VaultManager {
            space: Space::new(Vault),
        }
    }

    /// Get an immutable reference to the internal Space
    pub fn get_space(&self) -> &Space<Vault> {
        &self.space
    }

    /// Get a mutable reference to the internal Space
    pub fn get_space_mut(&mut self) -> &mut Space<Vault> {
        &mut self.space
    }

    /// Get a read-only instance of the vault configuration file
    pub fn vault_config(&self) -> Result<ReadOnlyAsset<VaultConfig>, VaultOperationError> {
        let config_path = self.space.local_path(vault_file_config())?;
        if !config_path.exists() {
            return Err(VaultOperationError::ConfigNotFound);
        }
        let asset = ReadOnlyAsset::from(config_path);
        Ok(asset)
    }
}
