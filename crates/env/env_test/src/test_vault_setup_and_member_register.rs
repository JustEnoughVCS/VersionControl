use std::io::Error;

use cfg_file::config::ConfigFile;
use env::{
    constants::{
        SERVER_FILE_MEMBER_INFO, SERVER_FILE_README, SERVER_FILE_VAULT, SERVER_PATH_MEMBER_PUB,
        SERVER_PATH_MEMBERS, SERVER_PATH_SHEETS, SERVER_PATH_VIRTUAL_FILE_ROOT,
    },
    workspace::{
        member::Member,
        vault::{Vault, config::VaultConfig},
    },
};

use crate::get_test_dir;

#[tokio::test]
async fn test_vault_setup_and_member_register() -> Result<(), std::io::Error> {
    let dir = get_test_dir("member_register").await?;

    // Setup vault
    Vault::setup_vault(dir.clone()).await?;

    // Check if the following files and directories are created in `dir`:
    // Files: SERVER_FILE_VAULT, SERVER_FILE_README
    // Directories: SERVER_PATH_SHEETS,
    //              SERVER_PATH_MEMBERS,
    //              SERVER_PATH_MEMBER_PUB,
    //              SERVER_PATH_VIRTUAL_FILE_ROOT
    assert!(dir.join(SERVER_FILE_VAULT).exists());
    assert!(dir.join(SERVER_FILE_README).exists());
    assert!(dir.join(SERVER_PATH_SHEETS).exists());
    assert!(dir.join(SERVER_PATH_MEMBERS).exists());
    assert!(dir.join(SERVER_PATH_MEMBER_PUB).exists());
    assert!(dir.join(SERVER_PATH_VIRTUAL_FILE_ROOT).exists());

    // Get vault
    let config = VaultConfig::read_from(dir.join(SERVER_FILE_VAULT)).await?;
    let Some(vault) = Vault::init(config, &dir) else {
        return Err(Error::new(std::io::ErrorKind::NotFound, "Vault not found!"));
    };

    // Add member
    let member_id = "test_member";
    vault
        .register_member_to_vault(Member::new(member_id))
        .await?;

    const ID_PARAM: &str = "{member_id}";

    // Check if the member info file exists
    assert_eq!(
        dir.join(SERVER_FILE_MEMBER_INFO.replace(ID_PARAM, member_id))
            .exists(),
        true
    );

    // Remove member
    vault.remove_member_from_vault(member_id.to_string())?;

    // Check if the member info file not exists
    assert_eq!(
        dir.join(SERVER_FILE_MEMBER_INFO.replace(ID_PARAM, member_id))
            .exists(),
        false
    );

    Ok(())
}
