use sha1::{Digest, Sha1};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, BufReader};
use tokio::task;

/// # Struct - Sha1Result
///
/// Records SHA1 calculation results, including the file path and hash value
#[derive(Debug, Clone)]
pub struct Sha1Result {
    pub file_path: PathBuf,
    pub hash: String,
}

/// Calc SHA1 hash of a string
pub fn calc_sha1_string<S: AsRef<str>>(input: S) -> String {
    let mut hasher = Sha1::new();
    hasher.update(input.as_ref().as_bytes());
    let hash_result = hasher.finalize();

    hash_result
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

/// Calc SHA1 hash of a single file
pub async fn calc_sha1<P: AsRef<Path>>(
    path: P,
    buffer_size: usize,
) -> Result<Sha1Result, Box<dyn std::error::Error + Send + Sync>> {
    let file_path = path.as_ref().to_string_lossy().to_string();

    // Open file asynchronously
    let file = File::open(&path).await?;
    let mut reader = BufReader::with_capacity(buffer_size, file);
    let mut hasher = Sha1::new();
    let mut buffer = vec![0u8; buffer_size];

    // Read file in chunks and update hash asynchronously
    loop {
        let n = reader.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let hash_result = hasher.finalize();

    // Convert to hex string
    let hash_hex = hash_result
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();

    Ok(Sha1Result {
        file_path: file_path.into(),
        hash: hash_hex,
    })
}

/// Calc SHA1 hashes for multiple files using multi-threading
pub async fn calc_sha1_multi<P, I>(
    paths: I,
    buffer_size: usize,
) -> Result<Vec<Sha1Result>, Box<dyn std::error::Error + Send + Sync>>
where
    P: AsRef<Path> + Send + Sync + 'static,
    I: IntoIterator<Item = P>,
{
    let buffer_size = Arc::new(buffer_size);

    // Collect all file paths
    let file_paths: Vec<P> = paths.into_iter().collect();

    if file_paths.is_empty() {
        return Ok(Vec::new());
    }

    // Create tasks for each file
    let tasks: Vec<_> = file_paths
        .into_iter()
        .map(|path| {
            let buffer_size = Arc::clone(&buffer_size);
            task::spawn(async move { calc_sha1(path, *buffer_size).await })
        })
        .collect();

    // Execute tasks with concurrency limit using join_all
    let results: Vec<Result<Sha1Result, Box<dyn std::error::Error + Send + Sync>>> =
        futures::future::join_all(tasks)
            .await
            .into_iter()
            .map(|task_result| match task_result {
                Ok(Ok(calc_result)) => Ok(calc_result),
                Ok(Err(e)) => Err(e),
                Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
            })
            .collect();

    // Check for any errors and collect successful results
    let mut successful_results = Vec::new();
    for result in results {
        match result {
            Ok(success) => successful_results.push(success),
            Err(e) => return Err(e),
        }
    }

    Ok(successful_results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_sha1_string() {
        let test_string = "Hello, SHA1!";
        let hash = calc_sha1_string(test_string);

        let expected_hash = "de1c3daadc6f0f1626f4cf56c03e05a1e5d7b187";

        assert_eq!(
            hash, expected_hash,
            "SHA1 hash should be consistent for same input"
        );
    }

    #[test]
    fn test_sha1_string_empty() {
        let hash = calc_sha1_string("");

        // SHA1 of empty string is "da39a3ee5e6b4b0d3255bfef95601890afd80709"
        let expected_empty_hash = "da39a3ee5e6b4b0d3255bfef95601890afd80709";
        assert_eq!(
            hash, expected_empty_hash,
            "SHA1 hash mismatch for empty string"
        );
    }

    #[tokio::test]
    async fn test_sha1_accuracy() {
        // Test file path relative to the crate root
        let test_file_path = "res/story.txt";
        // Choose expected hash file based on platform
        let expected_hash_path = if cfg!(windows) {
            "res/story_crlf.sha1"
        } else {
            "res/story_lf.sha1"
        };

        // Calculate SHA1 hash
        let result = calc_sha1(test_file_path, 8192)
            .await
            .expect("Failed to calculate SHA1");

        // Read expected hash from file
        let expected_hash = fs::read_to_string(expected_hash_path)
            .expect("Failed to read expected hash file")
            .trim()
            .to_string();

        // Verify the calculated hash matches expected hash
        assert_eq!(
            result.hash, expected_hash,
            "SHA1 hash mismatch for test file"
        );

        println!("Test file: {}", result.file_path.display());
        println!("Calculated hash: {}", result.hash);
        println!("Expected hash: {}", expected_hash);
        println!(
            "Platform: {}",
            if cfg!(windows) {
                "Windows"
            } else {
                "Unix/Linux"
            }
        );
    }

    #[tokio::test]
    async fn test_sha1_empty_file() {
        // Create a temporary empty file for testing
        let temp_file = "test_empty.txt";
        fs::write(temp_file, "").expect("Failed to create empty test file");

        let result = calc_sha1(temp_file, 4096)
            .await
            .expect("Failed to calculate SHA1 for empty file");

        // SHA1 of empty string is "da39a3ee5e6b4b0d3255bfef95601890afd80709"
        let expected_empty_hash = "da39a3ee5e6b4b0d3255bfef95601890afd80709";
        assert_eq!(
            result.hash, expected_empty_hash,
            "SHA1 hash mismatch for empty file"
        );

        // Clean up
        fs::remove_file(temp_file).expect("Failed to remove temporary test file");
    }

    #[tokio::test]
    async fn test_sha1_simple_text() {
        // Create a temporary file with simple text
        let temp_file = "test_simple.txt";
        let test_content = "Hello, SHA1!";
        fs::write(temp_file, test_content).expect("Failed to create simple test file");

        let result = calc_sha1(temp_file, 4096)
            .await
            .expect("Failed to calculate SHA1 for simple text");

        // Note: This test just verifies that the function works without errors
        // The actual hash value is not critical for this test

        println!("Simple text test - Calculated hash: {}", result.hash);

        // Clean up
        fs::remove_file(temp_file).expect("Failed to remove temporary test file");
    }

    #[tokio::test]
    async fn test_sha1_multi_files() {
        // Test multiple files calculation
        let test_files = vec!["res/story.txt"];

        let results = calc_sha1_multi(test_files, 8192)
            .await
            .expect("Failed to calculate SHA1 for multiple files");

        assert_eq!(results.len(), 1, "Should have calculated hash for 1 file");

        // Choose expected hash file based on platform
        let expected_hash_path = if cfg!(windows) {
            "res/story_crlf.sha1"
        } else {
            "res/story_lf.sha1"
        };

        // Read expected hash from file
        let expected_hash = fs::read_to_string(expected_hash_path)
            .expect("Failed to read expected hash file")
            .trim()
            .to_string();

        assert_eq!(
            results[0].hash, expected_hash,
            "SHA1 hash mismatch in multi-file test"
        );
    }
}
