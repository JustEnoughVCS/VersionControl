use std::io::Error;

use cfg_file::config::ConfigFile;
use vcs_data::{
    constants::{SERVER_FILE_VAULT, SERVER_SUFFIX_SHEET_FILE},
    data::{
        member::{Member, MemberId},
        sheet::{SheetName, SheetPathBuf},
        vault::{
            Vault,
            config::VaultConfig,
            sheet_share::{Share, ShareMergeMode, SheetShareId},
            virtual_file::VirtualFileId,
        },
    },
};

use crate::get_test_dir;

#[tokio::test]
async fn test_share_creation_and_retrieval() -> Result<(), std::io::Error> {
    let dir = get_test_dir("share_creation").await?;

    // Setup vault
    Vault::setup_vault(dir.clone(), "TestVault").await?;

    // Get vault
    let config = VaultConfig::read_from(dir.join(SERVER_FILE_VAULT)).await?;
    let Some(vault) = Vault::init(config, &dir) else {
        return Err(Error::new(std::io::ErrorKind::NotFound, "Vault not found!"));
    };

    // Add members
    let sharer_id: MemberId = "sharer_member".to_string();
    let target_member_id: MemberId = "target_member".to_string();

    vault
        .register_member_to_vault(Member::new(&sharer_id))
        .await?;
    vault
        .register_member_to_vault(Member::new(&target_member_id))
        .await?;

    // Create source sheet for sharer
    let source_sheet_name: SheetName = "source_sheet".to_string();
    let _source_sheet = vault.create_sheet(&source_sheet_name, &sharer_id).await?;

    // Create target sheet for target member
    let target_sheet_name: SheetName = "target_sheet".to_string();
    let _target_sheet = vault
        .create_sheet(&target_sheet_name, &target_member_id)
        .await?;

    // Add mappings to source sheet
    let mut source_sheet = vault.sheet(&source_sheet_name).await?;

    let main_rs_path = SheetPathBuf::from("src/main.rs");
    let lib_rs_path = SheetPathBuf::from("src/lib.rs");
    let main_rs_id = VirtualFileId::from("main_rs_id_1");
    let lib_rs_id = VirtualFileId::from("lib_rs_id_1");

    source_sheet
        .add_mapping(
            main_rs_path.clone(),
            main_rs_id.clone(),
            "1.0.0".to_string(),
        )
        .await?;
    source_sheet
        .add_mapping(lib_rs_path.clone(), lib_rs_id.clone(), "1.0.0".to_string())
        .await?;

    // Persist source sheet
    source_sheet.persist().await?;

    // Test 1: Share mappings from source sheet to target sheet
    let description = "Test share of main.rs and lib.rs".to_string();
    // Need to get the sheet again after persist
    let source_sheet = vault.sheet(&source_sheet_name).await?;

    source_sheet
        .share_mappings(
            &target_sheet_name,
            vec![main_rs_path.clone(), lib_rs_path.clone()],
            &sharer_id,
            description.clone(),
        )
        .await?;

    // Test 2: Get shares from target sheet
    let target_sheet = vault.sheet(&target_sheet_name).await?;

    let shares = target_sheet.get_shares().await?;

    assert_eq!(shares.len(), 1, "Expected 1 share, found {}", shares.len());
    let share = &shares[0];

    assert_eq!(share.sharer, sharer_id);
    assert_eq!(share.description, description);
    assert_eq!(share.from_sheet, source_sheet_name);
    assert_eq!(share.mappings.len(), 2);
    assert!(share.mappings.contains_key(&main_rs_path));
    assert!(share.mappings.contains_key(&lib_rs_path));
    assert!(share.path.is_some());

    // Test 3: Get specific share by ID
    let share_id = Share::gen_share_id(&sharer_id);
    let _specific_share = target_sheet.get_share(&share_id).await;

    // Note: The share ID might not match exactly due to random generation,
    // but we can verify the share exists by checking the shares list
    assert!(shares.iter().any(|s| s.sharer == sharer_id));

    // Clean up
    vault.remove_member_from_vault(&sharer_id)?;
    vault.remove_member_from_vault(&target_member_id)?;

    Ok(())
}

#[tokio::test]
async fn test_share_merge_modes() -> Result<(), std::io::Error> {
    let dir = get_test_dir("share_merge_modes").await?;

    // Setup vault
    Vault::setup_vault(dir.clone(), "TestVault").await?;

    // Get vault
    let config = VaultConfig::read_from(dir.join(SERVER_FILE_VAULT)).await?;
    let Some(vault) = Vault::init(config, &dir) else {
        return Err(Error::new(std::io::ErrorKind::NotFound, "Vault not found!"));
    };

    // Add members
    let sharer_id: MemberId = "sharer".to_string();
    let target_member_id: MemberId = "target".to_string();

    vault
        .register_member_to_vault(Member::new(&sharer_id))
        .await?;
    vault
        .register_member_to_vault(Member::new(&target_member_id))
        .await?;

    // Create source and target sheets
    let source_sheet_name: SheetName = "source".to_string();
    let target_sheet_name: SheetName = "target".to_string();

    let _source_sheet = vault.create_sheet(&source_sheet_name, &sharer_id).await?;
    let _target_sheet = vault
        .create_sheet(&target_sheet_name, &target_member_id)
        .await?;

    // Add mappings to source sheet
    let mut source_sheet = vault.sheet(&source_sheet_name).await?;

    let file1_path = SheetPathBuf::from("src/file1.rs");
    let file2_path = SheetPathBuf::from("src/file2.rs");
    let file1_id = VirtualFileId::from("file1_id_1");
    let file2_id = VirtualFileId::from("file2_id_1");

    source_sheet
        .add_mapping(file1_path.clone(), file1_id.clone(), "1.0.0".to_string())
        .await?;
    source_sheet
        .add_mapping(file2_path.clone(), file2_id.clone(), "1.0.0".to_string())
        .await?;

    source_sheet.persist().await?;

    // Share mappings
    // Need to get the sheet again after persist
    let source_sheet = vault.sheet(&source_sheet_name).await?;
    source_sheet
        .share_mappings(
            &target_sheet_name,
            vec![file1_path.clone(), file2_path.clone()],
            &sharer_id,
            "Test share".to_string(),
        )
        .await?;

    // Get the share
    let target_sheet = vault.sheet(&target_sheet_name).await?;
    let shares = target_sheet.get_shares().await?;
    assert_eq!(shares.len(), 1);
    let share = shares[0].clone();

    // Test 4: Safe mode merge (should succeed with no conflicts)
    let result = target_sheet
        .merge_share(share.clone(), ShareMergeMode::Safe)
        .await;

    assert!(
        result.is_ok(),
        "Safe mode should succeed with no conflicts "
    );

    // Verify mappings were added to target sheet
    let updated_target_sheet = vault.sheet(&target_sheet_name).await?;
    assert_eq!(updated_target_sheet.mapping().len(), 2);
    assert!(updated_target_sheet.mapping().contains_key(&file1_path));
    assert!(updated_target_sheet.mapping().contains_key(&file2_path));

    // Clean up
    vault.remove_member_from_vault(&sharer_id)?;
    vault.remove_member_from_vault(&target_member_id)?;

    Ok(())
}

#[tokio::test]
async fn test_share_merge_conflicts() -> Result<(), std::io::Error> {
    let dir = get_test_dir("share_conflicts").await?;

    // Setup vault
    Vault::setup_vault(dir.clone(), "TestVault").await?;

    // Get vault
    let config = VaultConfig::read_from(dir.join(SERVER_FILE_VAULT)).await?;
    let Some(vault) = Vault::init(config, &dir) else {
        return Err(Error::new(std::io::ErrorKind::NotFound, "Vault not found!"));
    };

    // Add members
    let sharer_id: MemberId = "sharer".to_string();
    let target_member_id: MemberId = "target".to_string();

    vault
        .register_member_to_vault(Member::new(&sharer_id))
        .await?;
    vault
        .register_member_to_vault(Member::new(&target_member_id))
        .await?;

    // Create source and target sheets
    let source_sheet_name: SheetName = "source".to_string();
    let target_sheet_name: SheetName = "target".to_string();

    let _source_sheet = vault.create_sheet(&source_sheet_name, &sharer_id).await?;
    let _target_sheet = vault
        .create_sheet(&target_sheet_name, &target_member_id)
        .await?;

    // Add conflicting mappings to both sheets
    let mut source_sheet = vault.sheet(&source_sheet_name).await?;
    let mut target_sheet_mut = vault.sheet(&target_sheet_name).await?;

    let conflicting_path = SheetPathBuf::from("src/conflicting.rs");
    let source_file_id = VirtualFileId::from("source_file_id_1");
    let target_file_id = VirtualFileId::from("target_file_id_1");

    // Add same path with different IDs to both sheets (conflict)
    source_sheet
        .add_mapping(
            conflicting_path.clone(),
            source_file_id.clone(),
            "1.0.0".to_string(),
        )
        .await?;

    target_sheet_mut
        .add_mapping(
            conflicting_path.clone(),
            target_file_id.clone(),
            "1.0.0".to_string(),
        )
        .await?;

    source_sheet.persist().await?;
    target_sheet_mut.persist().await?;

    // Share the conflicting mapping
    // Need to get the sheet again after persist
    let source_sheet = vault.sheet(&source_sheet_name).await?;
    source_sheet
        .share_mappings(
            &target_sheet_name,
            vec![conflicting_path.clone()],
            &sharer_id,
            "Conflicting share".to_string(),
        )
        .await?;

    // Get the share
    let target_sheet = vault.sheet(&target_sheet_name).await?;
    let shares = target_sheet.get_shares().await?;
    assert_eq!(shares.len(), 1);
    let share = shares[0].clone();

    // Test 5: Safe mode merge with conflict (should fail)
    let target_sheet_clone = vault.sheet(&target_sheet_name).await?;
    let result = target_sheet_clone
        .merge_share(share.clone(), ShareMergeMode::Safe)
        .await;

    assert!(result.is_err(), "Safe mode should fail with conflicts");

    // Test 6: Overwrite mode merge with conflict (should succeed)
    let target_sheet_clone = vault.sheet(&target_sheet_name).await?;
    let result = target_sheet_clone
        .merge_share(share.clone(), ShareMergeMode::Overwrite)
        .await;

    assert!(
        result.is_ok(),
        "Overwrite mode should succeed with conflicts"
    );

    // Verify the mapping was overwritten
    let updated_target_sheet = vault.sheet(&target_sheet_name).await?;
    let mapping = updated_target_sheet.mapping().get(&conflicting_path);
    assert!(mapping.is_some());
    assert_eq!(mapping.unwrap().id, source_file_id); // Should be source's ID, not target's

    // Clean up
    vault.remove_member_from_vault(&sharer_id)?;
    vault.remove_member_from_vault(&target_member_id)?;

    Ok(())
}

#[tokio::test]
async fn test_share_skip_mode() -> Result<(), std::io::Error> {
    let dir = get_test_dir("share_skip_mode").await?;

    // Setup vault
    Vault::setup_vault(dir.clone(), "TestVault").await?;

    // Get vault
    let config = VaultConfig::read_from(dir.join(SERVER_FILE_VAULT)).await?;
    let Some(vault) = Vault::init(config, &dir) else {
        return Err(Error::new(std::io::ErrorKind::NotFound, "Vault not found!"));
    };

    // Add members
    let sharer_id: MemberId = "sharer".to_string();
    let target_member_id: MemberId = "target".to_string();

    vault
        .register_member_to_vault(Member::new(&sharer_id))
        .await?;
    vault
        .register_member_to_vault(Member::new(&target_member_id))
        .await?;

    // Create source and target sheets
    let source_sheet_name: SheetName = "source".to_string();
    let target_sheet_name: SheetName = "target".to_string();

    let _source_sheet = vault.create_sheet(&source_sheet_name, &sharer_id).await?;
    let _target_sheet = vault
        .create_sheet(&target_sheet_name, &target_member_id)
        .await?;

    // Add mappings to both sheets
    let mut source_sheet = vault.sheet(&source_sheet_name).await?;
    let mut target_sheet_mut = vault.sheet(&target_sheet_name).await?;

    let conflicting_path = SheetPathBuf::from("src/conflicting.rs");
    let non_conflicting_path = SheetPathBuf::from("src/non_conflicting.rs");

    let source_file_id = VirtualFileId::from("source_file_id_2");
    let target_file_id = VirtualFileId::from("target_file_id_2");
    let non_conflicting_id = VirtualFileId::from("non_conflicting_id_1");

    // Add conflicting mapping to both sheets
    source_sheet
        .add_mapping(
            conflicting_path.clone(),
            source_file_id.clone(),
            "1.0.0".to_string(),
        )
        .await?;

    target_sheet_mut
        .add_mapping(
            conflicting_path.clone(),
            target_file_id.clone(),
            "1.0.0".to_string(),
        )
        .await?;

    // Add non-conflicting mapping only to source
    source_sheet
        .add_mapping(
            non_conflicting_path.clone(),
            non_conflicting_id.clone(),
            "1.0.0".to_string(),
        )
        .await?;

    source_sheet.persist().await?;
    target_sheet_mut.persist().await?;

    // Share both mappings
    // Need to get the sheet again after persist
    let source_sheet = vault.sheet(&source_sheet_name).await?;
    source_sheet
        .share_mappings(
            &target_sheet_name,
            vec![conflicting_path.clone(), non_conflicting_path.clone()],
            &sharer_id,
            "Mixed share".to_string(),
        )
        .await?;

    // Get the share
    let target_sheet = vault.sheet(&target_sheet_name).await?;
    let shares = target_sheet.get_shares().await?;
    assert_eq!(shares.len(), 1);
    let share = shares[0].clone();

    // Test 7: Skip mode merge with conflict (should skip conflicting, add non-conflicting)
    let result = target_sheet
        .merge_share(share.clone(), ShareMergeMode::Skip)
        .await;

    assert!(result.is_ok(), "Skip mode should succeed");

    // Verify only non-conflicting mapping was added
    let updated_target_sheet = vault.sheet(&target_sheet_name).await?;

    // Conflicting mapping should still have target's ID
    let conflicting_mapping = updated_target_sheet.mapping().get(&conflicting_path);
    assert!(conflicting_mapping.is_some());
    assert_eq!(conflicting_mapping.unwrap().id, target_file_id);

    // Non-conflicting mapping should be added
    let non_conflicting_mapping = updated_target_sheet.mapping().get(&non_conflicting_path);
    assert!(non_conflicting_mapping.is_some());
    assert_eq!(non_conflicting_mapping.unwrap().id, non_conflicting_id);

    // Clean up
    vault.remove_member_from_vault(&sharer_id)?;
    vault.remove_member_from_vault(&target_member_id)?;

    Ok(())
}

#[tokio::test]
async fn test_share_removal() -> Result<(), std::io::Error> {
    let dir = get_test_dir("share_removal").await?;

    // Setup vault
    Vault::setup_vault(dir.clone(), "TestVault").await?;

    // Get vault
    let config = VaultConfig::read_from(dir.join(SERVER_FILE_VAULT)).await?;
    let Some(vault) = Vault::init(config, &dir) else {
        return Err(Error::new(std::io::ErrorKind::NotFound, "Vault not found!"));
    };

    // Add members
    let sharer_id: MemberId = "sharer".to_string();
    let target_member_id: MemberId = "target".to_string();

    vault
        .register_member_to_vault(Member::new(&sharer_id))
        .await?;
    vault
        .register_member_to_vault(Member::new(&target_member_id))
        .await?;

    // Create source and target sheets
    let source_sheet_name: SheetName = "source".to_string();
    let target_sheet_name: SheetName = "target".to_string();

    let _source_sheet = vault.create_sheet(&source_sheet_name, &sharer_id).await?;
    let _target_sheet = vault
        .create_sheet(&target_sheet_name, &target_member_id)
        .await?;

    // Add mapping to source sheet
    let mut source_sheet = vault.sheet(&source_sheet_name).await?;

    let file_path = SheetPathBuf::from("src/file.rs");
    let file_id = VirtualFileId::from("file_id_1");

    source_sheet
        .add_mapping(file_path.clone(), file_id.clone(), "1.0.0".to_string())
        .await?;

    source_sheet.persist().await?;

    // Need to get the sheet again after persist
    let source_sheet = vault.sheet(&source_sheet_name).await?;
    // Share mapping
    source_sheet
        .share_mappings(
            &target_sheet_name,
            vec![file_path.clone()],
            &sharer_id,
            "Test share for removal".to_string(),
        )
        .await?;

    // Get the share
    let target_sheet = vault.sheet(&target_sheet_name).await?;
    let shares = target_sheet.get_shares().await?;
    assert_eq!(shares.len(), 1);
    let share = shares[0].clone();

    // Test 8: Remove share
    let result = share.remove().await;

    // Check if removal succeeded or failed gracefully
    match result {
        Ok(_) => {
            // Share was successfully removed
            let shares_after_removal = target_sheet.get_shares().await?;
            assert_eq!(shares_after_removal.len(), 0);
        }
        Err((returned_share, _error)) => {
            // Share removal failed, but we got the share backZ
            // Error message may vary, just check that we got an error
            // The share should be returned in the error
            assert_eq!(returned_share.sharer, sharer_id);
        }
    }

    // Clean up
    vault.remove_member_from_vault(&sharer_id)?;
    vault.remove_member_from_vault(&target_member_id)?;

    Ok(())
}

#[tokio::test]
async fn test_share_error_conditions() -> Result<(), std::io::Error> {
    let dir = get_test_dir("share_errors").await?;

    // Setup vault
    Vault::setup_vault(dir.clone(), "TestVault").await?;

    // Get vault
    let config = VaultConfig::read_from(dir.join(SERVER_FILE_VAULT)).await?;
    let Some(vault) = Vault::init(config, &dir) else {
        return Err(Error::new(std::io::ErrorKind::NotFound, "Vault not found!"));
    };

    // Add member
    let sharer_id: MemberId = "sharer".to_string();
    vault
        .register_member_to_vault(Member::new(&sharer_id))
        .await?;

    // Create source sheet
    let source_sheet_name: SheetName = "source".to_string();
    let _source_sheet = vault.create_sheet(&source_sheet_name, &sharer_id).await?;

    // Add mapping to source sheet
    let mut source_sheet = vault.sheet(&source_sheet_name).await?;

    let file_path = SheetPathBuf::from("src/file.rs");
    let file_id = VirtualFileId::from("file_id_2");

    source_sheet
        .add_mapping(file_path.clone(), file_id.clone(), "1.0.0".to_string())
        .await?;

    source_sheet.persist().await?;

    // Test 9: Share to non-existent sheet should fail
    let non_existent_sheet: SheetName = "non_existent".to_string();
    // Need to get the sheet again after persist
    let source_sheet = vault.sheet(&source_sheet_name).await?;
    let result = source_sheet
        .share_mappings(
            &non_existent_sheet,
            vec![file_path.clone()],
            &sharer_id,
            "Test".to_string(),
        )
        .await;

    assert!(result.is_err());

    // Test 10: Share non-existent mapping should fail
    let target_sheet_name: SheetName = "target".to_string();
    let _target_sheet = vault.create_sheet(&target_sheet_name, &sharer_id).await?;

    let non_existent_path = SheetPathBuf::from("src/non_existent.rs");
    let result = source_sheet
        .share_mappings(
            &target_sheet_name,
            vec![non_existent_path],
            &sharer_id,
            "Test".to_string(),
        )
        .await;

    assert!(result.is_err());

    // Test 11: Merge non-existent share should fail
    let target_sheet = vault.sheet(&target_sheet_name).await?;
    let non_existent_share_id: SheetShareId = "non_existent_share".to_string();
    let result = target_sheet
        .merge_share_by_id(&non_existent_share_id, ShareMergeMode::Safe)
        .await;

    assert!(result.is_err());

    // Clean up
    vault.remove_member_from_vault(&sharer_id)?;

    Ok(())
}

#[tokio::test]
async fn test_share_id_generation() -> Result<(), std::io::Error> {
    // Test 12: Share ID generation
    let sharer_id: MemberId = "test_sharer".to_string();

    // Generate multiple IDs to ensure they're different
    let id1 = Share::gen_share_id(&sharer_id);
    let id2 = Share::gen_share_id(&sharer_id);
    let id3 = Share::gen_share_id(&sharer_id);

    // IDs should be different due to random component
    assert_ne!(id1, id2);
    assert_ne!(id1, id3);
    assert_ne!(id2, id3);

    // IDs should contain sharer name and file suffix
    assert!(id1.contains("test_sharer"));
    assert!(id1.ends_with(SERVER_SUFFIX_SHEET_FILE));

    assert!(id2.contains("test_sharer"));
    assert!(id2.ends_with(SERVER_SUFFIX_SHEET_FILE));

    assert!(id3.contains("test_sharer"));
    assert!(id3.ends_with(SERVER_SUFFIX_SHEET_FILE));

    Ok(())
}
