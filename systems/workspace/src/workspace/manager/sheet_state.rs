use std::path::PathBuf;

use crate::workspace::manager::WorkspaceManager;
use asset_system::asset::ReadOnlyAsset;
use constants::workspace::files::{workspace_file_current_sheet, workspace_file_sheet};
use framework::space::error::SpaceError;
use just_fmt::snake_case;
use sheet_system::sheet::{Sheet, SheetData};

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
    pub async fn edit_using_sheet_name(&self, name: &str) -> Result<(), SpaceError> {
        self.space
            .write(workspace_file_current_sheet(), name.as_bytes())
            .await
    }

    /// Read a sheet from the workspace space by name
    ///
    /// Simple read of Sheet data, no disk write operations involved
    pub async fn read_sheet(&self, sheet_name: &str) -> Option<Sheet> {
        let sheet_name = snake_case!(sheet_name);
        let sheet_path = self.get_sheet_path(&sheet_name);

        let mut sheet_data = SheetData::empty();
        if sheet_path.exists() {
            // If reading fails, treat it as if the sheet does not exist and return `None`
            sheet_data.full_read(sheet_path).await.ok()?;
            return Some(sheet_data.pack(sheet_name));
        } else {
            None
        }
    }

    /// Get a resource pointing to local Sheet data by name
    ///
    /// Can be used to load content, edit, and transactionally write
    pub fn get_sheet_data_asset(&self, sheet_name: &str) -> Option<ReadOnlyAsset<SheetData>> {
        let sheet_name = snake_case!(sheet_name);
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
            .local_path(workspace_file_sheet(&sheet_name))
            // The `local_path` only produces path formatting errors.
            // If the path cannot be guaranteed to be correct,
            //   execution should not continue, so we unwrap()
            .unwrap()
    }
}
