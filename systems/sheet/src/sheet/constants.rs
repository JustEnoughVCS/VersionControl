// Header (15: 1 + 2 + 4 + 4 + 4)
//
// [SHEET_VERSION: u8]
// [MAPPING_BUCKET_COUNT: u16]
// [INDEX_COUNT: u32]
// [OFFSET_MAPPING_DIR: u32]
// [OFFSET_INDEX_TABLE: u32]

pub const CURRENT_SHEET_VERSION: u8 = 1;
pub const HEADER_SIZE: usize = 0
    + 1 // SHEET_VERSION
    + 2 // MAPPING_BUCKET_COUNT
    + 4 // INDEX_COUNT
    + 4 // OFFSET_MAPPING_DIR
    + 4 // OFFSET_INDEX_TABLE
;

// Mapping Directory (12: 4 + 4 + 4)
//
// [BUCKET_HASH_PREFIX: u32]
// [BUCKET_OFFSET: u32]
// [BUCKET_LENGTH: u32]

pub const MAPPING_DIR_ENTRY_SIZE: usize = 0
    + 4 // BUCKET_HASH_PREFIX
    + 4 // BUCKET_OFFSET
    + 4 // BUCKET_LENGTH
;

// Mapping Buckets (6 + 1b + N)
//
// [KEY_LEN: u8]
// [FORWARD_TYPE: byte]
// [FORWARD_INFO_LEN: u8]
// [KEY_BYTES: ?]
// [FORWARD_INFO_BYTES: ?]
// [INDEX_OFFSET: u32]

pub const MAPPING_BUCKET_MIN_SIZE: usize = 0
    + 1 // KEY_LEN
    + 1 // FORWARD_TYPE
    + 1 // FORWARD_INFO_LEN
    + 2 // KEY_BYTES (MIN:1) + FORWARD_INFO_BYTES (MIN:1)
    + 2 // INDEX_OFFSET
;

// Index Table (6: 4 + 2)
//
// [INDEX_ID: u32]
// [INDEX_VERSION: u16]

pub const INDEX_ENTRY_SIZE: usize = 0
    + 4 // INDEX_ID
    + 2 // INDEX_VERSION
;
