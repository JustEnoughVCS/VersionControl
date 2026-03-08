use crate::{
    index_source::IndexSource,
    mapping::{LocalMapping, LocalMappingForward, Mapping},
    sheet::{
        SheetData,
        error::ReadSheetDataError,
        v1::constants::{
            CURRENT_SHEET_VERSION, HEADER_SIZE, INDEX_ENTRY_SIZE, MAPPING_BUCKET_MIN_SIZE,
            MAPPING_DIR_ENTRY_SIZE,
        },
        v1::writer::calculate_path_hash,
    },
};
use std::collections::HashSet;

pub fn read_sheet_data(full_sheet_data: &[u8]) -> Result<SheetData, ReadSheetDataError> {
    if full_sheet_data.len() < HEADER_SIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Sheet data too small for header",
        )
        .into());
    }

    // Read file header
    let version = full_sheet_data[0];
    if version != CURRENT_SHEET_VERSION {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Unsupported sheet version: {}", version),
        )
        .into());
    }

    let bucket_count = u16::from_le_bytes([full_sheet_data[1], full_sheet_data[2]]) as usize;
    let index_count = u32::from_le_bytes([
        full_sheet_data[3],
        full_sheet_data[4],
        full_sheet_data[5],
        full_sheet_data[6],
    ]) as usize;

    let mapping_dir_offset = u32::from_le_bytes([
        full_sheet_data[7],
        full_sheet_data[8],
        full_sheet_data[9],
        full_sheet_data[10],
    ]) as usize;

    let index_table_offset = u32::from_le_bytes([
        full_sheet_data[11],
        full_sheet_data[12],
        full_sheet_data[13],
        full_sheet_data[14],
    ]) as usize;

    // Validate offsets
    if mapping_dir_offset > full_sheet_data.len() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Mapping directory offset out of bounds",
        )
        .into());
    }

    if index_table_offset > full_sheet_data.len() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Index table offset out of bounds",
        )
        .into());
    }

    // Read index table
    let index_sources = read_index_table(full_sheet_data, index_table_offset, index_count)?;

    // Read mapping directory and build all mappings
    let mut mappings = HashSet::new();
    let mapping_dir_end = mapping_dir_offset + bucket_count * MAPPING_DIR_ENTRY_SIZE;

    if mapping_dir_end > full_sheet_data.len() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Mapping directory exceeds buffer",
        )
        .into());
    }

    // Iterate through all buckets
    for i in 0..bucket_count {
        let dir_entry_offset = mapping_dir_offset + i * MAPPING_DIR_ENTRY_SIZE;

        // Skip BUCKET_HASH_PREFIX, directly read BUCKET_OFFSET and BUCKET_LENGTH
        let bucket_offset = u32::from_le_bytes([
            full_sheet_data[dir_entry_offset + 4],
            full_sheet_data[dir_entry_offset + 5],
            full_sheet_data[dir_entry_offset + 6],
            full_sheet_data[dir_entry_offset + 7],
        ]) as usize;

        let bucket_length = u32::from_le_bytes([
            full_sheet_data[dir_entry_offset + 8],
            full_sheet_data[dir_entry_offset + 9],
            full_sheet_data[dir_entry_offset + 10],
            full_sheet_data[dir_entry_offset + 11],
        ]) as usize;

        // Read bucket data
        if bucket_offset + bucket_length > full_sheet_data.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!("Bucket data exceeds buffer (bucket {})", i),
            )
            .into());
        }

        let bucket_data = &full_sheet_data[bucket_offset..bucket_offset + bucket_length];
        let bucket_mappings = read_bucket_data(bucket_data, &index_sources)?;

        for mapping in bucket_mappings {
            mappings.insert(mapping);
        }
    }

    Ok(SheetData { mappings })
}

pub fn read_mapping<'a>(
    full_sheet_data: &'a [u8],
    node: &[&str],
) -> Result<Option<(Mapping<'a>, LocalMappingForward)>, ReadSheetDataError> {
    if full_sheet_data.len() < HEADER_SIZE {
        return Ok(None);
    }

    // Read file header
    let version = full_sheet_data[0];
    if version != CURRENT_SHEET_VERSION {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Unsupported sheet version: {}", version),
        )
        .into());
    }

    let bucket_count = u16::from_le_bytes([full_sheet_data[1], full_sheet_data[2]]) as usize;
    let index_count = u32::from_le_bytes([
        full_sheet_data[3],
        full_sheet_data[4],
        full_sheet_data[5],
        full_sheet_data[6],
    ]) as usize;

    let mapping_dir_offset = u32::from_le_bytes([
        full_sheet_data[7],
        full_sheet_data[8],
        full_sheet_data[9],
        full_sheet_data[10],
    ]) as usize;

    let index_table_offset = u32::from_le_bytes([
        full_sheet_data[11],
        full_sheet_data[12],
        full_sheet_data[13],
        full_sheet_data[14],
    ]) as usize;

    // Validate offsets
    if mapping_dir_offset > full_sheet_data.len() || index_table_offset > full_sheet_data.len() {
        return Ok(None);
    }

    // Read index table
    let index_sources = read_index_table(full_sheet_data, index_table_offset, index_count)?;

    // Calculate hash prefix for target node
    let node_path: Vec<String> = node.iter().map(|s| s.to_string()).collect();
    let target_hash = calculate_path_hash(&node_path);
    let target_bucket_key = target_hash >> 16; // Take high 16 bits as bucket key

    // Find corresponding bucket in mapping directory using binary search
    let mapping_dir_end = mapping_dir_offset + bucket_count * MAPPING_DIR_ENTRY_SIZE;
    if mapping_dir_end > full_sheet_data.len() {
        return Ok(None);
    }

    // Binary search for the bucket with matching hash prefix
    let mut left = 0;
    let mut right = bucket_count;

    while left < right {
        let mid = left + (right - left) / 2;
        let dir_entry_offset = mapping_dir_offset + mid * MAPPING_DIR_ENTRY_SIZE;

        let bucket_hash_prefix = u32::from_le_bytes([
            full_sheet_data[dir_entry_offset],
            full_sheet_data[dir_entry_offset + 1],
            full_sheet_data[dir_entry_offset + 2],
            full_sheet_data[dir_entry_offset + 3],
        ]);

        if bucket_hash_prefix < target_bucket_key {
            left = mid + 1;
        } else if bucket_hash_prefix > target_bucket_key {
            right = mid;
        } else {
            // Found matching bucket
            let bucket_offset = u32::from_le_bytes([
                full_sheet_data[dir_entry_offset + 4],
                full_sheet_data[dir_entry_offset + 5],
                full_sheet_data[dir_entry_offset + 6],
                full_sheet_data[dir_entry_offset + 7],
            ]) as usize;

            let bucket_length = u32::from_le_bytes([
                full_sheet_data[dir_entry_offset + 8],
                full_sheet_data[dir_entry_offset + 9],
                full_sheet_data[dir_entry_offset + 10],
                full_sheet_data[dir_entry_offset + 11],
            ]) as usize;

            // Read bucket data and find target node
            if bucket_offset + bucket_length > full_sheet_data.len() {
                break;
            }

            let bucket_data = &full_sheet_data[bucket_offset..bucket_offset + bucket_length];
            return find_mapping_in_bucket(bucket_data, node, &index_sources);
        }
    }

    Ok(None)
}

/// Read index table
fn read_index_table(
    data: &[u8],
    offset: usize,
    count: usize,
) -> Result<Vec<IndexSource>, ReadSheetDataError> {
    let table_size = count * INDEX_ENTRY_SIZE;
    if offset + table_size > data.len() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Index table exceeds buffer",
        )
        .into());
    }

    let mut sources = Vec::with_capacity(count);
    let mut pos = offset;

    for _ in 0..count {
        if pos + INDEX_ENTRY_SIZE > data.len() {
            break;
        }

        let id = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        let ver = u16::from_le_bytes([data[pos + 4], data[pos + 5]]);
        let remote = data[pos + 6] != 0; // 0 = local, non-zero = remote

        sources.push(IndexSource::new(remote, id, ver));
        pos += INDEX_ENTRY_SIZE;
    }

    Ok(sources)
}

/// Read all mappings in bucket data
fn read_bucket_data(
    bucket_data: &[u8],
    index_sources: &[IndexSource],
) -> Result<Vec<LocalMapping>, ReadSheetDataError> {
    let mut mappings = Vec::new();
    let mut pos = 0;

    while pos < bucket_data.len() {
        if pos + MAPPING_BUCKET_MIN_SIZE > bucket_data.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Incomplete mapping bucket entry",
            )
            .into());
        }

        // Read mapping bucket entry header
        let key_len = bucket_data[pos] as usize;
        let forward_type = bucket_data[pos + 1];
        let forward_info_len = bucket_data[pos + 2] as usize;

        pos += 3; // KEY_LEN + FORWARD_TYPE + FORWARD_INFO_LEN

        // Check bounds
        if pos + key_len > bucket_data.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Key data exceeds buffer",
            )
            .into());
        }

        // Read key data (path)
        let key_bytes = &bucket_data[pos..pos + key_len];
        let path = deserialize_path(key_bytes)?;
        pos += key_len;

        // Read forward info data
        if pos + forward_info_len > bucket_data.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Forward info data exceeds buffer",
            )
            .into());
        }

        let forward_bytes = &bucket_data[pos..pos + forward_info_len];
        pos += forward_info_len;

        // Read index offset
        if pos + 4 > bucket_data.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Index offset exceeds buffer",
            )
            .into());
        }

        let index_offset = u32::from_le_bytes([
            bucket_data[pos],
            bucket_data[pos + 1],
            bucket_data[pos + 2],
            bucket_data[pos + 3],
        ]) as usize;
        pos += 4;

        // Get index source
        if index_offset >= index_sources.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid index offset: {}", index_offset),
            )
            .into());
        }

        let source = index_sources[index_offset];

        // Build forward info
        let forward = LocalMappingForward::pack(forward_type, forward_bytes).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Failed to unpack forward info",
            )
        })?;

        // Create LocalMapping
        let mapping = LocalMapping::new(path, source, forward).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "Failed to create mapping")
        })?;

        mappings.push(mapping);
    }

    Ok(mappings)
}

/// Find mapping for specific node in bucket data
fn find_mapping_in_bucket<'a>(
    bucket_data: &'a [u8],
    node: &[&str],
    index_sources: &[IndexSource],
) -> Result<Option<(Mapping<'a>, LocalMappingForward)>, ReadSheetDataError> {
    // Build a list of entry start positions for binary search
    let entry_positions = build_entry_positions(bucket_data)?;

    if entry_positions.is_empty() {
        return Ok(None);
    }

    // Binary search on entry positions
    let mut left = 0;
    let mut right = entry_positions.len();

    while left < right {
        let mid = left + (right - left) / 2;
        let entry_start = entry_positions[mid];

        // Read entry header
        let (_, key_len, forward_type, forward_info_len) =
            read_entry_header(bucket_data, entry_start)?;

        // Read key data (path)
        let header_end = entry_start + 3;
        if header_end + key_len > bucket_data.len() {
            break;
        }

        let key_bytes = &bucket_data[header_end..header_end + key_len];
        let current_path = deserialize_path(key_bytes)?;

        // Compare with target node
        match compare_paths(&current_path, node) {
            std::cmp::Ordering::Less => {
                left = mid + 1;
            }
            std::cmp::Ordering::Greater => {
                right = mid;
            }
            std::cmp::Ordering::Equal => {
                // Found matching node
                // Read forward info data
                let forward_start = header_end + key_len;
                if forward_start + forward_info_len > bucket_data.len() {
                    break;
                }

                let forward_bytes = &bucket_data[forward_start..forward_start + forward_info_len];

                // Read index offset
                let index_offset_pos = forward_start + forward_info_len;
                if index_offset_pos + 4 > bucket_data.len() {
                    break;
                }

                let index_offset = u32::from_le_bytes([
                    bucket_data[index_offset_pos],
                    bucket_data[index_offset_pos + 1],
                    bucket_data[index_offset_pos + 2],
                    bucket_data[index_offset_pos + 3],
                ]) as usize;

                // Get index source
                if index_offset >= index_sources.len() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Invalid index offset: {}", index_offset),
                    )
                    .into());
                }

                let source = index_sources[index_offset];

                // Build forward info
                let forward =
                    LocalMappingForward::pack(forward_type, forward_bytes).ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Failed to unpack forward info",
                        )
                    })?;

                // Create Mapping
                let path_str = std::str::from_utf8(key_bytes).map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Invalid UTF-8 in path: {}", e),
                    )
                })?;
                let mapping = Mapping::new("", path_str, source);

                return Ok(Some((mapping, forward)));
            }
        }
    }

    Ok(None)
}

/// Build a list of all entry start positions in the bucket
fn build_entry_positions(bucket_data: &[u8]) -> Result<Vec<usize>, ReadSheetDataError> {
    let mut positions = Vec::new();
    let mut pos = 0;

    while pos < bucket_data.len() {
        if pos + MAPPING_BUCKET_MIN_SIZE > bucket_data.len() {
            break;
        }

        // Record this position as an entry start
        positions.push(pos);

        // Read entry header to get entry size
        let key_len = bucket_data[pos] as usize;
        let forward_info_len = bucket_data[pos + 2] as usize;

        // Calculate entry size: header(3) + key_len + forward_info_len + index_offset(4)
        let entry_size = 3 + key_len + forward_info_len + 4;

        // Move to next entry
        pos += entry_size;
    }

    Ok(positions)
}

/// Read entry header at the given position
fn read_entry_header(
    bucket_data: &[u8],
    pos: usize,
) -> Result<(usize, usize, u8, usize), ReadSheetDataError> {
    if pos + MAPPING_BUCKET_MIN_SIZE > bucket_data.len() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Incomplete mapping bucket entry",
        )
        .into());
    }

    let key_len = bucket_data[pos] as usize;
    let forward_type = bucket_data[pos + 1];
    let forward_info_len = bucket_data[pos + 2] as usize;

    Ok((pos, key_len, forward_type, forward_info_len))
}

/// Compare two paths lexicographically
fn compare_paths(path1: &[String], path2: &[&str]) -> std::cmp::Ordering {
    let min_len = std::cmp::min(path1.len(), path2.len());

    for i in 0..min_len {
        match path1[i].as_str().cmp(path2[i]) {
            std::cmp::Ordering::Equal => continue,
            ordering => return ordering,
        }
    }

    // If all compared segments are equal, compare lengths
    path1.len().cmp(&path2.len())
}

/// Deserialize path
fn deserialize_path(bytes: &[u8]) -> Result<Vec<String>, ReadSheetDataError> {
    let path_str = std::str::from_utf8(bytes).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Invalid UTF-8 in path: {}", e),
        )
    })?;

    if path_str.is_empty() {
        return Ok(Vec::new());
    }

    let segments: Vec<String> = path_str.split('/').map(|s| s.to_string()).collect();
    Ok(segments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_path() {
        let bytes = b"dir/subdir/file.txt";
        let path = deserialize_path(bytes).unwrap();
        assert_eq!(path, vec!["dir", "subdir", "file.txt"]);
    }

    #[test]
    fn test_paths_match() {
        let path = vec!["dir".to_string(), "file.txt".to_string()];
        let node = &["dir", "file.txt"];
        assert!(paths_match(&path, node));

        let node2 = &["dir", "other.txt"];
        assert!(!paths_match(&path, node2));
    }

    /// Check if paths match
    fn paths_match(path: &[String], node: &[&str]) -> bool {
        compare_paths(path, node) == std::cmp::Ordering::Equal
    }

    #[test]
    fn test_read_index_table() {
        let mut data = Vec::new();
        // First entry: local source
        data.extend_from_slice(&123u32.to_le_bytes());
        data.extend_from_slice(&456u16.to_le_bytes());
        data.push(0); // remote flag (0 = local)
        data.extend_from_slice(&[0u8; 3]); // reserved bytes

        // Second entry: remote source
        data.extend_from_slice(&789u32.to_le_bytes());
        data.extend_from_slice(&1011u16.to_le_bytes());
        data.push(1); // remote flag (1 = remote)
        data.extend_from_slice(&[0u8; 3]); // reserved bytes

        let sources = read_index_table(&data, 0, 2).unwrap();
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].id(), 123);
        assert_eq!(sources[0].version(), 456);
        assert_eq!(sources[0].is_remote(), false);
        assert_eq!(sources[1].id(), 789);
        assert_eq!(sources[1].version(), 1011);
        assert_eq!(sources[1].is_remote(), true);
    }

    #[test]
    fn test_read_bucket_data() {
        // Create simple bucket data
        let mut bucket_data = Vec::new();

        // First mapping
        let path1 = b"dir/file.txt";
        bucket_data.push(path1.len() as u8); // KEY_LEN
        bucket_data.push(0); // FORWARD_TYPE (Latest)
        bucket_data.push(0); // FORWARD_INFO_LEN
        bucket_data.extend_from_slice(path1); // KEY_BYTES
        bucket_data.extend_from_slice(&0u32.to_le_bytes()); // INDEX_OFFSET

        // Second mapping
        let path2 = b"other/test.txt";
        bucket_data.push(path2.len() as u8); // KEY_LEN
        bucket_data.push(0); // FORWARD_TYPE (Latest)
        bucket_data.push(0); // FORWARD_INFO_LEN
        bucket_data.extend_from_slice(path2); // KEY_BYTES
        bucket_data.extend_from_slice(&1u32.to_le_bytes()); // INDEX_OFFSET

        let index_sources = vec![IndexSource::new_local(1, 1), IndexSource::new_local(2, 1)];

        let mappings = read_bucket_data(&bucket_data, &index_sources).unwrap();
        assert_eq!(mappings.len(), 2);

        // Verify first mapping
        assert_eq!(
            mappings[0].value(),
            &["dir".to_string(), "file.txt".to_string()]
        );
        assert_eq!(mappings[0].index_source().id(), 1);

        // Verify second mapping
        assert_eq!(
            mappings[1].value(),
            &["other".to_string(), "test.txt".to_string()]
        );
        assert_eq!(mappings[1].index_source().id(), 2);
    }

    #[test]
    fn test_binary_search_bucket_lookup() {
        use crate::sheet::writer::convert_sheet_data_to_bytes;

        // Create test sheet data with multiple buckets
        let mut sheet_data = crate::sheet::SheetData::empty();

        // Add mappings that will go to different buckets
        let mapping1 = crate::mapping::LocalMapping::new(
            vec!["aaa".to_string(), "file1.txt".to_string()],
            crate::index_source::IndexSource::new_local(1, 1),
            crate::mapping::LocalMappingForward::Latest,
        )
        .unwrap();

        let mapping2 = crate::mapping::LocalMapping::new(
            vec!["mmm".to_string(), "file2.txt".to_string()],
            crate::index_source::IndexSource::new_local(2, 2),
            crate::mapping::LocalMappingForward::Latest,
        )
        .unwrap();

        let mapping3 = crate::mapping::LocalMapping::new(
            vec!["zzz".to_string(), "file3.txt".to_string()],
            crate::index_source::IndexSource::new_local(3, 3),
            crate::mapping::LocalMappingForward::Latest,
        )
        .unwrap();

        sheet_data.mappings.insert(mapping1.clone());
        sheet_data.mappings.insert(mapping2.clone());
        sheet_data.mappings.insert(mapping3.clone());

        // Convert to bytes
        let bytes = convert_sheet_data_to_bytes(sheet_data);

        // Test finding each mapping using binary search
        let node1 = &["aaa", "file1.txt"];
        let result1 = read_mapping(&bytes, node1).unwrap();
        assert!(result1.is_some(), "Should find mapping for aaa/file1.txt");

        let node2 = &["mmm", "file2.txt"];
        let result2 = read_mapping(&bytes, node2).unwrap();
        assert!(result2.is_some(), "Should find mapping for mmm/file2.txt");

        let node3 = &["zzz", "file3.txt"];
        let result3 = read_mapping(&bytes, node3).unwrap();
        assert!(result3.is_some(), "Should find mapping for zzz/file3.txt");

        // Test non-existent mapping
        let node4 = &["xxx", "notfound.txt"];
        let result4 = read_mapping(&bytes, node4).unwrap();
        assert!(result4.is_none(), "Should not find non-existent mapping");

        // Test that binary search handles empty data
        let empty_bytes = convert_sheet_data_to_bytes(crate::sheet::SheetData::empty());
        let result5 = read_mapping(&empty_bytes, node1).unwrap();
        assert!(result5.is_none(), "Should not find anything in empty sheet");
    }
}
