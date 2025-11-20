use std::{collections::HashMap, io::Error};

use cfg_file::config::ConfigFile;
use string_proc::snake_case;
use tokio::fs;

use crate::{
    constants::{SERVER_PATH_SHEETS, SERVER_SUFFIX_SHEET_FILE_NO_DOT},
    data::{
        member::MemberId,
        sheet::{Sheet, SheetData, SheetName},
        vault::Vault,
    },
};

/// Vault Sheets Management
impl Vault {
    /// Load all sheets in the vault
    ///
    /// It is generally not recommended to call this function frequently.
    /// Although a vault typically won't contain too many sheets,
    /// if individual sheet contents are large, this operation may cause
    /// significant performance bottlenecks.
    pub async fn sheets<'a>(&'a self) -> Result<Vec<Sheet<'a>>, std::io::Error> {
        let sheet_names = self.sheet_names()?;
        let mut sheets = Vec::new();

        for sheet_name in sheet_names {
            let sheet = self.sheet(&sheet_name).await?;
            sheets.push(sheet);
        }

        Ok(sheets)
    }

    /// Search for all sheet names in the vault
    ///
    /// The complexity of this operation is proportional to the number of sheets,
    /// but generally there won't be too many sheets in a Vault
    pub fn sheet_names(&self) -> Result<Vec<SheetName>, std::io::Error> {
        // Get the sheets directory path
        let sheets_dir = self.vault_path.join(SERVER_PATH_SHEETS);

        // If the directory doesn't exist, return an empty list
        if !sheets_dir.exists() {
            return Ok(vec![]);
        }

        let mut sheet_names = Vec::new();

        // Iterate through all files in the sheets directory
        for entry in std::fs::read_dir(sheets_dir)? {
            let entry = entry?;
            let path = entry.path();

            // Check if it's a YAML file
            if path.is_file()
                && path
                    .extension()
                    .is_some_and(|ext| ext == SERVER_SUFFIX_SHEET_FILE_NO_DOT)
                && let Some(file_stem) = path.file_stem().and_then(|s| s.to_str())
            {
                // Create a new SheetName and add it to the result list
                sheet_names.push(file_stem.to_string());
            }
        }

        Ok(sheet_names)
    }

    /// Read a sheet from its name
    ///
    /// If the sheet information is successfully found in the vault,
    /// it will be deserialized and read as a sheet.
    /// This is the only correct way to obtain a sheet instance.
    pub async fn sheet<'a>(&'a self, sheet_name: &SheetName) -> Result<Sheet<'a>, std::io::Error> {
        let sheet_name = snake_case!(sheet_name.clone());

        // Get the path to the sheet file
        let sheet_path = Sheet::sheet_path_with_name(self, &sheet_name);

        // Ensure the sheet file exists
        if !sheet_path.exists() {
            // If the sheet does not exist, try to restore it from the trash
            if self.restore_sheet(&sheet_name).await.is_err() {
                // If restoration fails, return an error
                return Err(Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Sheet `{}` not found!", sheet_name),
                ));
            }
        }

        // Read the sheet data from the file
        let data = SheetData::read_from(sheet_path).await?;

        Ok(Sheet {
            name: sheet_name.clone(),
            data,
            vault_reference: self,
        })
    }

    /// Create a sheet locally and return the sheet instance
    ///
    /// This method creates a new sheet in the vault with the given name and holder.
    /// It will verify that the member exists and that the sheet doesn't already exist
    /// before creating the sheet file with default empty data.
    pub async fn create_sheet<'a>(
        &'a self,
        sheet_name: &SheetName,
        holder: &MemberId,
    ) -> Result<Sheet<'a>, std::io::Error> {
        let sheet_name = snake_case!(sheet_name.clone());

        // Ensure member exists
        if !self.member_cfg_path(holder).exists() {
            return Err(Error::new(
                std::io::ErrorKind::NotFound,
                format!("Member `{}` not found!", &holder),
            ));
        }

        // Ensure sheet does not already exist
        let sheet_file_path = Sheet::sheet_path_with_name(self, &sheet_name);
        if sheet_file_path.exists() {
            return Err(Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("Sheet `{}` already exists!", &sheet_name),
            ));
        }

        // Create the sheet file
        let sheet_data = SheetData {
            holder: Some(holder.clone()),
            inputs: Vec::new(),
            mapping: HashMap::new(),
            id_mapping: None,
            write_count: 0,
        };
        SheetData::write_to(&sheet_data, sheet_file_path).await?;

        Ok(Sheet {
            name: sheet_name,
            data: sheet_data,
            vault_reference: self,
        })
    }

    /// Delete the sheet file from local disk by name
    ///
    /// This method will remove the sheet file with the given name from the vault.
    /// It will verify that the sheet exists before attempting to delete it.
    /// If the sheet is successfully deleted, it will return Ok(()).
    ///
    /// Warning: This operation is dangerous. Deleting a sheet will cause local workspaces
    /// using this sheet to become invalid. Please ensure the sheet is not currently in use
    /// and will not be used in the future.
    ///
    /// For a safer deletion method, consider using `delete_sheet_safety`.
    ///
    /// Note: This function is intended for server-side use only and should not be
    /// arbitrarily called by other members to prevent unauthorized data deletion.
    pub async fn delete_sheet(&self, sheet_name: &SheetName) -> Result<(), std::io::Error> {
        let sheet_name = snake_case!(sheet_name.clone());

        // Ensure sheet exists
        let sheet_file_path = Sheet::sheet_path_with_name(self, &sheet_name);
        if !sheet_file_path.exists() {
            return Err(Error::new(
                std::io::ErrorKind::NotFound,
                format!("Sheet `{}` not found!", &sheet_name),
            ));
        }

        // Delete the sheet file
        fs::remove_file(sheet_file_path).await?;

        Ok(())
    }

    /// Safely delete the sheet
    ///
    /// The sheet will be moved to the trash directory, ensuring it does not appear in the
    /// results of `sheets` and `sheet_names` methods.
    /// However, if the sheet's holder attempts to access the sheet through the `sheet` method,
    /// the system will automatically restore it from the trash directory.
    /// This means: the sheet will only permanently remain in the trash directory,
    /// waiting for manual cleanup by an administrator, when it is truly no longer in use.
    ///
    /// This is a safer deletion method because it provides the possibility of recovery,
    /// avoiding irreversible data loss caused by accidental deletion.
    ///
    /// Note: This function is intended for server-side use only and should not be
    /// arbitrarily called by other members to prevent unauthorized data deletion.
    pub async fn delete_sheet_safely(&self, sheet_name: &SheetName) -> Result<(), std::io::Error> {
        let sheet_name = snake_case!(sheet_name.clone());

        // Ensure the sheet exists
        let sheet_file_path = Sheet::sheet_path_with_name(self, &sheet_name);
        if !sheet_file_path.exists() {
            return Err(Error::new(
                std::io::ErrorKind::NotFound,
                format!("Sheet `{}` not found!", &sheet_name),
            ));
        }

        // Create the trash directory
        let trash_dir = self.vault_path.join(".trash");
        if !trash_dir.exists() {
            fs::create_dir_all(&trash_dir).await?;
        }

        // Generate a unique filename in the trash
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let trash_file_name = format!(
            "{}_{}.{}",
            sheet_name, timestamp, SERVER_SUFFIX_SHEET_FILE_NO_DOT
        );
        let trash_path = trash_dir.join(trash_file_name);

        // Move the sheet file to the trash
        fs::rename(&sheet_file_path, &trash_path).await?;

        Ok(())
    }

    /// Restore the sheet from the trash
    ///
    /// Restore the specified sheet from the trash to its original location, making it accessible normally.
    pub async fn restore_sheet(&self, sheet_name: &SheetName) -> Result<(), std::io::Error> {
        let sheet_name = snake_case!(sheet_name.clone());

        // Search for matching files in the trash
        let trash_dir = self.vault_path.join(".trash");
        if !trash_dir.exists() {
            return Err(Error::new(
                std::io::ErrorKind::NotFound,
                "Trash directory does not exist!".to_string(),
            ));
        }

        let mut found_path = None;
        for entry in std::fs::read_dir(&trash_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file()
                && let Some(file_name) = path.file_stem().and_then(|s| s.to_str())
            {
                // Check if the filename starts with the sheet name
                if file_name.starts_with(&sheet_name) {
                    found_path = Some(path);
                    break;
                }
            }
        }

        let trash_path = found_path.ok_or_else(|| {
            Error::new(
                std::io::ErrorKind::NotFound,
                format!("Sheet `{}` not found in trash!", &sheet_name),
            )
        })?;

        // Restore the sheet to its original location
        let original_path = Sheet::sheet_path_with_name(self, &sheet_name);
        fs::rename(&trash_path, &original_path).await?;

        Ok(())
    }
}
