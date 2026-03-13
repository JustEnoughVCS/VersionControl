use asset_system::rw::RWData;
use constants::vault::{
    dirs::{vault_dir_changes, vault_dir_ignore_rules, vault_dir_member_root, vault_dir_refsheets},
    files::vault_file_config,
};
use framework::{space::SpaceRoot, space_macro::SpaceRootTest};
use tokio::fs;

use crate::vault::config::VaultConfig;

pub mod config;
pub mod error;
pub mod manager;

#[derive(Default, SpaceRootTest)]
pub struct Vault;

impl SpaceRoot for Vault {
    fn get_pattern() -> framework::space::SpaceRootFindPattern {
        framework::space::SpaceRootFindPattern::IncludeFile(vault_file_config().into())
    }

    async fn create_space(
        path: &std::path::Path,
    ) -> Result<(), framework::space::error::SpaceError> {
        let vault_toml = path.join(vault_file_config());

        // Create configuration file
        VaultConfig::write(VaultConfig::default(), &vault_toml)
            .await
            .map_err(|e| framework::space::error::SpaceError::Other(e.to_string()))?;

        // Create directories
        fs::create_dir_all(vault_dir_refsheets()).await?;
        fs::create_dir_all(vault_dir_member_root()).await?;
        fs::create_dir_all(vault_dir_ignore_rules()).await?;
        fs::create_dir_all(vault_dir_changes()).await?;

        Ok(())
    }
}
