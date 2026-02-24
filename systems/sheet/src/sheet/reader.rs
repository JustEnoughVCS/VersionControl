use crate::{
    mapping::{LocalMappingForward, Mapping},
    sheet::{SheetData, error::ReadSheetDataError},
};

include!("current.rs");

/// Reconstruct complete SheetData from full sheet data
pub fn read_sheet_data(full_sheet_data: &[u8]) -> Result<SheetData, ReadSheetDataError> {
    reader::read_sheet_data(full_sheet_data)
}

/// Read mapping information for a specific node from complete sheet data
pub fn read_mapping<'a>(
    full_sheet_data: &'a [u8],
    node: &[&str],
) -> Result<Option<(Mapping<'a>, LocalMappingForward)>, ReadSheetDataError> {
    reader::read_mapping(full_sheet_data, node)
}
