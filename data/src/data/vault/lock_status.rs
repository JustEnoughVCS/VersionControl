use std::path::PathBuf;

use crate::{constants::SERVER_FILE_LOCKFILE, data::vault::Vault};

impl Vault {
    /// Get the path of the lock file for the current Vault
    pub fn lock_file_path(&self) -> PathBuf {
        self.vault_path().join(SERVER_FILE_LOCKFILE)
    }

    /// Check if the current Vault is locked
    pub fn is_locked(&self) -> bool {
        self.lock_file_path().exists()
    }

    /// Lock the current Vault
    pub fn lock(&self) -> Result<(), std::io::Error> {
        if self.is_locked() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!(
                    "Vault is locked! This indicates a service is already running here.\nPlease stop other services or delete the lock file at the vault root directory: {}",
                    self.lock_file_path().display()
                ),
            ));
        }
        std::fs::File::create(self.lock_file_path())?;
        Ok(())
    }

    /// Unlock the current Vault
    pub fn unlock(&self) -> Result<(), std::io::Error> {
        if let Err(e) = std::fs::remove_file(self.lock_file_path())
            && e.kind() != std::io::ErrorKind::NotFound
        {
            return Err(e);
        }
        Ok(())
    }
}
