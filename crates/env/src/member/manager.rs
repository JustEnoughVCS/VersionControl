use std::{
    fs,
    io::{Error, ErrorKind},
    path::PathBuf,
};

use cfg_file::config::ConfigFile;

use crate::{
    constants::{SERVER_FILE_MEMBER_INFO, SERVER_FILE_MEMBER_PUB},
    current::current_vault_path,
    member::Member,
    vault::config::MemberUuid,
};

pub struct VaultMemberManager;

const UUID_PARAM: &str = "{member_uuid}";

impl VaultMemberManager {
    /// Read member from configuration file
    pub async fn member(uuid: MemberUuid) -> Result<Member, std::io::Error> {
        if let Some(cfg_file) = Self::member_cfg(uuid)? {
            let member = Member::read_from(cfg_file).await?;
            return Ok(member);
        }

        Err(Error::new(ErrorKind::NotFound, "Member not found!"))
    }

    /// Register a member to vault
    pub async fn register_member_to_vault(member: Member) -> Result<(), std::io::Error> {
        // Ensure member not exist
        if let Some(_) = Self::member_cfg(member.uuid())? {
            return Err(Error::new(
                ErrorKind::DirectoryNotEmpty,
                format!("Member `{}` already registered!", member.id()),
            ));
        }

        // Wrtie config file to member dir
        let member_cfg_path = Self::member_cfg_path(member.uuid())?;
        Member::write_to(&member, member_cfg_path).await?;

        Ok(())
    }

    /// Remove member from vault
    pub fn remove_member_from_vault(uuid: MemberUuid) -> Result<(), std::io::Error> {
        // Ensure member exist
        if let Some(member_cfg_path) = Self::member_cfg(uuid)? {
            fs::remove_file(member_cfg_path)?;
        }

        Ok(())
    }

    /// Try to get the member's configuration file to determine if the member exists
    pub fn member_cfg(uuid: MemberUuid) -> Result<Option<PathBuf>, std::io::Error> {
        let cfg_file = Self::member_cfg_path(uuid)?;
        if cfg_file.exists() {
            Ok(Some(cfg_file))
        } else {
            Ok(None)
        }
    }

    /// Try to get the member's public key file to determine if the member has login permission
    pub fn member_key(uuid: MemberUuid) -> Result<Option<PathBuf>, std::io::Error> {
        let key_file = Self::member_key_path(uuid)?;
        if key_file.exists() {
            Ok(Some(key_file))
        } else {
            Ok(None)
        }
    }

    /// Get the member's configuration file path, but do not check if the file exists
    pub fn member_cfg_path(uuid: MemberUuid) -> Result<PathBuf, std::io::Error> {
        // Has vault
        let Some(vault) = current_vault_path() else {
            return Err(Error::new(ErrorKind::NotFound, "Vault not found!"));
        };

        let path =
            vault.join(SERVER_FILE_MEMBER_INFO.replace(UUID_PARAM, uuid.to_string().as_str()));
        Ok(path)
    }

    /// Get the member's public key file path, but do not check if the file exists
    pub fn member_key_path(uuid: MemberUuid) -> Result<PathBuf, std::io::Error> {
        // Has vault
        let Some(vault) = current_vault_path() else {
            return Err(Error::new(ErrorKind::NotFound, "Vault not found!"));
        };

        let path =
            vault.join(SERVER_FILE_MEMBER_PUB.replace(UUID_PARAM, uuid.to_string().as_str()));
        Ok(path)
    }
}

pub struct UserMemberManager;

impl UserMemberManager {}
