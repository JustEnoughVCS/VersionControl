use std::io::Error;

use cfg_file::config::ConfigFile;
use vcs::{
    constants::{CLIENT_FILE_README, CLIENT_FILE_WORKSPACE, USER_FILE_KEY, USER_FILE_MEMBER},
    data::{
        local::{LocalWorkspace, config::LocalConfig},
        member::Member,
        user::UserDirectory,
    },
};

use crate::get_test_dir;

#[tokio::test]
async fn test_local_workspace_setup_and_account_management() -> Result<(), std::io::Error> {
    let dir = get_test_dir("local_workspace_account_management").await?;

    // Setup local workspace
    LocalWorkspace::setup_local_workspace(dir.clone()).await?;

    // Check if the following files are created in `dir`:
    // Files: CLIENT_FILE_WORKSPACE, CLIENT_FILE_README
    assert!(dir.join(CLIENT_FILE_WORKSPACE).exists());
    assert!(dir.join(CLIENT_FILE_README).exists());

    // Get local workspace
    let config = LocalConfig::read_from(dir.join(CLIENT_FILE_WORKSPACE)).await?;
    let Some(_local_workspace) = LocalWorkspace::init(config, &dir) else {
        return Err(Error::new(
            std::io::ErrorKind::NotFound,
            "Local workspace not found!",
        ));
    };

    // Create user directory from workspace path
    let Some(user_directory) = UserDirectory::from_path(&dir) else {
        return Err(Error::new(
            std::io::ErrorKind::NotFound,
            "User directory not found!",
        ));
    };

    // Test account registration
    let member_id = "test_account";
    let member = Member::new(member_id);

    // Register account
    user_directory.register_account(member.clone()).await?;

    // Check if the account config file exists
    assert!(
        dir.join(USER_FILE_MEMBER.replace("{self_id}", member_id))
            .exists()
    );

    // Test account retrieval
    let retrieved_member = user_directory.account(&member_id.to_string()).await?;
    assert_eq!(retrieved_member.id(), member.id());

    // Test account IDs listing
    let account_ids = user_directory.account_ids()?;
    assert!(account_ids.contains(&member_id.to_string()));

    // Test accounts listing
    let accounts = user_directory.accounts().await?;
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].id(), member.id());

    // Test account existence check
    assert!(user_directory.account_cfg(&member_id.to_string()).is_some());

    // Test private key check (should be false initially)
    assert!(!user_directory.has_private_key(&member_id.to_string()));

    // Test account update
    let mut updated_member = member.clone();
    updated_member.set_metadata("email", "test@example.com");
    user_directory
        .update_account(updated_member.clone())
        .await?;

    // Verify update
    let updated_retrieved = user_directory.account(&member_id.to_string()).await?;
    assert_eq!(
        updated_retrieved.metadata("email"),
        Some(&"test@example.com".to_string())
    );

    // Test account removal
    user_directory.remove_account(&member_id.to_string())?;

    // Check if the account config file no longer exists
    assert!(
        !dir.join(USER_FILE_MEMBER.replace("{self_id}", member_id))
            .exists()
    );

    // Check if account is no longer in the list
    let account_ids_after_removal = user_directory.account_ids()?;
    assert!(!account_ids_after_removal.contains(&member_id.to_string()));

    Ok(())
}

#[tokio::test]
async fn test_account_private_key_management() -> Result<(), std::io::Error> {
    let dir = get_test_dir("account_private_key_management").await?;

    // Create user directory
    let Some(user_directory) = UserDirectory::from_path(&dir) else {
        return Err(Error::new(
            std::io::ErrorKind::NotFound,
            "User directory not found!",
        ));
    };

    // Register account
    let member_id = "test_account_with_key";
    let member = Member::new(member_id);
    user_directory.register_account(member).await?;

    // Create a dummy private key file for testing
    let private_key_path = dir.join(USER_FILE_KEY.replace("{self_id}", member_id));
    std::fs::create_dir_all(private_key_path.parent().unwrap())?;
    std::fs::write(&private_key_path, "dummy_private_key_content")?;

    // Test private key existence check
    assert!(user_directory.has_private_key(&member_id.to_string()));

    // Test private key path retrieval
    assert!(
        user_directory
            .account_private_key(&member_id.to_string())
            .is_some()
    );

    // Remove account (should also remove private key)
    user_directory.remove_account(&member_id.to_string())?;

    // Check if private key file is also removed
    assert!(!private_key_path.exists());

    Ok(())
}

#[tokio::test]
async fn test_multiple_account_management() -> Result<(), std::io::Error> {
    let dir = get_test_dir("multiple_account_management").await?;

    // Create user directory
    let Some(user_directory) = UserDirectory::from_path(&dir) else {
        return Err(Error::new(
            std::io::ErrorKind::NotFound,
            "User directory not found!",
        ));
    };

    // Register multiple accounts
    let account_names = vec!["alice", "bob", "charlie"];

    for name in &account_names {
        user_directory.register_account(Member::new(*name)).await?;
    }

    // Test account IDs listing
    let account_ids = user_directory.account_ids()?;
    assert_eq!(account_ids.len(), 3);

    for name in &account_names {
        assert!(account_ids.contains(&name.to_string()));
    }

    // Test accounts listing
    let accounts = user_directory.accounts().await?;
    assert_eq!(accounts.len(), 3);

    // Remove one account
    user_directory.remove_account(&"bob".to_string())?;

    // Verify removal
    let account_ids_after_removal = user_directory.account_ids()?;
    assert_eq!(account_ids_after_removal.len(), 2);
    assert!(!account_ids_after_removal.contains(&"bob".to_string()));
    assert!(account_ids_after_removal.contains(&"alice".to_string()));
    assert!(account_ids_after_removal.contains(&"charlie".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_account_registration_duplicate_prevention() -> Result<(), std::io::Error> {
    let dir = get_test_dir("account_duplicate_prevention").await?;

    // Create user directory
    let Some(user_directory) = UserDirectory::from_path(&dir) else {
        return Err(Error::new(
            std::io::ErrorKind::NotFound,
            "User directory not found!",
        ));
    };

    // Register account
    let member_id = "duplicate_test";
    user_directory
        .register_account(Member::new(member_id))
        .await?;

    // Try to register same account again - should fail
    let result = user_directory
        .register_account(Member::new(member_id))
        .await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_nonexistent_account_operations() -> Result<(), std::io::Error> {
    let dir = get_test_dir("nonexistent_account_operations").await?;

    // Create user directory
    let Some(user_directory) = UserDirectory::from_path(&dir) else {
        return Err(Error::new(
            std::io::ErrorKind::NotFound,
            "User directory not found!",
        ));
    };

    // Try to read non-existent account - should fail
    let result = user_directory.account(&"nonexistent".to_string()).await;
    assert!(result.is_err());

    // Try to update non-existent account - should fail
    let result = user_directory
        .update_account(Member::new("nonexistent"))
        .await;
    assert!(result.is_err());

    // Try to remove non-existent account - should succeed (idempotent)
    let result = user_directory.remove_account(&"nonexistent".to_string());
    assert!(result.is_ok());

    // Check private key for non-existent account - should be false
    assert!(!user_directory.has_private_key(&"nonexistent".to_string()));

    Ok(())
}
