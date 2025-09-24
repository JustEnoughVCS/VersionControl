use std::{
    env::current_dir,
    fs::{self, create_dir_all},
    path::PathBuf,
};

use cfg_file::config::ConfigFile;

use crate::{
    constants::{
        SERVER_FILE_README, SERVER_FILE_VAULT, SERVER_PATH_MEMBER_PUB, SERVER_PATH_MEMBERS,
        SERVER_PATH_SHEETS, SERVER_PATH_VIRTUAL_FILE_ROOT,
    },
    current::{current_vault_path, find_vault_path},
    workspace::vault::config::VaultConfig,
};

pub mod config;
pub mod member;
pub mod virtual_file;

pub type MemberId = String;

pub struct Vault {
    config: VaultConfig,
    vault_path: PathBuf,
}

impl Vault {
    /// Get vault path
    pub fn vault_path(&self) -> &PathBuf {
        &self.vault_path
    }

    /// Initialize vault
    pub fn init(config: VaultConfig, vault_path: impl Into<PathBuf>) -> Option<Self> {
        let Some(vault_path) = find_vault_path(vault_path) else {
            return None;
        };
        Some(Self { config, vault_path })
    }

    /// Initialize vault
    pub fn init_current_dir(config: VaultConfig) -> Option<Self> {
        let Some(vault_path) = current_vault_path() else {
            return None;
        };
        Some(Self { config, vault_path })
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
        create_dir_all(vault_path.join(SERVER_PATH_VIRTUAL_FILE_ROOT))?;

        // Final, generate README.md
        let readme_content = format!(
            "\
# JustEnoughVCS Server Setup

This directory contains the server configuration and data for `JustEnoughVCS`.

## User Authentication
To allow users to connect to this server, place their public keys in the `{}` directory.
Each public key file should be named `{{member_id}}.pem` (e.g., `juliet.pem`), and contain the user's public key in PEM format.

**ECDSA:**
```bash
openssl genpkey -algorithm ed25519 -out your_name_private.pem
openssl pkey -in your_name_private.pem -pubout -out your_name.pem
```

**RSA:**
```bash
openssl genpkey -algorithm RSA -out your_name_private.pem -pkeyopt rsa_keygen_bits:2048
openssl pkey -in your_name_private.pem -pubout -out your_name.pem
```

**DSA:**
```bash
openssl genpkey -algorithm DSA -out your_name_private.pem -pkeyopt dsa_paramgen_bits:2048
openssl pkey -in your_name_private.pem -pubout -out your_name.pem
```

Place only the `your_name.pem` file in the server's `./key/` directory, renamed to match the user's member ID.

## File Storage
All version-controlled files (Virtual File) are stored in the `{}` directory.

## License
This software is distributed under the MIT License. For complete license details, please see the main repository homepage.

## Support
Repository: `https://github.com/JustEnoughVCS/VersionControl`
Please report any issues or questions on the GitHub issue tracker.

## Thanks :)
Thank you for using `JustEnoughVCS!`
        ",
            SERVER_PATH_MEMBER_PUB, SERVER_PATH_VIRTUAL_FILE_ROOT
        )
        .trim()
        .to_string();
        fs::write(vault_path.join(SERVER_FILE_README), readme_content)?;

        Ok(())
    }

    /// Setup vault in current directory
    pub async fn setup_vault_current_dir() -> Result<(), std::io::Error> {
        Self::setup_vault(current_dir()?).await?;
        Ok(())
    }
}
