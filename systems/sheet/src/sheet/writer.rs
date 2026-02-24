use crate::sheet::SheetData;

include!("current.rs");

/// Convert SheetData to byte array
pub fn convert_sheet_data_to_bytes(sheet_data: SheetData) -> Vec<u8> {
    writer::convert_sheet_data_to_bytes(sheet_data)
}
