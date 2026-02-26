use std::path::PathBuf;
use std::time::Instant;

use log::{info, trace};
use memmap2::Mmap;
use tokio::fs;
use tokio::task;

use crate::{
    error::StorageIOError,
    store::{ChunkingResult, IndexEntry, StorageConfig, create_chunk, get_dir, precheck},
};

/// Split data using fixed-size chunking
pub fn split_fixed_impl(data: &[u8], chunk_size: u32) -> Result<ChunkingResult, StorageIOError> {
    let chunk_size = chunk_size as usize;

    if chunk_size == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Chunk size must be greater than 0",
        )
        .into());
    }

    let mut chunks = Vec::new();
    let mut start = 0;
    let total_size = data.len();

    while start < total_size {
        let end = (start + chunk_size).min(total_size);
        let chunk_data = data[start..end].to_vec();

        let chunk = crate::store::create_chunk(chunk_data);
        chunks.push(chunk);

        start = end;
    }

    Ok(ChunkingResult {
        chunks,
        total_size: total_size as u64,
    })
}

/// Split file using fixed-size chunking
pub async fn write_file_fixed<I: Into<PathBuf>>(
    file_to_write: I,
    storage_dir: I,
    output_index_file: I,
    fixed_size: u32,
) -> Result<(), StorageIOError> {
    let config = StorageConfig::fixed_size(fixed_size);
    write_file_parallel(file_to_write, storage_dir, output_index_file, &config).await
}

/// Split file using fixed-size chunking with parallel processing
async fn write_file_parallel(
    file_to_write: impl Into<PathBuf>,
    storage_dir: impl Into<PathBuf>,
    output_index_file: impl Into<PathBuf>,
    cfg: &StorageConfig,
) -> Result<(), StorageIOError> {
    let (file_to_write, storage_dir, output_index_file) =
        precheck(file_to_write, storage_dir, output_index_file).await?;

    info!("Starting file write: {}", file_to_write.display());
    let start_time = Instant::now();

    // Memory map the entire file
    let file = std::fs::File::open(&file_to_write)?;
    let mmap = unsafe { Mmap::map(&file)? };
    let data = &mmap[..];

    // Split into chunks based on policy
    let chunking_result = split_into_chunks_parallel(&data, &cfg.chunking_policy).await?;

    // Store chunks in parallel and create index
    let index_entries = store_chunks_parallel(&chunking_result.chunks, &storage_dir).await?;

    // Write index file
    write_index_file(&index_entries, &output_index_file).await?;

    let duration = start_time.elapsed();
    info!(
        "File write completed in {:?}: {}",
        duration,
        file_to_write.display()
    );

    Ok(())
}

/// Split data into chunks based on the specified policy with parallel processing
async fn split_into_chunks_parallel(
    data: &[u8],
    policy: &crate::store::ChunkingPolicy,
) -> Result<ChunkingResult, StorageIOError> {
    match policy {
        crate::store::ChunkingPolicy::FixedSize(chunk_size) => {
            split_fixed_parallel(data, *chunk_size).await
        }
        _ => split_fixed_impl(data, 64 * 1024), // Fallback for non-fixed chunking
    }
}

/// Split data using fixed-size chunking with parallel processing
async fn split_fixed_parallel(
    data: &[u8],
    chunk_size: u32,
) -> Result<ChunkingResult, StorageIOError> {
    let chunk_size = chunk_size as usize;

    if chunk_size == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Chunk size must be greater than 0",
        )
        .into());
    }

    let total_size = data.len();
    let num_chunks = (total_size + chunk_size - 1) / chunk_size; // Ceiling division

    // Create a vector to hold chunk boundaries
    let mut chunk_boundaries = Vec::with_capacity(num_chunks);
    let mut start = 0;

    while start < total_size {
        let end = (start + chunk_size).min(total_size);
        chunk_boundaries.push((start, end));
        start = end;
    }

    // Process chunks in parallel using Tokio tasks
    let chunks: Vec<crate::store::Chunk> = if chunk_boundaries.len() > 1 {
        // Use parallel processing for multiple chunks
        let mut tasks = Vec::with_capacity(chunk_boundaries.len());

        for (start, end) in chunk_boundaries {
            let chunk_data = data[start..end].to_vec();

            // Spawn a blocking task for each chunk
            tasks.push(task::spawn_blocking(move || create_chunk(chunk_data)));
        }

        // Wait for all tasks to complete
        let mut chunks = Vec::with_capacity(tasks.len());
        for task in tasks {
            match task.await {
                Ok(chunk) => chunks.push(chunk),
                Err(e) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Task join error: {}", e),
                    )
                    .into());
                }
            }
        }

        chunks
    } else {
        // Single chunk, no need for parallel processing
        chunk_boundaries
            .into_iter()
            .map(|(start, end)| {
                let chunk_data = data[start..end].to_vec();
                create_chunk(chunk_data)
            })
            .collect()
    };

    Ok(ChunkingResult {
        chunks,
        total_size: total_size as u64,
    })
}

/// Store chunks in the storage directory in parallel and return index entries
async fn store_chunks_parallel(
    chunks: &[crate::store::Chunk],
    storage_dir: &std::path::Path,
) -> Result<Vec<IndexEntry>, StorageIOError> {
    let mut tasks = Vec::with_capacity(chunks.len());

    let writed_counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let total_chunks = chunks.len();

    for chunk in chunks {
        let chunk = chunk.clone();
        let storage_dir = storage_dir.to_path_buf();
        let writed_counter = writed_counter.clone();

        // Spawn async task for each chunk storage operation
        tasks.push(task::spawn(async move {
            // Create storage directory structure based on hash
            let hash_str = hex::encode(chunk.hash);
            let chunk_dir = get_dir(storage_dir, hash_str.clone())?;

            // Create directory if it doesn't exist
            if let Some(parent) = chunk_dir.parent() {
                fs::create_dir_all(parent).await?;
            }

            // Write chunk data
            let chunk_path = chunk_dir.with_extension("chunk");
            if !chunk_path.exists() {
                trace!("W: {}", hash_str);
                fs::write(&chunk_path, &chunk.data).await?;
                writed_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }

            Ok::<IndexEntry, StorageIOError>(IndexEntry {
                hash: chunk.hash,
                size: chunk.data.len() as u32,
            })
        }));
    }

    let writed_count = writed_counter.load(std::sync::atomic::Ordering::Relaxed);
    info!(
        "Chunk storage completed: {}/{} ({}%) chunks written ({} duplicates skipped)",
        writed_count,
        total_chunks,
        (writed_count as f32 / total_chunks as f32) * 100 as f32,
        total_chunks - writed_count
    );

    // Wait for all tasks to complete
    let mut index_entries = Vec::with_capacity(chunks.len());
    for task in tasks {
        let entry = task.await.map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("Task join error: {}", e))
        })??;
        index_entries.push(entry);
    }

    Ok(index_entries)
}

/// Write index file containing chunk hashes and sizes
async fn write_index_file(
    entries: &[IndexEntry],
    output_path: &std::path::Path,
) -> Result<(), StorageIOError> {
    let mut index_data = Vec::with_capacity(entries.len() * 36); // 32 bytes hash + 4 bytes size

    for entry in entries {
        index_data.extend_from_slice(&entry.hash);
        index_data.extend_from_slice(&entry.size.to_le_bytes());
    }

    fs::write(output_path, &index_data).await?;
    Ok(())
}

/// Utility function to calculate optimal fixed chunk size based on file size
pub fn calculate_optimal_chunk_size(file_size: u64, target_chunks: usize) -> u32 {
    if target_chunks == 0 || file_size == 0 {
        return 64 * 1024; // Default 64KB
    }

    let chunk_size = (file_size as f64 / target_chunks as f64).ceil() as u32;

    // Round to nearest power of 2 for better performance
    let rounded_size = if chunk_size <= 1024 {
        // Small chunks: use exact size
        chunk_size
    } else {
        // Larger chunks: round to nearest power of 2
        let mut size = chunk_size;
        size -= 1;
        size |= size >> 1;
        size |= size >> 2;
        size |= size >> 4;
        size |= size >> 8;
        size |= size >> 16;
        size += 1;
        size
    };

    // Ensure minimum and maximum bounds
    rounded_size.max(1024).min(16 * 1024 * 1024) // 1KB min, 16MB max
}

/// Split file with automatic chunk size calculation
pub async fn write_file_fixed_auto<I: Into<PathBuf>, J: Into<PathBuf>, K: Into<PathBuf>>(
    file_to_write: I,
    storage_dir: J,
    output_index_file: K,
    target_chunks: usize,
) -> Result<(), StorageIOError> {
    let file_path = file_to_write.into();
    let storage_dir = storage_dir.into();
    let output_index_file = output_index_file.into();

    let file_size = fs::metadata(&file_path).await?.len();

    let chunk_size = calculate_optimal_chunk_size(file_size, target_chunks);
    write_file_fixed(file_path, storage_dir, output_index_file, chunk_size).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_chunking_basic() {
        // Create 10KB of test data
        let data: Vec<u8> = (0..10240).map(|i| (i % 256) as u8).collect();

        // Split into 1KB chunks
        let result = split_fixed_impl(&data, 1024).unwrap();

        // Should have 10 chunks
        assert_eq!(result.chunks.len(), 10);

        // Verify total size
        let total_chunk_size: usize = result.chunks.iter().map(|c| c.data.len()).sum();
        assert_eq!(total_chunk_size, data.len());

        // Verify chunk sizes (last chunk may be smaller)
        for (i, chunk) in result.chunks.iter().enumerate() {
            if i < 9 {
                assert_eq!(chunk.data.len(), 1024);
            } else {
                assert_eq!(chunk.data.len(), 1024); // 10240 / 1024 = 10 exactly
            }
        }
    }

    #[test]
    fn test_fixed_chunking_uneven() {
        // Create 5.5KB of test data
        let data: Vec<u8> = (0..5632).map(|i| (i % 256) as u8).collect();

        // Split into 2KB chunks
        let result = split_fixed_impl(&data, 2048).unwrap();

        // Should have 3 chunks (2048 + 2048 + 1536)
        assert_eq!(result.chunks.len(), 3);

        // Verify chunk sizes
        assert_eq!(result.chunks[0].data.len(), 2048);
        assert_eq!(result.chunks[1].data.len(), 2048);
        assert_eq!(result.chunks[2].data.len(), 1536);

        // Verify data integrity
        let mut reconstructed = Vec::new();
        for chunk in &result.chunks {
            reconstructed.extend_from_slice(&chunk.data);
        }
        assert_eq!(reconstructed, data);
    }

    #[test]
    fn test_fixed_chunking_small_file() {
        // Small file smaller than chunk size
        let data = vec![1, 2, 3, 4, 5];

        let result = split_fixed_impl(&data, 1024).unwrap();

        // Should have exactly one chunk
        assert_eq!(result.chunks.len(), 1);
        assert_eq!(result.chunks[0].data.len(), data.len());
    }

    #[test]
    fn test_fixed_chunking_zero_size() {
        let data = vec![1, 2, 3];

        let result = split_fixed_impl(&data, 0);
        assert!(result.is_err());

        match result {
            Err(StorageIOError::IOErr(e)) => {
                assert_eq!(e.kind(), std::io::ErrorKind::InvalidInput);
            }
            _ => panic!("Expected IOErr with InvalidInput"),
        }
    }

    #[test]
    fn test_calculate_optimal_chunk_size() {
        // Test basic calculation
        assert_eq!(calculate_optimal_chunk_size(1024 * 1024, 16), 64 * 1024); // 1MB / 16 = 64KB

        // Test rounding to power of 2
        assert_eq!(calculate_optimal_chunk_size(1000 * 1000, 17), 64 * 1024); // ~58.8KB rounds to 64KB

        // Test minimum bound
        assert_eq!(calculate_optimal_chunk_size(100, 10), 1024); // 10 bytes per chunk, but min is 1KB

        // Test edge cases
        assert_eq!(calculate_optimal_chunk_size(0, 10), 64 * 1024); // Default
        assert_eq!(calculate_optimal_chunk_size(1000, 0), 64 * 1024); // Default

        // Test large file
        assert_eq!(
            calculate_optimal_chunk_size(100 * 1024 * 1024, 10),
            16 * 1024 * 1024
        ); // 100MB / 10 = 10MB, max is 16MB
    }

    #[test]
    fn test_chunk_hash_uniqueness() {
        // Test that different data produces different hashes
        let data1 = vec![1, 2, 3, 4, 5];
        let data2 = vec![1, 2, 3, 4, 6];

        let result1 = split_fixed_impl(&data1, 1024).unwrap();
        let result2 = split_fixed_impl(&data2, 1024).unwrap();

        assert_ne!(result1.chunks[0].hash, result2.chunks[0].hash);
    }
}
