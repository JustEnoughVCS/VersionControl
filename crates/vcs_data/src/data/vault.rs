use std::{
    env::current_dir,
    fs::{self, create_dir_all},
    path::PathBuf,
    sync::Arc,
};

use cfg_file::config::ConfigFile;
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
    pub async fn setup_vault(vault_path: impl Into<PathBuf>) -> Result<(), std::io::Error> {
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
        VaultConfig::write_to(&config, vault_path.join(SERVER_FILE_VAULT)).await?;

        // 2. Setup sheets directory
        create_dir_all(vault_path.join(SERVER_PATH_SHEETS))?;

        // 3. Setup key directory
        create_dir_all(vault_path.join(SERVER_PATH_MEMBER_PUB))?;

        // 4. Setup member directory
        create_dir_all(vault_path.join(SERVER_PATH_MEMBERS))?;

        // 5. Setup storage directory
        create_dir_all(vault_path.join(SERVER_PATH_VF_ROOT))?;

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
        fs::write(vault_path.join(SERVER_FILE_README), readme_content)?;

        Ok(())
    }

    /// Setup vault in current directory
    pub async fn setup_vault_current_dir() -> Result<(), std::io::Error> {
        Self::setup_vault(current_dir()?).await?;
        Ok(())
    }

    /// Get vault configuration
    pub fn config(&self) -> &Arc<VaultConfig> {
        &self.config
    }
}
