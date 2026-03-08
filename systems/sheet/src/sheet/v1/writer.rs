use crate::index_source::IndexSource;
use crate::mapping::LocalMapping;
use crate::sheet::SheetData;
use crate::sheet::v1::constants::{
    CURRENT_SHEET_VERSION, HEADER_SIZE, INDEX_ENTRY_SIZE, MAPPING_DIR_ENTRY_SIZE,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};

pub fn convert_sheet_data_to_bytes(sheet_data: SheetData) -> Vec<u8> {
    // Collect all mappings
    let mappings: Vec<LocalMapping> = sheet_data.mappings.into_iter().collect();

    // Collect all unique index sources
    let mut index_sources = Vec::new();
    let mut source_to_offset = HashMap::new();

    for mapping in &mappings {
        let source = mapping.index_source();
        let key = (source.is_remote(), source.id(), source.version());
        if !source_to_offset.contains_key(&key) {
            let offset = index_sources.len() as u32;
            source_to_offset.insert(key, offset);
            index_sources.push(IndexSource::new(
                source.is_remote(),
                source.id(),
                source.version(),
            ));
        }
    }

    let index_count = index_sources.len() as u32;

    // 1. Organize mappings into hash buckets
    let mut buckets: BTreeMap<u32, Vec<LocalMapping>> = BTreeMap::new();
    for mapping in mappings {
        let hash = calculate_path_hash(mapping.value());
        let bucket_key = hash >> 16; // Take high 16 bits as bucket key
        buckets
            .entry(bucket_key)
            .or_insert_with(Vec::new)
            .push(mapping);
    }

    let bucket_count = buckets.len() as u16;

    // 2. Calculate offsets for each section
    let header_size = HEADER_SIZE;
    let mapping_dir_offset = header_size;
    let mapping_dir_size = bucket_count as usize * MAPPING_DIR_ENTRY_SIZE;
    let index_table_offset = mapping_dir_offset + mapping_dir_size;
    let index_table_size = index_count as usize * INDEX_ENTRY_SIZE;

    // 3. Calculate bucket data offsets
    let mut bucket_data_offset = index_table_offset + index_table_size;
    let mut bucket_entries = Vec::new();

    // Prepare data for each bucket
    for (&bucket_key, bucket_mappings) in &buckets {
        // Sort mappings within bucket by path for binary search
        let mut sorted_mappings: Vec<&LocalMapping> = bucket_mappings.iter().collect();
        sorted_mappings.sort_by(|a, b| a.value().cmp(b.value()));

        // Calculate bucket data size
        let mut bucket_data = Vec::new();
        for mapping in sorted_mappings {
            write_mapping_bucket(&mut bucket_data, mapping, &source_to_offset);
        }

        let bucket_length = bucket_data.len() as u32;
        bucket_entries.push((bucket_key, bucket_data_offset, bucket_length, bucket_data));
        bucket_data_offset += bucket_length as usize;
    }

    // 4. Build result
    let total_size = bucket_data_offset;
    let mut result = Vec::with_capacity(total_size);

    // 5. File header
    result.push(CURRENT_SHEET_VERSION); // Version (1 byte)
    result.extend_from_slice(&bucket_count.to_le_bytes()); // Mapping bucket count (2 bytes)
    result.extend_from_slice(&index_count.to_le_bytes()); // Index count (4 bytes)
    result.extend_from_slice(&(mapping_dir_offset as u32).to_le_bytes()); // Mapping directory offset (4 bytes)
    result.extend_from_slice(&(index_table_offset as u32).to_le_bytes()); // Index table offset (4 bytes)

    // 6. Mapping directory
    for (bucket_key, bucket_offset, bucket_length, _) in &bucket_entries {
        result.extend_from_slice(&bucket_key.to_le_bytes()); // Bucket hash prefix (4 bytes)
        result.extend_from_slice(&(*bucket_offset as u32).to_le_bytes()); // Bucket offset (4 bytes)
        result.extend_from_slice(&bucket_length.to_le_bytes()); // Bucket length (4 bytes)
    }

    // 7. Index table
    for source in &index_sources {
        result.extend_from_slice(&source.id().to_le_bytes()); // Index ID (4 bytes)
        result.extend_from_slice(&source.version().to_le_bytes()); // Index version (2 bytes)
        result.push(if source.is_remote() { 1 } else { 0 }); // Remote flag (1 byte)
        result.extend_from_slice(&[0u8; 3]); // Reserved bytes (3 bytes)
    }

    // 8. Bucket data
    for (_, _, _, bucket_data) in bucket_entries {
        result.extend_from_slice(&bucket_data);
    }

    result
}

pub fn calculate_path_hash(path: &[String]) -> u32 {
    let mut hasher = Sha256::new();
    for segment in path {
        hasher.update(segment.as_bytes());
        hasher.update(b"/");
    }
    let result = hasher.finalize();
    u32::from_le_bytes([result[0], result[1], result[2], result[3]])
}

/// Write single mapping to bucket data
fn write_mapping_bucket(
    result: &mut Vec<u8>,
    mapping: &LocalMapping,
    source_to_offset: &HashMap<(bool, u32, u16), u32>,
) {
    // Serialize path
    let path_bytes = serialize_path(mapping.value());
    let path_len = path_bytes.len();

    // Get forward information
    let (forward_type, forward_info_len, forward_bytes) = mapping.forward().unpack();

    // Get index offset
    let source = mapping.index_source();
    let key = (source.is_remote(), source.id(), source.version());
    let index_offset = source_to_offset.get(&key).unwrap();

    // Write mapping bucket entry
    result.push(path_len as u8); // Key length (1 byte)
    result.push(forward_type); // Forward type (1 byte)
    result.push(forward_info_len); // Forward info length (1 byte)

    // Write key data (path)
    result.extend_from_slice(&path_bytes);

    // Write forward info data
    result.extend_from_slice(&forward_bytes);

    // Write index offset
    result.extend_from_slice(&index_offset.to_le_bytes()); // Index offset (4 bytes)
}

/// Serialize path to byte array
fn serialize_path(path: &[String]) -> Vec<u8> {
    let mut result = Vec::new();
    for (i, segment) in path.iter().enumerate() {
        result.extend_from_slice(segment.as_bytes());
        if i < path.len() - 1 {
            result.push(b'/');
        }
    }
    result
}

/// Test only: Calculate single mapping bucket entry size
#[cfg(test)]
fn calculate_mapping_bucket_size(mapping: &LocalMapping) -> usize {
    use crate::sheet::v1::constants::MAPPING_BUCKET_MIN_SIZE;

    let path_size = serialize_path(mapping.value()).len();
    let (_, forward_info_len, _) = mapping.forward().unpack();

    MAPPING_BUCKET_MIN_SIZE + path_size + forward_info_len as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{mapping::LocalMappingForward, sheet::v1::constants::MAPPING_BUCKET_MIN_SIZE};

    #[test]
    fn test_serialize_path() {
        let path = vec![
            "dir".to_string(),
            "subdir".to_string(),
            "file.txt".to_string(),
        ];
        let bytes = serialize_path(&path);
        assert_eq!(bytes, b"dir/subdir/file.txt");
    }

    #[test]
    fn test_calculate_path_hash() {
        let path1 = vec!["test".to_string(), "file.txt".to_string()];
        let path2 = vec!["test".to_string(), "file.txt".to_string()];
        let path3 = vec!["other".to_string(), "file.txt".to_string()];

        let hash1 = calculate_path_hash(&path1);
        let hash2 = calculate_path_hash(&path2);
        let hash3 = calculate_path_hash(&path3);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_calculate_mapping_bucket_size() {
        let mapping = LocalMapping::new(
            vec!["test".to_string(), "file.txt".to_string()],
            IndexSource::new_local(1, 1),
            LocalMappingForward::Latest,
        )
        .unwrap();

        let size = calculate_mapping_bucket_size(&mapping);
        // 13 == "test/file.txt".len()
        assert_eq!(size, MAPPING_BUCKET_MIN_SIZE + 13);
    }

    #[test]
    fn test_convert_empty_sheet() {
        let sheet_data = SheetData::empty();
        let bytes = convert_sheet_data_to_bytes(sheet_data);

        // Verify file header
        assert_eq!(bytes[0], CURRENT_SHEET_VERSION); // Version
        assert_eq!(u16::from_le_bytes([bytes[1], bytes[2]]), 0); // Mapping bucket count
        assert_eq!(
            u32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]),
            0
        ); // Index count

        // Total size should be HEADER_SIZE
        assert_eq!(bytes.len(), HEADER_SIZE);
    }

    #[test]
    fn test_convert_sheet_with_one_mapping() {
        let mut sheet_data = SheetData::empty();
        let mapping = LocalMapping::new(
            vec!["dir".to_string(), "file.txt".to_string()],
            IndexSource::new_local(1, 1),
            LocalMappingForward::Latest,
        )
        .unwrap();
        sheet_data.mappings.insert(mapping);

        let bytes = convert_sheet_data_to_bytes(sheet_data);

        // Verify file header
        assert_eq!(bytes[0], CURRENT_SHEET_VERSION); // Version
        assert_eq!(u16::from_le_bytes([bytes[1], bytes[2]]), 1); // Should have 1 bucket
        assert_eq!(
            u32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]),
            1
        ); // 1 index source

        // Verify mapping directory
        let mapping_dir_offset = HEADER_SIZE;

        // Bucket offset should point after the index table
        let index_table_offset =
            u32::from_le_bytes([bytes[11], bytes[12], bytes[13], bytes[14]]) as usize;
        let bucket_offset = u32::from_le_bytes([
            bytes[mapping_dir_offset + 4],
            bytes[mapping_dir_offset + 5],
            bytes[mapping_dir_offset + 6],
            bytes[mapping_dir_offset + 7],
        ]) as usize;

        assert!(bucket_offset >= index_table_offset + INDEX_ENTRY_SIZE);
    }
}
