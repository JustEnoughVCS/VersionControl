use hex_display::hex_display_slice;

use crate::{
    index_source::IndexSource,
    mapping::{LocalMapping, LocalMappingForward},
    sheet::{
        SheetData, reader::read_sheet_data, v1::constants::HEADER_SIZE,
        writer::convert_sheet_data_to_bytes,
    },
};
use std::collections::HashSet;
use std::fs;

#[cfg(test)]
mod tests {
    use super::*;

    /// Test writing and re-reading sheet data
    #[test]
    fn test_sheet_data_roundtrip() {
        // Create test data
        let _sheet_data = SheetData::empty();

        // Create some test mappings
        let mapping1 = LocalMapping::new(
            vec!["src".to_string(), "main.rs".to_string()],
            IndexSource::new_local(1001, 1),
            LocalMappingForward::Latest,
        )
        .unwrap();

        let mapping2 = LocalMapping::new(
            vec!["docs".to_string(), "README.md".to_string()],
            IndexSource::new_local(1002, 2),
            LocalMappingForward::Ref {
                sheet_name: "reference".to_string(),
            },
        )
        .unwrap();

        let mapping3 = LocalMapping::new(
            vec![
                "assets".to_string(),
                "images".to_string(),
                "logo.png".to_string(),
            ],
            IndexSource::new_local(1003, 3),
            LocalMappingForward::Version { version: 12345 },
        )
        .unwrap();

        // Add mappings to SheetData
        // Note: Since the mappings field of SheetData is private, we need to create SheetData in another way
        // Here we directly create a new HashSet
        let mut mappings = HashSet::new();
        mappings.insert(mapping1.clone());
        mappings.insert(mapping2.clone());
        mappings.insert(mapping3.clone());

        let sheet_data = SheetData { mappings };

        // Convert SheetData to bytes
        let bytes = convert_sheet_data_to_bytes(sheet_data.clone());

        // Verify byte data is not empty
        assert!(!bytes.is_empty(), "Converted bytes should not be empty");

        // Verify file header
        assert_eq!(bytes[0], 1, "Sheet version should be 1");

        // Re-read SheetData from bytes
        let restored_sheet_data =
            read_sheet_data(&bytes).expect("Failed to read sheet data from bytes");

        // Verify mapping count
        assert_eq!(
            restored_sheet_data.mappings.len(),
            sheet_data.mappings.len(),
            "Restored sheet should have same number of mappings"
        );

        // Verify each mapping exists
        for mapping in &sheet_data.mappings {
            assert!(
                restored_sheet_data.mappings.contains(mapping),
                "Restored sheet should contain mapping: {:?}",
                mapping
            );
        }

        // Verify specific mapping content
        for mapping in &restored_sheet_data.mappings {
            // Find original mapping
            let original_mapping = sheet_data.mappings.get(mapping.value()).unwrap();

            // Verify path
            assert_eq!(
                mapping.value(),
                original_mapping.value(),
                "Path should match"
            );

            // Verify index source
            assert_eq!(
                mapping.index_source().id(),
                original_mapping.index_source().id(),
                "Index source ID should match"
            );

            assert_eq!(
                mapping.index_source().version(),
                original_mapping.index_source().version(),
                "Index source version should match"
            );

            // Verify forward information
            let (original_type, _, _) = original_mapping.forward().unpack();
            let (restored_type, _, _) = mapping.forward().unpack();
            assert_eq!(restored_type, original_type, "Forward type should match");
        }
    }

    /// Test reading and writing empty sheet data
    #[test]
    fn test_empty_sheet_roundtrip() {
        // Create empty SheetData
        let sheet_data = SheetData::empty();

        // Convert to bytes
        let bytes = convert_sheet_data_to_bytes(sheet_data.clone());

        // Verify file header
        assert_eq!(bytes.len(), 15, "Empty sheet should have header size only");
        assert_eq!(bytes[0], 1, "Sheet version should be 1");

        // Verify offsets - For empty sheet, mapping data offset and index table offset should be the same
        let mapping_data_offset =
            u32::from_le_bytes([bytes[7], bytes[8], bytes[9], bytes[10]]) as usize;
        let index_table_offset =
            u32::from_le_bytes([bytes[11], bytes[12], bytes[13], bytes[14]]) as usize;
        assert_eq!(
            mapping_data_offset, index_table_offset,
            "For empty sheet, both offsets should be the same"
        );
        assert_eq!(
            mapping_data_offset, HEADER_SIZE,
            "Offsets should point to end of header"
        );

        // Mapping count should be 0
        let mapping_count = u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
        assert_eq!(mapping_count, 0, "Mapping count should be 0");

        // Index source count should be 0
        let index_count = u16::from_le_bytes([bytes[5], bytes[6]]);
        assert_eq!(index_count, 0, "Index count should be 0");

        // Re-read
        let restored_sheet_data = read_sheet_data(&bytes).expect("Failed to read empty sheet data");

        // Verify it's empty
        assert!(
            restored_sheet_data.mappings.is_empty(),
            "Restored empty sheet should have no mappings"
        );
    }

    /// Test reading and writing a single mapping
    #[test]
    fn test_single_mapping_roundtrip() {
        // Create a single mapping
        let mapping = LocalMapping::new(
            vec!["test.txt".to_string()],
            IndexSource::new_local(999, 42),
            LocalMappingForward::Latest,
        )
        .unwrap();

        let mut mappings = HashSet::new();
        mappings.insert(mapping.clone());

        let sheet_data = SheetData { mappings };

        // Convert to bytes
        let bytes = convert_sheet_data_to_bytes(sheet_data.clone());

        // Re-read
        let restored_sheet_data = read_sheet_data(&bytes).expect("Failed to read sheet data");

        // Verify
        assert_eq!(restored_sheet_data.mappings.len(), 1);
        let restored_mapping = restored_sheet_data.mappings.iter().next().unwrap();

        assert_eq!(restored_mapping.value(), &["test.txt".to_string()]);
        assert_eq!(restored_mapping.index_source().id(), 999);
        assert_eq!(restored_mapping.index_source().version(), 42);

        let (forward_type, _, _) = restored_mapping.forward().unpack();
        assert_eq!(forward_type, 0); // Latest type id is 0
    }

    /// Test file system read/write
    #[test]
    fn test_file_system_roundtrip() {
        // Create test data
        let mapping1 = LocalMapping::new(
            vec!["file0.txt".to_string()],
            IndexSource::new_local(1, 1),
            LocalMappingForward::Latest,
        )
        .unwrap();

        let mapping2 = LocalMapping::new(
            vec!["dir1".to_string(), "file1.txt".to_string()],
            IndexSource::new_local(2, 2),
            LocalMappingForward::Ref {
                sheet_name: "other".to_string(),
            },
        )
        .unwrap();

        let mapping3 = LocalMapping::new(
            vec!["dir2".to_string(), "file2.txt".to_string()],
            IndexSource::new_local(3, 3),
            LocalMappingForward::Version { version: 35 },
        )
        .unwrap();

        let mut mappings = HashSet::new();
        mappings.insert(mapping1.clone());
        mappings.insert(mapping2.clone());
        mappings.insert(mapping3.clone());

        let sheet_data = SheetData { mappings };

        // Convert to bytes
        let bytes = convert_sheet_data_to_bytes(sheet_data.clone());

        // Write to file
        let test_file_path = ".temp/test.sheet";
        let test_file_path_hex = ".temp/test_hex.txt";

        // Ensure directory exists
        if let Some(parent) = std::path::Path::new(test_file_path).parent() {
            fs::create_dir_all(parent).expect("Failed to create test directory");
        }

        fs::write(test_file_path, &bytes).expect("Failed to write test file");
        fs::write(test_file_path_hex, hex_display_slice(&bytes))
            .expect("Failed to write test file");

        // Read file
        let file_bytes = fs::read(test_file_path).expect("Failed to read test file");

        // Verify file content matches original bytes
        assert_eq!(
            file_bytes, bytes,
            "File content should match original bytes"
        );

        // Re-read SheetData from file bytes
        let restored_from_file =
            read_sheet_data(&file_bytes).expect("Failed to read from file bytes");

        // Use SheetData's Eq trait for direct comparison
        assert_eq!(
            restored_from_file, sheet_data,
            "Restored sheet data should be equal to original"
        );

        // Verify mappings in SheetData read from file
        // Check if each original mapping can be found in restored data
        for original_mapping in &sheet_data.mappings {
            let found = restored_from_file
                .mappings
                .iter()
                .any(|m| m == original_mapping);
            assert!(
                found,
                "Original mapping {:?} should be present in restored sheet data",
                original_mapping
            );
        }

        // Also check if each mapping in restored data can be found in original data
        for restored_mapping in &restored_from_file.mappings {
            let found = sheet_data.mappings.iter().any(|m| m == restored_mapping);
            assert!(
                found,
                "Restored mapping {:?} should be present in original sheet data",
                restored_mapping
            );
        }

        // Test file remains in .temp/test.sheet for subsequent inspection
        // Note: Need to manually clean up .temp directory before next test run
    }

    /// Test reading and writing different forward types
    #[test]
    fn test_different_forward_types() {
        // Test Latest type
        let mapping_latest = LocalMapping::new(
            vec!["latest.txt".to_string()],
            IndexSource::new_local(1, 1),
            LocalMappingForward::Latest,
        )
        .unwrap();

        // Test Ref type
        let mapping_ref = LocalMapping::new(
            vec!["ref.txt".to_string()],
            IndexSource::new_local(2, 2),
            LocalMappingForward::Ref {
                sheet_name: "reference_sheet".to_string(),
            },
        )
        .unwrap();

        // Test Version type
        let mapping_version = LocalMapping::new(
            vec!["version.txt".to_string()],
            IndexSource::new_local(3, 3),
            LocalMappingForward::Version { version: 54321 },
        )
        .unwrap();

        let mut mappings = HashSet::new();
        mappings.insert(mapping_latest.clone());
        mappings.insert(mapping_ref.clone());
        mappings.insert(mapping_version.clone());

        let sheet_data = SheetData { mappings };

        // Convert to bytes and re-read
        let bytes = convert_sheet_data_to_bytes(sheet_data.clone());
        let restored_sheet_data = read_sheet_data(&bytes).expect("Failed to read sheet data");

        // Verify all mappings exist
        assert_eq!(restored_sheet_data.mappings.len(), 3);

        // Verify Latest type
        let restored_latest = restored_sheet_data
            .mappings
            .get(&vec!["latest.txt".to_string()])
            .unwrap();
        let (latest_type, latest_len, _) = restored_latest.forward().unpack();
        assert_eq!(latest_type, 0);
        assert_eq!(latest_len, 0);

        // Verify Ref type
        let restored_ref = restored_sheet_data
            .mappings
            .get(&vec!["ref.txt".to_string()])
            .unwrap();
        let (ref_type, ref_len, ref_bytes) = restored_ref.forward().unpack();
        assert_eq!(ref_type, 1);
        assert_eq!(ref_len as usize, "reference_sheet".len());
        assert_eq!(String::from_utf8(ref_bytes).unwrap(), "reference_sheet");

        // Verify Version type
        let restored_version = restored_sheet_data
            .mappings
            .get(&vec!["version.txt".to_string()])
            .unwrap();
        let (version_type, version_len, version_bytes) = restored_version.forward().unpack();
        assert_eq!(version_type, 2);
        assert_eq!(version_len, 2); // u16 is 2 bytes
        assert_eq!(u16::from_be_bytes(version_bytes.try_into().unwrap()), 54321);
    }

    /// Test duplicate index source optimization
    #[test]
    fn test_duplicate_index_source_optimization() {
        // Create multiple mappings sharing the same index source
        let shared_source = IndexSource::new_local(777, 88);

        let mapping1 = LocalMapping::new(
            vec!["file1.txt".to_string()],
            shared_source,
            LocalMappingForward::Latest,
        )
        .unwrap();

        let mapping2 = LocalMapping::new(
            vec!["file2.txt".to_string()],
            shared_source,
            LocalMappingForward::Latest,
        )
        .unwrap();

        let mapping3 = LocalMapping::new(
            vec!["file3.txt".to_string()],
            shared_source,
            LocalMappingForward::Latest,
        )
        .unwrap();

        let mut mappings = HashSet::new();
        mappings.insert(mapping1);
        mappings.insert(mapping2);
        mappings.insert(mapping3);

        let sheet_data = SheetData { mappings };

        // Convert to bytes
        let bytes = convert_sheet_data_to_bytes(sheet_data.clone());

        // Verify index table should have only one entry
        let index_count = u32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]);
        assert_eq!(index_count, 1, "Should have only one unique index source");

        // Re-read and verify
        let restored_sheet_data = read_sheet_data(&bytes).expect("Failed to read sheet data");
        assert_eq!(restored_sheet_data.mappings.len(), 3);

        // Verify all mappings use the same index source
        for mapping in &restored_sheet_data.mappings {
            assert_eq!(mapping.index_source().id(), 777);
            assert_eq!(mapping.index_source().version(), 88);
        }
    }

    /// Test path serialization and deserialization
    #[test]
    fn test_path_serialization_deserialization() {
        // Test various paths
        let test_cases = vec![
            vec!["single".to_string()],
            vec!["dir".to_string(), "file.txt".to_string()],
            vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d.txt".to_string(),
            ],
            vec!["with spaces".to_string(), "file name.txt".to_string()],
            vec!["unicode".to_string(), "文件.txt".to_string()],
        ];

        for path in test_cases {
            let mapping = LocalMapping::new(
                path.clone(),
                IndexSource::new_local(1, 1),
                LocalMappingForward::Latest,
            )
            .unwrap();

            let mut mappings = HashSet::new();
            mappings.insert(mapping);

            let sheet_data = SheetData { mappings };

            // Convert to bytes and re-read
            let bytes = convert_sheet_data_to_bytes(sheet_data.clone());
            let restored_sheet_data = read_sheet_data(&bytes).expect("Failed to read sheet data");

            // Verify path
            let restored_mapping = restored_sheet_data.mappings.iter().next().unwrap();
            assert_eq!(
                restored_mapping.value(),
                &path,
                "Path should be preserved after roundtrip"
            );
        }
    }

    /// Test mixed local and remote index sources
    #[test]
    fn test_mixed_local_remote_index_sources() {
        // Create mappings with mixed local and remote index sources
        let mapping_local1 = LocalMapping::new(
            vec!["local1.txt".to_string()],
            IndexSource::new_local(100, 1),
            LocalMappingForward::Latest,
        )
        .unwrap();

        let mapping_local2 = LocalMapping::new(
            vec!["local2.txt".to_string()],
            IndexSource::new_local(200, 2),
            LocalMappingForward::Ref {
                sheet_name: "ref_sheet".to_string(),
            },
        )
        .unwrap();

        let mapping_remote1 = LocalMapping::new(
            vec!["remote1.txt".to_string()],
            IndexSource::new_remote(300, 3),
            LocalMappingForward::Latest,
        )
        .unwrap();

        let mapping_remote2 = LocalMapping::new(
            vec!["remote2.txt".to_string()],
            IndexSource::new_remote(400, 4),
            LocalMappingForward::Version { version: 12345 },
        )
        .unwrap();

        // Test same ID but different remote status
        let mapping_same_id_local = LocalMapping::new(
            vec!["same_id_local.txt".to_string()],
            IndexSource::new_local(500, 5),
            LocalMappingForward::Latest,
        )
        .unwrap();

        let mapping_same_id_remote = LocalMapping::new(
            vec!["same_id_remote.txt".to_string()],
            IndexSource::new_remote(500, 5),
            LocalMappingForward::Latest,
        )
        .unwrap();

        let mut mappings = HashSet::new();
        mappings.insert(mapping_local1.clone());
        mappings.insert(mapping_local2.clone());
        mappings.insert(mapping_remote1.clone());
        mappings.insert(mapping_remote2.clone());
        mappings.insert(mapping_same_id_local.clone());
        mappings.insert(mapping_same_id_remote.clone());

        let sheet_data = SheetData { mappings };

        // Convert to bytes
        let bytes = convert_sheet_data_to_bytes(sheet_data.clone());

        // Re-read from bytes
        let restored_sheet_data = read_sheet_data(&bytes).expect("Failed to read sheet data");

        // Verify all mappings exist
        assert_eq!(
            restored_sheet_data.mappings.len(),
            6,
            "Should have all 6 mappings"
        );

        // Verify local mappings
        let restored_local1 = restored_sheet_data
            .mappings
            .get(&vec!["local1.txt".to_string()])
            .unwrap();
        assert_eq!(restored_local1.index_source().id(), 100);
        assert_eq!(restored_local1.index_source().version(), 1);
        assert_eq!(restored_local1.index_source().is_remote(), false);

        let restored_local2 = restored_sheet_data
            .mappings
            .get(&vec!["local2.txt".to_string()])
            .unwrap();
        assert_eq!(restored_local2.index_source().id(), 200);
        assert_eq!(restored_local2.index_source().version(), 2);
        assert_eq!(restored_local2.index_source().is_remote(), false);

        // Verify remote mappings
        let restored_remote1 = restored_sheet_data
            .mappings
            .get(&vec!["remote1.txt".to_string()])
            .unwrap();
        assert_eq!(restored_remote1.index_source().id(), 300);
        assert_eq!(restored_remote1.index_source().version(), 3);
        assert_eq!(restored_remote1.index_source().is_remote(), true);

        let restored_remote2 = restored_sheet_data
            .mappings
            .get(&vec!["remote2.txt".to_string()])
            .unwrap();
        assert_eq!(restored_remote2.index_source().id(), 400);
        assert_eq!(restored_remote2.index_source().version(), 4);
        assert_eq!(restored_remote2.index_source().is_remote(), true);

        // Verify same ID but different remote status are treated as different sources
        let restored_same_id_local = restored_sheet_data
            .mappings
            .get(&vec!["same_id_local.txt".to_string()])
            .unwrap();
        assert_eq!(restored_same_id_local.index_source().id(), 500);
        assert_eq!(restored_same_id_local.index_source().version(), 5);
        assert_eq!(restored_same_id_local.index_source().is_remote(), false);

        let restored_same_id_remote = restored_sheet_data
            .mappings
            .get(&vec!["same_id_remote.txt".to_string()])
            .unwrap();
        assert_eq!(restored_same_id_remote.index_source().id(), 500);
        assert_eq!(restored_same_id_remote.index_source().version(), 5);
        assert_eq!(restored_same_id_remote.index_source().is_remote(), true);

        // Verify that local and remote with same ID are different
        assert_ne!(
            restored_same_id_local.index_source(),
            restored_same_id_remote.index_source()
        );

        // Verify forward types are preserved
        let (forward_type_local2, forward_len_local2, forward_bytes_local2) =
            restored_local2.forward().unpack();
        assert_eq!(forward_type_local2, 1); // Ref type
        assert_eq!(forward_len_local2 as usize, "ref_sheet".len());
        assert_eq!(
            String::from_utf8(forward_bytes_local2).unwrap(),
            "ref_sheet"
        );

        let (forward_type_remote2, forward_len_remote2, forward_bytes_remote2) =
            restored_remote2.forward().unpack();
        assert_eq!(forward_type_remote2, 2); // Version type
        assert_eq!(forward_len_remote2, 2); // u16 is 2 bytes
        assert_eq!(
            u16::from_be_bytes(forward_bytes_remote2.try_into().unwrap()),
            12345
        );

        // Test duplicate index source optimization with remote flag
        // Should have 6 unique index sources (local1, local2, remote1, remote2, local500, remote500)
        let index_count = u32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]);
        assert_eq!(
            index_count, 6,
            "Should have 6 unique index sources (including remote flag)"
        );

        println!("Mixed local/remote test passed successfully!");
    }
}
