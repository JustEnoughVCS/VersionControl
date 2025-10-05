use std::io::Error;

use cfg_file::config::ConfigFile;
use vcs_data::{
    constants::{SERVER_FILE_SHEET, SERVER_FILE_VAULT},
    data::{
        member::{Member, MemberId},
        sheet::{InputRelativePathBuf, SheetName},
        vault::{Vault, config::VaultConfig, virtual_file::VirtualFileId},
    },
};

use crate::get_test_dir;

#[tokio::test]
async fn test_sheet_creation_management_and_persistence() -> Result<(), std::io::Error> {
    let dir = get_test_dir("sheet_management").await?;

    // Setup vault
    Vault::setup_vault(dir.clone()).await?;

    // Get vault
    let config = VaultConfig::read_from(dir.join(SERVER_FILE_VAULT)).await?;
    let Some(vault) = Vault::init(config, &dir) else {
        return Err(Error::new(std::io::ErrorKind::NotFound, "Vault not found!"));
    };

    // Add a member to use as sheet holder
    let member_id: MemberId = "test_member".to_string();
    vault
        .register_member_to_vault(Member::new(&member_id))
        .await?;

    // Test 1: Create a new sheet
    let sheet_name: SheetName = "test_sheet".to_string();
    let sheet = vault.create_sheet(&sheet_name, &member_id).await?;

    // Verify sheet properties
    assert_eq!(sheet.holder(), &member_id);
    assert_eq!(sheet.holder(), &member_id);
    assert!(sheet.inputs().is_empty());
    assert!(sheet.mapping().is_empty());

    // Verify sheet file was created
    const SHEET_NAME_PARAM: &str = "{sheet-name}";
    let sheet_path = dir.join(SERVER_FILE_SHEET.replace(SHEET_NAME_PARAM, &sheet_name));
    assert!(sheet_path.exists());

    // Test 2: Add input packages to the sheet
    let input_name = "source_files".to_string();

    // First add mapping entries that will be used to generate the input package
    let mut sheet = vault.sheet(&sheet_name).await?;

    // Add mapping entries for the files
    let main_rs_path = vcs_data::data::sheet::SheetPathBuf::from("src/main.rs");
    let lib_rs_path = vcs_data::data::sheet::SheetPathBuf::from("src/lib.rs");
    let main_rs_id = VirtualFileId::new();
    let lib_rs_id = VirtualFileId::new();

    sheet
        .add_mapping(main_rs_path.clone(), main_rs_id.clone())
        .await?;
    sheet
        .add_mapping(lib_rs_path.clone(), lib_rs_id.clone())
        .await?;

    // Use output_mappings to generate the InputPackage
    let paths = vec![main_rs_path, lib_rs_path];
    let input_package = sheet.output_mappings(input_name.clone(), &paths)?;
    sheet.add_input(input_package)?;

    // Verify input was added
    assert_eq!(sheet.inputs().len(), 1);
    let added_input = &sheet.inputs()[0];
    assert_eq!(added_input.name, input_name);
    assert_eq!(added_input.files.len(), 2);
    assert_eq!(
        added_input.files[0].0,
        InputRelativePathBuf::from("source_files/main.rs")
    );
    assert_eq!(
        added_input.files[1].0,
        InputRelativePathBuf::from("source_files/lib.rs")
    );

    // Test 3: Add mapping entries
    let mapping_path = vcs_data::data::sheet::SheetPathBuf::from("output/build.exe");
    let virtual_file_id = VirtualFileId::new();

    sheet
        .add_mapping(mapping_path.clone(), virtual_file_id.clone())
        .await?;

    // Verify mapping was added
    assert_eq!(sheet.mapping().len(), 3);
    assert_eq!(sheet.mapping().get(&mapping_path), Some(&virtual_file_id));

    // Test 4: Persist sheet to disk
    sheet.persist().await?;

    // Verify persistence by reloading the sheet
    let reloaded_sheet = vault.sheet(&sheet_name).await?;
    assert_eq!(reloaded_sheet.holder(), &member_id);
    assert_eq!(reloaded_sheet.inputs().len(), 1);
    assert_eq!(reloaded_sheet.mapping().len(), 3);

    // Test 5: Remove input package
    let mut sheet_for_removal = vault.sheet(&sheet_name).await?;
    let removed_input = sheet_for_removal.deny_input(&input_name);
    assert!(removed_input.is_some());
    let removed_input = removed_input.unwrap();
    assert_eq!(removed_input.name, input_name);
    assert_eq!(removed_input.files.len(), 2);
    assert_eq!(sheet_for_removal.inputs().len(), 0);

    // Test 6: Remove mapping entry
    let _removed_virtual_file_id = sheet_for_removal.remove_mapping(&mapping_path).await;
    // Don't check the return value since it depends on virtual file existence
    assert_eq!(sheet_for_removal.mapping().len(), 2);

    // Test 7: List all sheets in vault
    let sheet_names = vault.sheet_names()?;
    assert_eq!(sheet_names.len(), 2);
    assert!(sheet_names.contains(&sheet_name));
    assert!(sheet_names.contains(&"ref".to_string()));

    let all_sheets = vault.sheets().await?;
    assert_eq!(all_sheets.len(), 2);
    // One sheet should be the test sheet, the other should be the ref sheet with host as holder
    let test_sheet_holder = all_sheets
        .iter()
        .find(|s| s.holder() == &member_id)
        .map(|s| s.holder())
        .unwrap();
    let ref_sheet_holder = all_sheets
        .iter()
        .find(|s| s.holder() == &"host".to_string())
        .map(|s| s.holder())
        .unwrap();
    assert_eq!(test_sheet_holder, &member_id);
    assert_eq!(ref_sheet_holder, &"host".to_string());

    // Test 8: Safe deletion (move to trash)
    vault.delete_sheet_safely(&sheet_name).await?;

    // Verify sheet is not in normal listing but can be restored
    let sheet_names_after_deletion = vault.sheet_names()?;
    assert_eq!(sheet_names_after_deletion.len(), 1);
    assert_eq!(sheet_names_after_deletion[0], "ref");

    // Test 9: Restore sheet from trash
    let restored_sheet = vault.sheet(&sheet_name).await?;
    assert_eq!(restored_sheet.holder(), &member_id);
    assert_eq!(restored_sheet.holder(), &member_id);

    // Verify sheet is back in normal listing
    let sheet_names_after_restore = vault.sheet_names()?;
    assert_eq!(sheet_names_after_restore.len(), 2);
    assert!(sheet_names_after_restore.contains(&sheet_name));
    assert!(sheet_names_after_restore.contains(&"ref".to_string()));

    // Test 10: Permanent deletion
    vault.delete_sheet(&sheet_name).await?;

    // Verify sheet is permanently gone
    let sheet_names_final = vault.sheet_names()?;
    assert_eq!(sheet_names_final.len(), 1);
    assert_eq!(sheet_names_final[0], "ref");

    // Attempt to access deleted sheet should fail
    let result = vault.sheet(&sheet_name).await;
    assert!(result.is_err());

    // Clean up: Remove member
    vault.remove_member_from_vault(&member_id)?;

    Ok(())
}

#[tokio::test]
async fn test_sheet_error_conditions() -> Result<(), std::io::Error> {
    let dir = get_test_dir("sheet_error_conditions").await?;

    // Setup vault
    Vault::setup_vault(dir.clone()).await?;

    // Get vault
    let config = VaultConfig::read_from(dir.join(SERVER_FILE_VAULT)).await?;
    let Some(vault) = Vault::init(config, &dir) else {
        return Err(Error::new(std::io::ErrorKind::NotFound, "Vault not found!"));
    };

    // Test 1: Create sheet with non-existent member should fail
    let non_existent_member: MemberId = "non_existent_member".to_string();
    let sheet_name: SheetName = "test_sheet".to_string();

    let result = vault.create_sheet(&sheet_name, &non_existent_member).await;
    assert!(result.is_err());

    // Add a member first
    let member_id: MemberId = "test_member".to_string();
    vault
        .register_member_to_vault(Member::new(&member_id))
        .await?;

    // Test 2: Create duplicate sheet should fail
    vault.create_sheet(&sheet_name, &member_id).await?;
    let result = vault.create_sheet(&sheet_name, &member_id).await;
    assert!(result.is_err());

    // Test 3: Delete non-existent sheet should fail
    let non_existent_sheet: SheetName = "non_existent_sheet".to_string();
    let result = vault.delete_sheet(&non_existent_sheet).await;
    assert!(result.is_err());

    // Test 4: Safe delete non-existent sheet should fail
    let result = vault.delete_sheet_safely(&non_existent_sheet).await;
    assert!(result.is_err());

    // Test 5: Restore non-existent sheet from trash should fail
    let result = vault.restore_sheet(&non_existent_sheet).await;
    assert!(result.is_err());

    // Clean up
    vault.remove_member_from_vault(&member_id)?;

    Ok(())
}

#[tokio::test]
async fn test_sheet_data_serialization() -> Result<(), std::io::Error> {
    let dir = get_test_dir("sheet_serialization").await?;

    // Test serialization by creating a sheet through the vault
    // Setup vault
    Vault::setup_vault(dir.clone()).await?;

    // Get vault
    let config = VaultConfig::read_from(dir.join(SERVER_FILE_VAULT)).await?;
    let Some(vault) = Vault::init(config, &dir) else {
        return Err(Error::new(std::io::ErrorKind::NotFound, "Vault not found!"));
    };

    // Add a member
    let member_id: MemberId = "test_member".to_string();
    vault
        .register_member_to_vault(Member::new(&member_id))
        .await?;

    // Create a sheet
    let sheet_name: SheetName = "test_serialization_sheet".to_string();
    let mut sheet = vault.create_sheet(&sheet_name, &member_id).await?;

    // Add some inputs
    let input_name = "source_files".to_string();
    let _files = vec![
        (
            InputRelativePathBuf::from("src/main.rs"),
            VirtualFileId::new(),
        ),
        (
            InputRelativePathBuf::from("src/lib.rs"),
            VirtualFileId::new(),
        ),
    ];
    // First add mapping entries
    let main_rs_path = vcs_data::data::sheet::SheetPathBuf::from("src/main.rs");
    let lib_rs_path = vcs_data::data::sheet::SheetPathBuf::from("src/lib.rs");
    let main_rs_id = VirtualFileId::new();
    let lib_rs_id = VirtualFileId::new();

    sheet
        .add_mapping(main_rs_path.clone(), main_rs_id.clone())
        .await?;
    sheet
        .add_mapping(lib_rs_path.clone(), lib_rs_id.clone())
        .await?;

    // Use output_mappings to generate the InputPackage
    let paths = vec![main_rs_path, lib_rs_path];
    let input_package = sheet.output_mappings(input_name.clone(), &paths)?;
    sheet.add_input(input_package)?;

    // Add some mappings
    let build_exe_id = VirtualFileId::new();

    sheet
        .add_mapping(
            vcs_data::data::sheet::SheetPathBuf::from("output/build.exe"),
            build_exe_id,
        )
        .await?;

    // Persist the sheet
    sheet.persist().await?;

    // Verify the sheet file was created
    const SHEET_NAME_PARAM: &str = "{sheet-name}";
    let sheet_path = dir.join(SERVER_FILE_SHEET.replace(SHEET_NAME_PARAM, &sheet_name));
    assert!(sheet_path.exists());

    // Clean up
    vault.remove_member_from_vault(&member_id)?;

    Ok(())
}
