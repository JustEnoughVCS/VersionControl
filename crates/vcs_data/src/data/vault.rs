use std::{env::current_dir, path::PathBuf, sync::Arc};

use tokio::fs::create_dir_all;
use vcs_docs::docs::READMES_VAULT_README;

use crate::{
    constants::{
        REF_SHEET_NAME, SERVER_FILE_README, SERVER_FILE_VAULT, SERVER_PATH_MEMBER_PUB,
        SERVER_PATH_MEMBERS, SERVER_PATH_SHEETS, SERVER_PATH_VF_ROOT, VAULT_HOST_NAME,
    },
    current::{current_vault_path, find_vault_path},
    data::{member::Member, vault::config::VaultConfig},
};

pub mod config;
pub mod member;
pub mod service;
pub mod sheet_share;
pub mod sheets;
pub mod virtual_file;

pub struct Vault {
    config: Arc<VaultConfig>,
    vault_path: PathBuf,
}

impl Vault {
    /// Get vault path
    pub fn vault_path(&self) -> &PathBuf {
        &self.vault_path
    }

    /// Initialize vault
    pub fn init(config: VaultConfig, vault_path: impl Into<PathBuf>) -> Option<Self> {
        let vault_path = find_vault_path(vault_path)?;
        Some(Self {
            config: Arc::new(config),
            vault_path,
        })
    }

    /// Initialize vault
    pub fn init_current_dir(config: VaultConfig) -> Option<Self> {
        let vault_path = current_vault_path()?;
        Some(Self {
            config: Arc::new(config),
            vault_path,
        })
    }

    /// Setup vault
    pub async fn setup_vault(
        vault_path: impl Into<PathBuf>,
        vault_name: impl AsRef<str>,
    ) -> Result<(), std::io::Error> {
        let vault_path: PathBuf = vault_path.into();

        // Ensure directory is empty
        if vault_path.exists() && vault_path.read_dir()?.next().is_some() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::DirectoryNotEmpty,
                "DirectoryNotEmpty",
            ));
        }

        // 1. Setup main config
        let config = VaultConfig::default();

        // NOTE:
        // Do not use the write_to method provided by the ConfigFile trait to store the Vault configuration file
        // Instead, use the PROFILES_VAULT content provided by the Documents Repository for writing

        // VaultConfig::write_to(&config, vault_path.join(SERVER_FILE_VAULT)).await?;
        let config_content = vcs_docs::docs::PROFILES_VAULT
            .replace("{vault_name}", vault_name.as_ref())
            .replace("{user_name}", whoami::username().as_str())
            .replace(
                "{date_format}",
                chrono::Local::now()
                    .format("%Y-%m-%d %H:%M")
                    .to_string()
                    .as_str(),
            )
            .replace("{vault_uuid}", &config.vault_uuid().to_string());
        tokio::fs::write(vault_path.join(SERVER_FILE_VAULT), config_content).await?;

        // 2. Setup sheets directory
        create_dir_all(vault_path.join(SERVER_PATH_SHEETS)).await?;

        // 3. Setup key directory
        create_dir_all(vault_path.join(SERVER_PATH_MEMBER_PUB)).await?;

        // 4. Setup member directory
        create_dir_all(vault_path.join(SERVER_PATH_MEMBERS)).await?;

        // 5. Setup storage directory
        create_dir_all(vault_path.join(SERVER_PATH_VF_ROOT)).await?;

        let Some(vault) = Vault::init(config, &vault_path) else {
            return Err(std::io::Error::other("Failed to initialize vault"));
        };

        // 6. Create host member
        vault
            .register_member_to_vault(Member::new(VAULT_HOST_NAME))
            .await?;

        // 7. Setup reference sheet
        vault
            .create_sheet(&REF_SHEET_NAME.to_string(), &VAULT_HOST_NAME.to_string())
            .await?;

        // Final, generate README.md
        let readme_content = READMES_VAULT_README;
        tokio::fs::write(vault_path.join(SERVER_FILE_README), readme_content).await?;

        Ok(())
    }

    /// Setup vault in current directory
    pub async fn setup_vault_current_dir(
        vault_name: impl AsRef<str>,
    ) -> Result<(), std::io::Error> {
        Self::setup_vault(current_dir()?, vault_name).await?;
        Ok(())
    }

    /// Get vault configuration
    pub fn config(&self) -> &Arc<VaultConfig> {
        &self.config
    }
}
