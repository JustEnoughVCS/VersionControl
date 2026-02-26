use std::path::{Path, PathBuf};
use std::time::Instant;

use blake3;
use log::{info, trace};
use memmap2::Mmap;
use tokio::fs;

use crate::error::StorageIOError;

pub mod cdc;
pub mod fixed;
pub mod line;

#[derive(Default, Debug)]
pub struct StorageConfig {
    /// Chunking policy for splitting files
    pub chunking_policy: ChunkingPolicy,
}

impl StorageConfig {
    /// Create a new StorageConfig with CDC chunking policy
    pub fn cdc(avg_size: u32) -> Self {
        Self {
            chunking_policy: ChunkingPolicy::Cdc(avg_size),
        }
    }

    /// Create a new StorageConfig with fixed-size chunking policy
    pub fn fixed_size(chunk_size: u32) -> Self {
        Self {
            chunking_policy: ChunkingPolicy::FixedSize(chunk_size),
        }
    }

    /// Create a new StorageConfig with line-based chunking policy
    pub fn line() -> Self {
        Self {
            chunking_policy: ChunkingPolicy::Line,
        }
    }
}

#[derive(Debug)]
pub enum ChunkingPolicy {
    /// Content-Defined Chunking using Rabin fingerprinting
    /// The `u32` value represents the desired average chunk size in bytes
    Cdc(u32),

    /// Fixed-size chunking
    /// The `u32` value represents the exact size of each chunk in bytes
    FixedSize(u32),

    /// Line-based chunking
    /// Each line becomes a separate chunk
    Line,
}

impl Default for ChunkingPolicy {
    fn default() -> Self {
        ChunkingPolicy::Cdc(64 * 1024) // Default to 64KB CDC
    }
}

/// Represents a chunk of data with its hash and content
#[derive(Debug, Clone)]
pub struct Chunk {
    /// Blake3 hash of the chunk content
    pub hash: [u8; 32],
    /// Raw chunk data
    pub data: Vec<u8>,
}

/// Represents an index entry pointing to a chunk
#[derive(Debug, Clone)]
pub struct IndexEntry {
    /// Blake3 hash of the chunk
    pub hash: [u8; 32],
    /// Size of the chunk in bytes
    pub size: u32,
}

/// Result of chunking a file
#[derive(Debug)]
pub struct ChunkingResult {
    /// List of chunks extracted from the file
    pub chunks: Vec<Chunk>,
    /// Total size of the original file
    pub total_size: u64,
}

/// Split a file into chunks and store them in the repository, then output the index file to the specified directory
pub async fn write_file(
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
    let chunking_result = split_into_chunks(&data, &cfg.chunking_policy)?;

    // Store chunks and create index
    let index_entries = store_chunks(&chunking_result.chunks, &storage_dir).await?;

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

/// Split data into chunks based on the specified policy
pub fn split_into_chunks(
    data: &[u8],
    policy: &ChunkingPolicy,
) -> Result<ChunkingResult, StorageIOError> {
    match policy {
        ChunkingPolicy::Cdc(avg_size) => split_cdc(data, *avg_size),
        ChunkingPolicy::FixedSize(chunk_size) => split_fixed(data, *chunk_size),
        ChunkingPolicy::Line => split_by_lines(data),
    }
}

/// Split data using Content-Defined Chunking
fn split_cdc(data: &[u8], avg_size: u32) -> Result<ChunkingResult, StorageIOError> {
    use crate::store::cdc::split_cdc_impl;
    split_cdc_impl(data, avg_size)
}

/// Split data using fixed-size chunking
fn split_fixed(data: &[u8], chunk_size: u32) -> Result<ChunkingResult, StorageIOError> {
    use crate::store::fixed::split_fixed_impl;
    split_fixed_impl(data, chunk_size)
}

/// Split data by lines
fn split_by_lines(data: &[u8]) -> Result<ChunkingResult, StorageIOError> {
    use crate::store::line::split_by_lines_impl;
    split_by_lines_impl(data)
}

/// Store chunks in the storage directory and return index entries
async fn store_chunks(
    chunks: &[Chunk],
    storage_dir: &Path,
) -> Result<Vec<IndexEntry>, StorageIOError> {
    let mut index_entries = Vec::with_capacity(chunks.len());

    let writed_counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let total_chunks = chunks.len();

    for chunk in chunks {
        // Create storage directory structure based on hash
        let hash_str = hex::encode(chunk.hash);
        let chunk_dir = get_dir(storage_dir.to_path_buf(), hash_str.clone())?;

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

        // Add to index
        index_entries.push(IndexEntry {
            hash: chunk.hash,
            size: chunk.data.len() as u32,
        });
    }

    let writed_count = writed_counter.load(std::sync::atomic::Ordering::Relaxed);
    info!(
        "Chunk storage completed: {}/{} ({}%) chunks written ({} duplicates skipped)",
        writed_count,
        total_chunks,
        (writed_count as f32 / total_chunks as f32) * 100 as f32,
        total_chunks - writed_count
    );

    Ok(index_entries)
}

/// Write index file containing chunk hashes and sizes
async fn write_index_file(
    entries: &[IndexEntry],
    output_path: &Path,
) -> Result<(), StorageIOError> {
    let mut index_data = Vec::with_capacity(entries.len() * 36); // 32 bytes hash + 4 bytes size

    for entry in entries {
        index_data.extend_from_slice(&entry.hash);
        index_data.extend_from_slice(&entry.size.to_le_bytes());
    }

    fs::write(output_path, &index_data).await?;
    Ok(())
}

/// Build a file from the repository to the specified directory using an index file
pub async fn build_file(
    index_to_build: impl Into<PathBuf>,
    storage_dir: impl Into<PathBuf>,
    output_file: impl Into<PathBuf>,
) -> Result<(), StorageIOError> {
    let (index_to_build, storage_dir, output_file) =
        precheck(index_to_build, storage_dir, output_file).await?;

    info!(
        "Starting file build from index: {}",
        index_to_build.display()
    );
    let start_time = Instant::now();

    // Read index file
    let index_entries = read_index_file(&index_to_build).await?;

    // Reconstruct file from chunks
    let reconstructed_data = reconstruct_from_chunks(&index_entries, &storage_dir).await?;

    // Write output file
    fs::write(&output_file, &reconstructed_data).await?;

    let duration = start_time.elapsed();
    info!(
        "File build completed in {:?}: {} -> {}",
        duration,
        index_to_build.display(),
        output_file.display()
    );

    Ok(())
}

/// Read index file and parse entries
async fn read_index_file(index_path: &Path) -> Result<Vec<IndexEntry>, StorageIOError> {
    // Open file and memory map it
    let file = std::fs::File::open(index_path)?;
    let mmap = unsafe { Mmap::map(&file)? };

    // Each entry is 36 bytes (32 bytes hash + 4 bytes size)
    if mmap.len() % 36 != 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid index file format",
        )
        .into());
    }

    let num_entries = mmap.len() / 36;
    let mut entries = Vec::with_capacity(num_entries);

    for i in 0..num_entries {
        let start = i * 36;
        let hash_start = start;
        let size_start = start + 32;

        let mut hash = [0u8; 32];
        hash.copy_from_slice(&mmap[hash_start..hash_start + 32]);

        let size = u32::from_le_bytes([
            mmap[size_start],
            mmap[size_start + 1],
            mmap[size_start + 2],
            mmap[size_start + 3],
        ]);

        entries.push(IndexEntry { hash, size });
    }

    Ok(entries)
}

/// Reconstruct file data from chunks using index entries
async fn reconstruct_from_chunks(
    entries: &[IndexEntry],
    storage_dir: &Path,
) -> Result<Vec<u8>, StorageIOError> {
    let mut reconstructed_data = Vec::new();

    for entry in entries {
        // Get chunk path from hash
        let hash_str = hex::encode(entry.hash);
        let chunk_dir = get_dir(storage_dir.to_path_buf(), hash_str.clone())?;
        let chunk_path = chunk_dir.with_extension("chunk");

        trace!("R: {}", hash_str);

        // Memory map the chunk file
        let file = std::fs::File::open(&chunk_path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        // Verify chunk size matches index
        if mmap.len() != entry.size as usize {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Chunk size mismatch: expected {}, got {}",
                    entry.size,
                    mmap.len()
                ),
            )
            .into());
        }

        // Verify chunk hash
        let actual_hash = blake3_digest(&mmap);
        let expected_hash = entry.hash;
        if actual_hash != expected_hash {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Chunk hash mismatch: expected {}, got {}",
                    hex::encode(expected_hash),
                    hex::encode(actual_hash)
                ),
            )
            .into());
        }

        // Append to reconstructed data
        reconstructed_data.extend_from_slice(&mmap);
    }

    Ok(reconstructed_data)
}

/// Calculate Blake3 hash of data
fn blake3_digest(data: &[u8]) -> [u8; 32] {
    let hash = blake3::hash(data);
    *hash.as_bytes()
}

/// Calculate and return the corresponding storage subdirectory path based on the given storage directory and hash string
pub fn get_dir(storage_dir: PathBuf, hash_str: String) -> Result<PathBuf, StorageIOError> {
    if hash_str.len() < 4 {
        return Err(StorageIOError::HashTooShort);
    }
    let dir = storage_dir
        .join(&hash_str[0..2])
        .join(&hash_str[2..4])
        .join(&hash_str);
    Ok(dir)
}

/// Calculate Blake3 hash of data and create a Chunk
pub fn create_chunk(data: Vec<u8>) -> Chunk {
    let hash = blake3_digest(&data);
    Chunk { hash, data }
}

/// Read a file and split it into chunks using the specified policy
pub async fn chunk_file(
    file_path: impl Into<PathBuf>,
    policy: &ChunkingPolicy,
) -> Result<ChunkingResult, StorageIOError> {
    let file_path = file_path.into();
    info!("Starting chunking: {}", file_path.display());
    let start_time = Instant::now();

    // Memory map the file
    let file = std::fs::File::open(&file_path)?;
    let mmap = unsafe { Mmap::map(&file)? };
    let data = &mmap[..];

    let result = split_into_chunks(data, policy);

    let duration = start_time.elapsed();
    info!(
        "Chunking completed in {:?}: {}",
        duration,
        file_path.display()
    );

    result
}

/// Get chunk statistics
pub fn get_chunk_stats(chunks: &[Chunk]) -> ChunkStats {
    let total_chunks = chunks.len();
    let total_size: usize = chunks.iter().map(|c| c.data.len()).sum();
    let avg_size = if total_chunks > 0 {
        total_size / total_chunks
    } else {
        0
    };
    let min_size = chunks.iter().map(|c| c.data.len()).min().unwrap_or(0);
    let max_size = chunks.iter().map(|c| c.data.len()).max().unwrap_or(0);

    ChunkStats {
        total_chunks,
        total_size: total_size as u64,
        avg_size,
        min_size,
        max_size,
    }
}

/// Statistics about chunks
#[derive(Debug, Clone)]
pub struct ChunkStats {
    pub total_chunks: usize,
    pub total_size: u64,
    pub avg_size: usize,
    pub min_size: usize,
    pub max_size: usize,
}

/// Pre-check whether the input file path, directory path, and output path are valid
pub async fn precheck(
    input_file: impl Into<PathBuf>,
    dir: impl Into<PathBuf>,
    output_file: impl Into<PathBuf>,
) -> Result<(PathBuf, PathBuf, PathBuf), std::io::Error> {
    let (input, dir, output) = (input_file.into(), dir.into(), output_file.into());

    // Perform all checks in parallel
    let (input_metadata_result, dir_metadata_result, output_metadata_result) = tokio::join!(
        fs::metadata(&input),
        fs::metadata(&dir),
        fs::metadata(&output)
    );

    // Check if input file exists
    let input_metadata = match input_metadata_result {
        Ok(metadata) => metadata,
        Err(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Input file not found: {}", input.display()),
            ));
        }
    };

    // Check if directory exists
    let dir_metadata = match dir_metadata_result {
        Ok(metadata) => metadata,
        Err(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Directory not found: {}", dir.display()),
            ));
        }
    };

    // Check if output file already exists
    if output_metadata_result.is_ok() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("Output file already exist: {}", output.display()),
        ));
    }

    // Check if input is a file
    if !input_metadata.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Input path is not a file: {}", input.display()),
        ));
    }

    // Check if dir is a directory
    if !dir_metadata.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotADirectory,
            format!("Input path is not a directory: {}", dir.display()),
        ));
    }

    Ok((input, dir, output))
}
