use std::path::PathBuf;

use crate::{error::StorageIOError, store::ChunkingResult};

/// Split data by lines (newline characters)
pub fn split_by_lines_impl(data: &[u8]) -> Result<ChunkingResult, StorageIOError> {
    let mut chunks = Vec::new();
    let mut start = 0;
    let total_size = data.len();

    // Iterate through data to find line boundaries
    let mut i = 0;
    while i < data.len() {
        if data[i] == b'\n' {
            // Unix line ending
            let line_end = i + 1; // Include \n
            // Extract line data (include newline character)
            let line_data = data[start..line_end].to_vec();

            // Create chunk for this line
            let chunk = crate::store::create_chunk(line_data);
            chunks.push(chunk);

            // Move start to next line
            start = line_end;
            i = line_end;
        } else if data[i] == b'\r' && i + 1 < data.len() && data[i + 1] == b'\n' {
            // Windows line ending
            let line_end = i + 2; // Include both \r and \n
            // Extract line data (include newline characters)
            let line_data = data[start..line_end].to_vec();

            // Create chunk for this line
            let chunk = crate::store::create_chunk(line_data);
            chunks.push(chunk);

            // Move start to next line
            start = line_end;
            i = line_end;
        } else {
            i += 1;
        }
    }

    // Handle remaining data (last line without newline)
    if start < total_size {
        let line_data = data[start..].to_vec();
        let chunk = crate::store::create_chunk(line_data);
        chunks.push(chunk);
    }

    // Handle empty file (no lines)
    if chunks.is_empty() && total_size == 0 {
        let chunk = crate::store::create_chunk(Vec::new());
        chunks.push(chunk);
    }

    Ok(ChunkingResult {
        chunks,
        total_size: total_size as u64,
    })
}

/// Split file by lines
pub async fn write_file_line<I: Into<PathBuf>>(
    file_to_write: I,
    storage_dir: I,
    output_index_file: I,
) -> Result<(), StorageIOError> {
    use crate::store::{StorageConfig, write_file};

    let config = StorageConfig::line();
    write_file(file_to_write, storage_dir, output_index_file, &config).await
}

/// Utility function to split data by lines with custom line ending detection
pub fn split_by_lines_custom<E: LineEnding>(data: &[u8]) -> Result<ChunkingResult, StorageIOError> {
    let mut chunks = Vec::new();
    let mut start = 0;
    let total_size = data.len();

    let mut i = 0;
    while i < total_size {
        if E::is_line_ending(data, i) {
            let line_end = i + E::ending_length(data, i);
            let line_data = data[start..line_end].to_vec();

            let chunk = crate::store::create_chunk(line_data);
            chunks.push(chunk);

            start = line_end;
            i = line_end;
        } else {
            i += 1;
        }
    }

    // Handle remaining data
    if start < total_size {
        let line_data = data[start..].to_vec();
        let chunk = crate::store::create_chunk(line_data);
        chunks.push(chunk);
    }

    // Handle empty file
    if chunks.is_empty() && total_size == 0 {
        let chunk = crate::store::create_chunk(Vec::new());
        chunks.push(chunk);
    }

    Ok(ChunkingResult {
        chunks,
        total_size: total_size as u64,
    })
}

/// Trait for different line ending types
pub trait LineEnding {
    /// Check if position i is the start of a line ending
    fn is_line_ending(data: &[u8], i: usize) -> bool;

    /// Get the length of the line ending at position i
    fn ending_length(data: &[u8], i: usize) -> usize;
}

/// Unix line endings (\n)
pub struct UnixLineEnding;

impl LineEnding for UnixLineEnding {
    fn is_line_ending(data: &[u8], i: usize) -> bool {
        i < data.len() && data[i] == b'\n'
    }

    fn ending_length(_data: &[u8], _i: usize) -> usize {
        1
    }
}

/// Windows line endings (\r\n)
pub struct WindowsLineEnding;

impl LineEnding for WindowsLineEnding {
    fn is_line_ending(data: &[u8], i: usize) -> bool {
        i + 1 < data.len() && data[i] == b'\r' && data[i + 1] == b'\n'
    }

    fn ending_length(_data: &[u8], _i: usize) -> usize {
        2
    }
}

/// Mixed line endings (detects both Unix and Windows)
pub struct MixedLineEnding;

impl LineEnding for MixedLineEnding {
    fn is_line_ending(data: &[u8], i: usize) -> bool {
        if i < data.len() && data[i] == b'\n' {
            true
        } else if i + 1 < data.len() && data[i] == b'\r' && data[i + 1] == b'\n' {
            true
        } else {
            false
        }
    }

    fn ending_length(data: &[u8], i: usize) -> usize {
        if i < data.len() && data[i] == b'\n' {
            1
        } else if i + 1 < data.len() && data[i] == b'\r' && data[i + 1] == b'\n' {
            2
        } else {
            1 // Default to 1 if somehow called incorrectly
        }
    }
}

/// Detect line ending type from data
pub fn detect_line_ending(data: &[u8]) -> LineEndingType {
    let mut unix_count = 0;
    let mut windows_count = 0;

    let mut i = 0;
    while i < data.len() {
        if data[i] == b'\n' {
            unix_count += 1;
            i += 1;
        } else if i + 1 < data.len() && data[i] == b'\r' && data[i + 1] == b'\n' {
            windows_count += 1;
            i += 2;
        } else {
            i += 1;
        }
    }

    if unix_count > windows_count {
        LineEndingType::Unix
    } else if windows_count > unix_count {
        LineEndingType::Windows
    } else {
        LineEndingType::Mixed
    }
}

/// Line ending type enum
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineEndingType {
    Unix,
    Windows,
    Mixed,
}

impl LineEndingType {
    /// Split data using the detected line ending type
    pub fn split_by_lines(&self, data: &[u8]) -> Result<ChunkingResult, StorageIOError> {
        match self {
            LineEndingType::Unix => split_by_lines_custom::<UnixLineEnding>(data),
            LineEndingType::Windows => split_by_lines_custom::<WindowsLineEnding>(data),
            LineEndingType::Mixed => split_by_lines_custom::<MixedLineEnding>(data),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_chunking_unix() {
        let data = b"Hello\nWorld\nTest\n";

        let result = split_by_lines_impl(data).unwrap();

        // Should have 3 chunks
        assert_eq!(result.chunks.len(), 3);

        // Verify chunk contents
        assert_eq!(result.chunks[0].data, b"Hello\n");
        assert_eq!(result.chunks[1].data, b"World\n");
        assert_eq!(result.chunks[2].data, b"Test\n");

        // Verify total size
        let total_chunk_size: usize = result.chunks.iter().map(|c| c.data.len()).sum();
        assert_eq!(total_chunk_size, data.len());
    }

    #[test]
    fn test_line_chunking_windows() {
        let data = b"Hello\r\nWorld\r\nTest\r\n";

        let result = split_by_lines_impl(data).unwrap();

        // Should have 3 chunks
        assert_eq!(result.chunks.len(), 3);

        // Verify chunk contents (should include \r\n)
        assert_eq!(result.chunks[0].data, b"Hello\r\n");
        assert_eq!(result.chunks[1].data, b"World\r\n");
        assert_eq!(result.chunks[2].data, b"Test\r\n");
    }

    #[test]
    fn test_line_chunking_mixed() {
        let data = b"Hello\nWorld\r\nTest\n";

        let result = split_by_lines_impl(data).unwrap();

        // Should have 3 chunks
        assert_eq!(result.chunks.len(), 3);

        // Verify chunk contents
        assert_eq!(result.chunks[0].data, b"Hello\n");
        assert_eq!(result.chunks[1].data, b"World\r\n");
        assert_eq!(result.chunks[2].data, b"Test\n");
    }

    #[test]
    fn test_line_chunking_no_trailing_newline() {
        let data = b"Hello\nWorld\nTest";

        let result = split_by_lines_impl(data).unwrap();

        // Should have 3 chunks
        assert_eq!(result.chunks.len(), 3);

        // Verify chunk contents
        assert_eq!(result.chunks[0].data, b"Hello\n");
        assert_eq!(result.chunks[1].data, b"World\n");
        assert_eq!(result.chunks[2].data, b"Test");
    }

    #[test]
    fn test_line_chunking_empty_lines() {
        let data = b"Hello\n\nWorld\n\n\n";

        let result = split_by_lines_impl(data).unwrap();

        // Should have 5 chunks (including empty lines)
        // "Hello\n", "\n", "World\n", "\n", "\n"
        assert_eq!(result.chunks.len(), 5);

        // Verify chunk contents
        assert_eq!(result.chunks[0].data, b"Hello\n");
        assert_eq!(result.chunks[1].data, b"\n");
        assert_eq!(result.chunks[2].data, b"World\n");
        assert_eq!(result.chunks[3].data, b"\n");
        assert_eq!(result.chunks[4].data, b"\n");
    }

    #[test]
    fn test_line_chunking_empty_file() {
        let data = b"";

        let result = split_by_lines_impl(data).unwrap();

        // Should have 1 empty chunk
        assert_eq!(result.chunks.len(), 1);
        assert_eq!(result.chunks[0].data, b"");
    }

    #[test]
    fn test_detect_line_ending() {
        // Test Unix detection
        let unix_data = b"Hello\nWorld\n";
        assert_eq!(detect_line_ending(unix_data), LineEndingType::Unix);

        // Test Windows detection
        let windows_data = b"Hello\r\nWorld\r\n";
        assert_eq!(detect_line_ending(windows_data), LineEndingType::Windows);

        // Test mixed detection
        let mixed_data = b"Hello\nWorld\r\n";
        assert_eq!(detect_line_ending(mixed_data), LineEndingType::Mixed);

        // Test no newlines
        let no_newlines = b"Hello World";
        assert_eq!(detect_line_ending(no_newlines), LineEndingType::Mixed);
    }

    #[test]
    fn test_custom_line_ending_unix() {
        let data = b"Hello\nWorld\n";

        let result = split_by_lines_custom::<UnixLineEnding>(data).unwrap();

        assert_eq!(result.chunks.len(), 2);
        assert_eq!(result.chunks[0].data, b"Hello\n");
        assert_eq!(result.chunks[1].data, b"World\n");
    }

    #[test]
    fn test_custom_line_ending_windows() {
        let data = b"Hello\r\nWorld\r\n";

        let result = split_by_lines_custom::<WindowsLineEnding>(data).unwrap();

        assert_eq!(result.chunks.len(), 2);
        assert_eq!(result.chunks[0].data, b"Hello\r\n");
        assert_eq!(result.chunks[1].data, b"World\r\n");
    }

    #[test]
    fn test_line_ending_type_split() {
        let unix_data = b"Hello\nWorld\n";
        let windows_data = b"Hello\r\nWorld\r\n";
        let mixed_data = b"Hello\nWorld\r\n";

        // Test Unix
        let unix_result = LineEndingType::Unix.split_by_lines(unix_data).unwrap();
        assert_eq!(unix_result.chunks.len(), 2);

        // Test Windows
        let windows_result = LineEndingType::Windows
            .split_by_lines(windows_data)
            .unwrap();
        assert_eq!(windows_result.chunks.len(), 2);

        // Test Mixed
        let mixed_result = LineEndingType::Mixed.split_by_lines(mixed_data).unwrap();
        assert_eq!(mixed_result.chunks.len(), 2);
    }

    #[test]
    fn test_chunk_hash_uniqueness() {
        // Test that different lines produce different hashes
        let data1 = b"Hello\n";
        let data2 = b"World\n";

        let result1 = split_by_lines_impl(data1).unwrap();
        let result2 = split_by_lines_impl(data2).unwrap();

        assert_ne!(result1.chunks[0].hash, result2.chunks[0].hash);
    }
}
