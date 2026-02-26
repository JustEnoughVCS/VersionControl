use std::path::PathBuf;

use crate::{error::StorageIOError, store::ChunkingResult};

/// Rabin fingerprint parameters for CDC
const WINDOW_SIZE: usize = 48;
const MIN_CHUNK_RATIO: f64 = 0.75;
const MAX_CHUNK_RATIO: f64 = 2.0;
const BASE_TARGET_MASK: u64 = 0xFFFF;

/// Rabin fingerprint polynomial (irreducible polynomial)
const POLYNOMIAL: u64 = 0x3DA3358B4DC173;

/// Precomputed table for Rabin fingerprint sliding window
static FINGERPRINT_TABLE: [u64; 256] = {
    let mut table = [0u64; 256];
    let mut i = 0;
    while i < 256 {
        let mut fingerprint = i as u64;
        let mut j = 0;
        while j < 8 {
            if fingerprint & 1 != 0 {
                fingerprint = (fingerprint >> 1) ^ POLYNOMIAL;
            } else {
                fingerprint >>= 1;
            }
            j += 1;
        }
        table[i] = fingerprint;
        i += 1;
    }
    table
};

/// Calculate Rabin fingerprint for a byte
#[inline]
fn rabin_fingerprint_byte(old_fingerprint: u64, old_byte: u8, new_byte: u8) -> u64 {
    let old_byte_index = old_byte as usize;

    let fingerprint =
        (old_fingerprint << 8) ^ (new_byte as u64) ^ FINGERPRINT_TABLE[old_byte_index];
    fingerprint
}

/// Split data using Content-Defined Chunking with Rabin fingerprinting
pub fn split_cdc_impl(data: &[u8], avg_size: u32) -> Result<ChunkingResult, StorageIOError> {
    let avg_size = avg_size as usize;

    if avg_size == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Average chunk size must be greater than 0",
        )
        .into());
    }

    // Calculate min and max chunk sizes based on average size
    let min_chunk_size = (avg_size as f64 * MIN_CHUNK_RATIO) as usize;
    let max_chunk_size = (avg_size as f64 * MAX_CHUNK_RATIO) as usize;

    // Ensure reasonable bounds
    let min_chunk_size = min_chunk_size.max(512); // At least 512 bytes
    let max_chunk_size = max_chunk_size.max(avg_size * 2);

    let target_mask = {
        let scale_factor = avg_size as f64 / (64.0 * 1024.0);
        let mask_value = (BASE_TARGET_MASK as f64 * scale_factor) as u64;
        mask_value.max(0xC000)
    };

    let mut chunks = Vec::new();
    let mut start = 0;
    let data_len = data.len();

    while start < data_len {
        let chunk_start = start;

        let search_end = (start + max_chunk_size).min(data_len);
        let mut boundary_found = false;
        let mut boundary_pos = search_end;

        let mut window = Vec::with_capacity(WINDOW_SIZE);
        let mut fingerprint: u64 = 0;

        for i in start..search_end {
            let byte = data[i];

            // Update sliding window
            if window.len() >= WINDOW_SIZE {
                let old_byte = window.remove(0);
                fingerprint = rabin_fingerprint_byte(fingerprint, old_byte, byte);
            } else {
                fingerprint = (fingerprint << 8) ^ (byte as u64);
            }
            window.push(byte);

            if i - chunk_start >= min_chunk_size {
                if (fingerprint & target_mask) == 0 {
                    boundary_found = true;
                    boundary_pos = i + 1;
                    break;
                }
            }
        }

        let chunk_end = if boundary_found {
            boundary_pos
        } else {
            search_end
        };

        let chunk_data = data[chunk_start..chunk_end].to_vec();
        let chunk = crate::store::create_chunk(chunk_data);
        chunks.push(chunk);

        start = chunk_end;
    }

    if chunks.len() > 1 {
        let mut merged_chunks = Vec::new();
        let mut i = 0;

        while i < chunks.len() {
            let current_chunk = &chunks[i];

            if i < chunks.len() - 1 {
                let next_chunk = &chunks[i + 1];
                if current_chunk.data.len() < min_chunk_size
                    && current_chunk.data.len() + next_chunk.data.len() <= max_chunk_size
                {
                    // Merge chunks
                    let mut merged_data = current_chunk.data.clone();
                    merged_data.extend_from_slice(&next_chunk.data);
                    let merged_chunk = crate::store::create_chunk(merged_data);
                    merged_chunks.push(merged_chunk);
                    i += 2;
                } else {
                    merged_chunks.push(current_chunk.clone());
                    i += 1;
                }
            } else {
                merged_chunks.push(current_chunk.clone());
                i += 1;
            }
        }

        chunks = merged_chunks;
    }

    Ok(ChunkingResult {
        chunks,
        total_size: data_len as u64,
    })
}

/// Split file using CDC with specified average chunk size
pub async fn write_file_cdc<I: Into<PathBuf>>(
    file_to_write: I,
    storage_dir: I,
    output_index_file: I,
    avg_size: u32,
) -> Result<(), StorageIOError> {
    use crate::store::{StorageConfig, write_file};

    let config = StorageConfig::cdc(avg_size);
    write_file(file_to_write, storage_dir, output_index_file, &config).await
}

/// Test function for CDC chunking
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cdc_basic() {
        let pattern: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();
        let mut data = Vec::new();
        for _ in 0..10 {
            data.extend_from_slice(&pattern);
        }

        let result = split_cdc_impl(&data, 4096).unwrap();

        // Should have at least one chunk
        assert!(!result.chunks.is_empty());

        assert!(
            result.chunks.len() >= 1 && result.chunks.len() <= 10,
            "Expected 1-10 chunks for 10KB data with 4KB average, got {}",
            result.chunks.len()
        );

        // Verify total size
        let total_chunk_size: usize = result.chunks.iter().map(|c| c.data.len()).sum();
        assert_eq!(total_chunk_size, data.len());

        // Verify chunk size bounds (more relaxed for CDC)
        let max_size = (4096.0 * MAX_CHUNK_RATIO) as usize;

        for chunk in &result.chunks {
            let chunk_size = chunk.data.len();
            // CDC can produce chunks smaller than min_size due to content patterns
            // But they should not exceed max_size
            assert!(
                chunk_size <= max_size,
                "Chunk size {} exceeds maximum {}",
                chunk_size,
                max_size
            );
        }
    }

    #[test]
    fn test_cdc_small_file() {
        // Small file should result in single chunk
        let data = vec![1, 2, 3, 4, 5];

        let result = split_cdc_impl(&data, 4096).unwrap();

        // Should have exactly one chunk
        assert_eq!(result.chunks.len(), 1);
        assert_eq!(result.chunks[0].data.len(), data.len());
    }

    #[test]
    fn test_cdc_large_avg_size() {
        // Test with larger average size (256KB)
        let mut data = Vec::new();
        for i in 0..(256 * 1024 * 3) {
            // 768KB of data
            data.push((i % 256) as u8);
        }

        let result = split_cdc_impl(&data, 256 * 1024).unwrap();

        // With 768KB data and 256KB average, should have reasonable number of chunks
        // CDC is content-defined, so exact count varies
        assert!(
            result.chunks.len() >= 1 && result.chunks.len() <= 10,
            "Expected 1-10 chunks for 768KB data with 256KB average, got {}",
            result.chunks.len()
        );

        // Verify total size
        let total_chunk_size: usize = result.chunks.iter().map(|c| c.data.len()).sum();
        assert_eq!(total_chunk_size, data.len());

        // Verify chunk size bounds
        let max_size = ((256 * 1024) as f64 * MAX_CHUNK_RATIO) as usize;

        for chunk in &result.chunks {
            // CDC can produce chunks smaller than min_size
            // But they should not exceed max_size
            assert!(
                chunk.data.len() <= max_size,
                "Chunk size {} exceeds maximum {}",
                chunk.data.len(),
                max_size
            );
        }
    }

    #[test]
    fn test_cdc_zero_avg_size() {
        let data = vec![1, 2, 3];

        let result = split_cdc_impl(&data, 0);
        assert!(result.is_err());

        match result {
            Err(StorageIOError::IOErr(e)) => {
                assert_eq!(e.kind(), std::io::ErrorKind::InvalidInput);
            }
            _ => panic!("Expected IOErr with InvalidInput"),
        }
    }

    #[test]
    fn test_rabin_fingerprint() {
        // Test that rabin_fingerprint_byte function is deterministic
        let test_bytes = [0x01, 0x02, 0x03, 0x04];

        // Calculate fingerprint with specific sequence
        let mut fingerprint1 = 0;
        for &byte in &test_bytes {
            fingerprint1 = rabin_fingerprint_byte(fingerprint1, 0xAA, byte);
        }

        // Calculate again with same inputs
        let mut fingerprint2 = 0;
        for &byte in &test_bytes {
            fingerprint2 = rabin_fingerprint_byte(fingerprint2, 0xAA, byte);
        }

        // Same input should produce same output
        assert_eq!(fingerprint1, fingerprint2);

        // Test with different old_byte produces different result
        let mut fingerprint3 = 0;
        for &byte in &test_bytes {
            fingerprint3 = rabin_fingerprint_byte(fingerprint3, 0xBB, byte);
        }

        // Different old_byte should produce different fingerprint
        assert_ne!(fingerprint1, fingerprint3);
    }
}
