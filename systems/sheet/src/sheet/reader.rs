use constants::CURRENT_SHEET_VERSION;

use crate::{
    mapping::{LocalMappingForward, Mapping},
    sheet::{SheetData, error::ReadSheetDataError},
};

include!("current.rs");

macro_rules! reader_do {
    ($full_sheet_data:expr, $func:ident($($arg:expr),*)) => {{
        let sheet_version = $full_sheet_data
            .first()
            .copied()
            .unwrap_or(CURRENT_SHEET_VERSION);
        match sheet_version {
            1 => crate::sheet::v1::reader::$func($($arg),*),
            _ => reader::$func($($arg),*),
        }
    }};
}

/// Reconstruct complete SheetData from full sheet data
pub fn read_sheet_data(full_sheet_data: &[u8]) -> Result<SheetData, ReadSheetDataError> {
    reader_do!(full_sheet_data, read_sheet_data(full_sheet_data))
}

/// Read mapping information for a specific node from complete sheet data
pub fn read_mapping<'a>(
    full_sheet_data: &'a [u8],
    node: &[&str],
) -> Result<Option<(Mapping<'a>, LocalMappingForward)>, ReadSheetDataError> {
    reader_do!(full_sheet_data, read_mapping(full_sheet_data, node))
}
