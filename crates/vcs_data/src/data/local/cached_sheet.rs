use std::{io::Error, path::PathBuf};

use cfg_file::config::ConfigFile;
use string_proc::snake_case;

use crate::{
    constants::CLIENT_FILE_CACHED_SHEET,
    current::current_local_path,
    data::{
        member::MemberId,
        sheet::{SheetData, SheetName},
    },
};

const SHEET_NAME: &str = "{sheet_name}";
const ACCOUNT_NAME: &str = "{account}";

pub struct CachedSheet;

impl CachedSheet {
    /// Read the cached sheet data.
    pub async fn cached_sheet_data(
        account_name: MemberId,
        sheet_name: SheetName,
    ) -> Result<SheetData, std::io::Error> {
        let account_name = snake_case!(account_name);
        let sheet_name = snake_case!(sheet_name);

        let Some(path) = Self::cached_sheet_path(account_name, sheet_name) else {
            return Err(Error::new(
                std::io::ErrorKind::NotFound,
                "Local workspace not found!",
            ));
        };
        let data = SheetData::read_from(path).await?;
        Ok(data)
    }

    /// Get the path to the cached sheet file.
    pub fn cached_sheet_path(account_name: MemberId, sheet_name: SheetName) -> Option<PathBuf> {
        let current_workspace = current_local_path()?;
        Some(
            current_workspace.join(
                CLIENT_FILE_CACHED_SHEET
                    .replace(SHEET_NAME, &sheet_name.to_string())
                    .replace(ACCOUNT_NAME, &account_name.to_string()),
            ),
        )
    }
}
