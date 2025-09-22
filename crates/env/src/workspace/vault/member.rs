use std::{
    fs,
    io::{Error, ErrorKind},
    path::PathBuf,
};

use cfg_file::config::ConfigFile;

use crate::{
    constants::{SERVER_FILE_MEMBER_INFO, SERVER_FILE_MEMBER_PUB},
    workspace::{
        member::Member,
        vault::{MemberId, Vault},
    },
};

const ID_PARAM: &str = "{member_id}";

/// Member Manage
impl Vault {
    /// Read member from configuration file
    pub async fn member(&self, id: MemberId) -> Result<Member, std::io::Error> {
        if let Some(cfg_file) = self.member_cfg(id) {
            let member = Member::read_from(cfg_file).await?;
            return Ok(member);
        }

        Err(Error::new(ErrorKind::NotFound, "Member not found!"))
    }

    /// Update member info
    pub async fn update_member(&self, member: Member) -> Result<(), std::io::Error> {
        // Ensure member exist
        if let Some(_) = self.member_cfg(member.id()) {
            let member_cfg_path = self.member_cfg_path(member.id());
            Member::write_to(&member, member_cfg_path).await?;
            return Ok(());
        }

        Err(Error::new(ErrorKind::NotFound, "Member not found!"))
    }

    /// Register a member to vault
    pub async fn register_member_to_vault(&self, member: Member) -> Result<(), std::io::Error> {
        // Ensure member not exist
        if let Some(_) = self.member_cfg(member.id()) {
            return Err(Error::new(
                ErrorKind::DirectoryNotEmpty,
                format!("Member `{}` already registered!", member.id()),
            ));
        }

        // Wrtie config file to member dir
        let member_cfg_path = self.member_cfg_path(member.id());
        Member::write_to(&member, member_cfg_path).await?;

        Ok(())
    }

    /// Remove member from vault
    pub fn remove_member_from_vault(&self, id: MemberId) -> Result<(), std::io::Error> {
        // Ensure member exist
        if let Some(member_cfg_path) = self.member_cfg(id) {
            fs::remove_file(member_cfg_path)?;
        }

        Ok(())
    }

    /// Try to get the member's configuration file to determine if the member exists
    pub fn member_cfg(&self, id: MemberId) -> Option<PathBuf> {
        let cfg_file = self.member_cfg_path(id);
        if cfg_file.exists() {
            Some(cfg_file)
        } else {
            None
        }
    }

    /// Try to get the member's public key file to determine if the member has login permission
    pub fn member_key(&self, id: MemberId) -> Option<PathBuf> {
        let key_file = self.member_key_path(id);
        if key_file.exists() {
            Some(key_file)
        } else {
            None
        }
    }

    /// Get the member's configuration file path, but do not check if the file exists
    pub fn member_cfg_path(&self, id: MemberId) -> PathBuf {
        let path = self
            .vault_path
            .join(SERVER_FILE_MEMBER_INFO.replace(ID_PARAM, id.to_string().as_str()));
        path
    }

    /// Get the member's public key file path, but do not check if the file exists
    pub fn member_key_path(&self, id: MemberId) -> PathBuf {
        let path = self
            .vault_path
            .join(SERVER_FILE_MEMBER_PUB.replace(ID_PARAM, id.to_string().as_str()));
        path
    }
}
