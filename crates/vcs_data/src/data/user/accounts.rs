use std::{
    fs,
    io::{Error, ErrorKind},
    path::PathBuf,
};

use cfg_file::config::ConfigFile;

use crate::{
    constants::{USER_FILE_ACCOUNTS, USER_FILE_KEY, USER_FILE_MEMBER},
    data::{
        member::{Member, MemberId},
        user::UserDirectory,
    },
};

const SELF_ID: &str = "{self_id}";

/// Account Management
impl UserDirectory {
    /// Read account from configuration file
    pub async fn account(&self, id: &MemberId) -> Result<Member, std::io::Error> {
        if let Some(cfg_file) = self.account_cfg(id) {
            let member = Member::read_from(cfg_file).await?;
            return Ok(member);
        }

        Err(Error::new(ErrorKind::NotFound, "Account not found!"))
    }

    /// List all account IDs in the user directory
    pub fn account_ids(&self) -> Result<Vec<MemberId>, std::io::Error> {
        let accounts_path = self
            .local_path
            .join(USER_FILE_ACCOUNTS.replace(SELF_ID, ""));

        if !accounts_path.exists() {
            return Ok(Vec::new());
        }

        let mut account_ids = Vec::new();

        for entry in fs::read_dir(accounts_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file()
                && let Some(file_name) = path.file_stem().and_then(|s| s.to_str())
                && path.extension().and_then(|s| s.to_str()) == Some("toml")
            {
                // Remove the "_private" suffix from key files if present
                let account_id = file_name.replace("_private", "");
                account_ids.push(account_id);
            }
        }

        Ok(account_ids)
    }

    /// Get all accounts
    /// This method will read and deserialize account information, please pay attention to performance issues
    pub async fn accounts(&self) -> Result<Vec<Member>, std::io::Error> {
        let mut accounts = Vec::new();

        for account_id in self.account_ids()? {
            if let Ok(account) = self.account(&account_id).await {
                accounts.push(account);
            }
        }

        Ok(accounts)
    }

    /// Update account info
    pub async fn update_account(&self, member: Member) -> Result<(), std::io::Error> {
        // Ensure account exist
        if self.account_cfg(&member.id()).is_some() {
            let account_cfg_path = self.account_cfg_path(&member.id());
            Member::write_to(&member, account_cfg_path).await?;
            return Ok(());
        }

        Err(Error::new(ErrorKind::NotFound, "Account not found!"))
    }

    /// Register an account to user directory
    pub async fn register_account(&self, member: Member) -> Result<(), std::io::Error> {
        // Ensure account not exist
        if self.account_cfg(&member.id()).is_some() {
            return Err(Error::new(
                ErrorKind::DirectoryNotEmpty,
                format!("Account `{}` already registered!", member.id()),
            ));
        }

        // Ensure accounts directory exists
        let accounts_dir = self
            .local_path
            .join(USER_FILE_ACCOUNTS.replace(SELF_ID, ""));
        if !accounts_dir.exists() {
            fs::create_dir_all(&accounts_dir)?;
        }

        // Write config file to accounts dir
        let account_cfg_path = self.account_cfg_path(&member.id());
        Member::write_to(&member, account_cfg_path).await?;

        Ok(())
    }

    /// Remove account from user directory
    pub fn remove_account(&self, id: &MemberId) -> Result<(), std::io::Error> {
        // Remove config file if exists
        if let Some(account_cfg_path) = self.account_cfg(id) {
            fs::remove_file(account_cfg_path)?;
        }

        // Remove private key file if exists
        if let Some(private_key_path) = self.account_private_key(id)
            && private_key_path.exists()
        {
            fs::remove_file(private_key_path)?;
        }

        Ok(())
    }

    /// Try to get the account's configuration file to determine if the account exists
    pub fn account_cfg(&self, id: &MemberId) -> Option<PathBuf> {
        let cfg_file = self.account_cfg_path(id);
        if cfg_file.exists() {
            Some(cfg_file)
        } else {
            None
        }
    }

    /// Try to get the account's private key file to determine if the account has a private key
    pub fn account_private_key(&self, id: &MemberId) -> Option<PathBuf> {
        let key_file = self.account_private_key_path(id);
        if key_file.exists() {
            Some(key_file)
        } else {
            None
        }
    }

    /// Check if account has private key
    pub fn has_private_key(&self, id: &MemberId) -> bool {
        self.account_private_key(id).is_some()
    }

    /// Get the account's configuration file path, but do not check if the file exists
    pub fn account_cfg_path(&self, id: &MemberId) -> PathBuf {
        self.local_path
            .join(USER_FILE_MEMBER.replace(SELF_ID, id.to_string().as_str()))
    }

    /// Get the account's private key file path, but do not check if the file exists
    pub fn account_private_key_path(&self, id: &MemberId) -> PathBuf {
        self.local_path
            .join(USER_FILE_KEY.replace(SELF_ID, id.to_string().as_str()))
    }
}
