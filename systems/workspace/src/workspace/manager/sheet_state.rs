use std::path::PathBuf;

use crate::workspace::manager::{WorkspaceManager, sheet_state::error::MakeSheetError};
use asset_system::asset::ReadOnlyAsset;
use constants::workspace::{
    dirs::workspace_dir_local_sheets,
    files::{workspace_file_current_sheet, workspace_file_sheet},
    values::workspace_value_current_sheet_file_name,
};
use framework::space::error::SpaceError;
use just_fmt::snake_case;
use sheet_system::sheet::{Sheet, SheetData};

pub mod error;

impl WorkspaceManager {
    /// Read the name of the currently active Sheet
    pub async fn using_sheet_name(&self) -> Result<Option<String>, SpaceError> {
        match self
            .space
            .read_to_string(workspace_file_current_sheet())
            .await
        {
            Ok(s) => Ok(Some(s.trim().to_string())),
            Err(SpaceError::Io(io_error)) => match io_error.kind() {
                std::io::ErrorKind::NotFound => Ok(None),
                _ => Err(SpaceError::Io(io_error)),
            },
            Err(e) => Err(e),
        }
    }

    /// Set the name of the currently active Sheet
    pub async fn edit_using_sheet_name(&self, name: impl AsRef<str>) -> Result<(), SpaceError> {
        self.space
            .write(workspace_file_current_sheet(), name.as_ref().as_bytes())
            .await
    }

    /// Read a sheet from the workspace space by name
    ///
    /// Simple read of Sheet data, no disk write operations involved
    pub async fn read_sheet(&self, sheet_name: impl AsRef<str>) -> Option<Sheet> {
        let sheet_name = snake_case!(sheet_name.as_ref());
        let sheet_path = self.get_sheet_path(&sheet_name);

        let mut sheet_data = SheetData::empty();
        if sheet_path.exists() {
            // If reading fails, treat it as if the sheet does not exist and return `None`
            sheet_data.full_read(sheet_path).await.ok()?;
            Some(sheet_data.pack(sheet_name))
        } else {
            None
        }
    }

    /// Get a resource pointing to local Sheet data by name
    ///
    /// Can be used to load content, edit, and transactionally write
    pub fn get_sheet_data_asset(
        &self,
        sheet_name: impl AsRef<str>,
    ) -> Option<ReadOnlyAsset<SheetData>> {
        let sheet_name = snake_case!(sheet_name.as_ref());
        let sheet_path = self.get_sheet_path(&sheet_name);
        if sheet_path.exists() {
            return Some(sheet_path.into());
        }
        None
    }

    /// Get the local filesystem path for a sheet by name
    pub fn get_sheet_path(&self, sheet_name: impl AsRef<str>) -> PathBuf {
        let sheet_name = sheet_name.as_ref();
        self.space
            .local_path(workspace_file_sheet(sheet_name))
            // The `local_path` only produces path formatting errors.
            // If the path cannot be guaranteed to be correct,
            //   execution should not continue, so we unwrap()
            .unwrap()
    }

    /// Get the names of all sheets in the workspace
    pub async fn list_sheet_names(&self) -> Vec<String> {
        let mut sheet_names = Vec::new();
        if let Ok(mut read_dir) = self.space.read_dir(workspace_dir_local_sheets()).await {
            while let Some(entry) = read_dir.next_entry().await.ok().flatten() {
                let path = entry.path();
                if path.is_file()
                    && let Some(file_name) = path.file_stem()
                    && let Some(name_str) = file_name.to_str()
                    && name_str != workspace_value_current_sheet_file_name()
                {
                    sheet_names.push(name_str.to_string());
                }
            }
        }
        sheet_names
    }

    /// Create a new sheet in the workspace by name
    pub async fn make_sheet(&self, sheet_name: impl AsRef<str>) -> Result<(), MakeSheetError> {
        let sheet_dir = workspace_dir_local_sheets();
        let sheet_path = workspace_file_sheet(sheet_name.as_ref());
        if self.space.exists(&sheet_path).await? {
            return Err(MakeSheetError::SheetAlreadyExists);
        }
        self.space.create_dir_all(&sheet_dir).await?;
        self.space
            .write(sheet_path, SheetData::empty().as_bytes())
            .await?;

        Ok(())
    }

    /// Delete a sheet from the workspace by name
    pub async fn drop_sheet(&self, sheet_name: impl AsRef<str>) -> Result<(), MakeSheetError> {
        let sheet_path = workspace_file_sheet(sheet_name.as_ref());
        if !self.space.exists(&sheet_path).await? {
            return Err(MakeSheetError::SheetNotFound);
        }
        self.space.remove_file(sheet_path).await?;
        Ok(())
    }
}
