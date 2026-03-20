use std::path::Path;
use std::path::PathBuf;
use tokio::fs;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

use crate::index_source::IndexSource;
use crate::index_source::error::IDAliasError;

const MAX_ENTRIES_PER_FILE: u32 = 65536;
const ENTRY_SIZE: usize = 4;
const FILE_SIZE: u64 = (MAX_ENTRIES_PER_FILE as u64) * (ENTRY_SIZE as u64);

impl IndexSource {
    /// Convert local index source to remote index source
    ///
    /// This method converts the local index source's ID to the corresponding remote ID,
    /// and updates the index source's state to remote.
    /// If the index source is already in remote state, it returns directly.
    ///
    /// - `alias_dir` - Directory path where alias files are stored
    /// - `Result<IndexSource, (IndexSource, aliasError)>` - Returns the converted index source on success,
    ///   or returns the original index source along with the error if conversion fails
    pub async fn to_remote_namespace(
        mut self,
        alias_dir: PathBuf,
    ) -> Result<IndexSource, (IndexSource, IDAliasError)> {
        if self.remote {
            return Ok(self);
        }

        let new_id = match convert_to_remote(alias_dir, self.id).await {
            Ok(id) => id,
            Err(e) => return Err((self, e)),
        };
        self.id = new_id;
        self.set_is_remote(true);
        Ok(self)
    }
}

pub struct IndexSourceAliasesManager;
impl IndexSourceAliasesManager {
    /// Write a alias between local and remote IDs
    pub async fn write_alias(
        aliases_dir: impl Into<PathBuf>,
        local_id: u32,
        remote_id: u32,
    ) -> Result<(), IDAliasError> {
        write_alias(aliases_dir.into(), local_id, remote_id).await
    }

    /// Delete a alias between local and remote IDs
    pub async fn delete_alias(
        aliases_dir: impl Into<PathBuf>,
        local_id: u32,
    ) -> Result<(), IDAliasError> {
        delete_alias(aliases_dir.into(), local_id).await
    }

    /// Check if a alias exists between local and remote IDs
    pub async fn alias_exists(
        aliases_dir: impl Into<PathBuf>,
        local_id: u32,
    ) -> Result<bool, IDAliasError> {
        alias_exists(aliases_dir.into(), local_id).await
    }
}

async fn write_alias(
    aliases_dir: PathBuf,
    local_id: u32,
    remote_id: u32,
) -> Result<(), IDAliasError> {
    ensure_aliases_dir(&aliases_dir).await?;

    let (file_index, offset) = get_file_path_and_offset(local_id);
    let file = get_or_create_alias_file(&aliases_dir, file_index).await?;
    write_entry_to_file(file, offset, remote_id).await?;

    Ok(())
}

async fn delete_alias(aliases_dir: PathBuf, local_id: u32) -> Result<(), IDAliasError> {
    ensure_aliases_dir(&aliases_dir).await?;

    let (file_index, offset) = get_file_path_and_offset(local_id);
    let file = get_or_create_alias_file(&aliases_dir, file_index).await?;
    write_entry_to_file(file, offset, 0).await?;

    Ok(())
}

async fn alias_exists(aliases_dir: PathBuf, local_id: u32) -> Result<bool, IDAliasError> {
    ensure_aliases_dir(&aliases_dir).await?;

    let (file_index, offset) = get_file_path_and_offset(local_id);
    let file = get_or_create_alias_file(&aliases_dir, file_index).await?;
    let remote_id = read_entry_from_file(file, offset).await?;

    Ok(remote_id != 0)
}

pub async fn convert_to_remote(aliases_dir: PathBuf, local_id: u32) -> Result<u32, IDAliasError> {
    ensure_aliases_dir(&aliases_dir).await?;

    let (file_index, offset) = get_file_path_and_offset(local_id);
    let file = get_or_create_alias_file(&aliases_dir, file_index).await?;
    let remote_id = read_entry_from_file(file, offset).await?;

    if remote_id == 0 {
        return Err(IDAliasError::AliasNotFound(local_id));
    }

    Ok(remote_id)
}

fn get_file_path_and_offset(id: u32) -> (u32, u64) {
    let file_index = id / MAX_ENTRIES_PER_FILE;
    let offset_within_file = id % MAX_ENTRIES_PER_FILE;
    let byte_offset = (offset_within_file as u64) * (ENTRY_SIZE as u64);
    (file_index, byte_offset)
}

async fn ensure_aliases_dir(aliases_dir: &Path) -> Result<(), IDAliasError> {
    if !aliases_dir.exists() {
        fs::create_dir_all(aliases_dir).await?;
    }
    Ok(())
}

async fn get_or_create_alias_file(
    aliases_dir: &Path,
    file_index: u32,
) -> Result<fs::File, IDAliasError> {
    let file_path = aliases_dir.join(file_index.to_string());

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&file_path)
        .await
        .map_err(IDAliasError::Io)?;

    let metadata = file.metadata().await.map_err(IDAliasError::Io)?;
    if metadata.len() != FILE_SIZE {
        drop(file);
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&file_path)
            .await
            .map_err(IDAliasError::Io)?;

        file.set_len(FILE_SIZE).await.map_err(IDAliasError::Io)?;

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&file_path)
            .await
            .map_err(IDAliasError::Io)?;

        Ok(file)
    } else {
        Ok(file)
    }
}

async fn write_entry_to_file(
    mut file: fs::File,
    offset: u64,
    remote_id: u32,
) -> Result<(), IDAliasError> {
    file.seek(std::io::SeekFrom::Start(offset)).await?;
    file.write_all(&remote_id.to_le_bytes()).await?;

    Ok(())
}

async fn read_entry_from_file(mut file: fs::File, offset: u64) -> Result<u32, IDAliasError> {
    file.seek(std::io::SeekFrom::Start(offset)).await?;

    let mut remote_buf = [0u8; 4];
    file.read_exact(&mut remote_buf).await?;

    let remote_id = u32::from_le_bytes(remote_buf);
    Ok(remote_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn get_test_dir(test_name: &str) -> PathBuf {
        let current_dir = env::current_dir().expect("Failed to get current directory");
        let test_dir = current_dir
            .join(".temp")
            .join("index_source")
            .join(test_name);
        test_dir
    }

    async fn cleanup_test_dir(test_name: &str) {
        let test_dir = get_test_dir(test_name);
        if test_dir.exists() {
            tokio::fs::remove_dir_all(&test_dir)
                .await
                .expect("Failed to cleanup test directory");
        }
    }

    async fn setup_test(test_name: &str) -> PathBuf {
        cleanup_test_dir(test_name).await;
        let test_dir = get_test_dir(test_name);
        tokio::fs::create_dir_all(&test_dir)
            .await
            .expect("Failed to create test directory");
        test_dir
    }

    #[test]
    fn test_get_file_path_and_offset() {
        assert_eq!(get_file_path_and_offset(0), (0, 0));
        assert_eq!(get_file_path_and_offset(1), (0, 4));
        assert_eq!(get_file_path_and_offset(65535), (0, 262140)); // 65535 * 4
        assert_eq!(get_file_path_and_offset(65536), (1, 0));
        assert_eq!(get_file_path_and_offset(65537), (1, 4));
        assert_eq!(get_file_path_and_offset(131071), (1, 262140)); // (131071-65536) * 4
        assert_eq!(get_file_path_and_offset(131072), (2, 0));
    }

    #[tokio::test]
    async fn test_write_and_read_alias() {
        let test_dir = setup_test("test_write_and_read_alias").await;

        write_alias(test_dir.clone(), 100, 200)
            .await
            .expect("Failed to write alias");

        let exists = alias_exists(test_dir.clone(), 100)
            .await
            .expect("Failed to check alias");
        assert!(exists, "alias should exist");

        let remote_id = convert_to_remote(test_dir.clone(), 100)
            .await
            .expect("Failed to convert to remote");
        assert_eq!(remote_id, 200, "Remote ID should be 200");

        let result = convert_to_remote(test_dir.clone(), 999).await;
        assert!(
            result.is_err(),
            "Should return error for non-existent alias"
        );

        cleanup_test_dir("test_write_and_read_alias").await;
    }

    #[tokio::test]
    async fn test_delete_alias() {
        let test_dir = setup_test("test_delete_alias").await;

        write_alias(test_dir.clone(), 300, 400)
            .await
            .expect("Failed to write alias");

        let exists = alias_exists(test_dir.clone(), 300)
            .await
            .expect("Failed to check alias");
        assert!(exists, "alias should exist before deletion");

        delete_alias(test_dir.clone(), 300)
            .await
            .expect("Failed to delete alias");

        let exists = alias_exists(test_dir.clone(), 300)
            .await
            .expect("Failed to check alias");
        assert!(!exists, "alias should not exist after deletion");

        let result = convert_to_remote(test_dir.clone(), 300).await;
        assert!(result.is_err(), "Should return error after deletion");

        cleanup_test_dir("test_delete_alias").await;
    }

    #[tokio::test]
    async fn test_large_id_alias() {
        let test_dir = setup_test("test_large_id_alias").await;

        let large_local_id = 70000;
        let large_remote_id = 80000;

        write_alias(test_dir.clone(), large_local_id, large_remote_id)
            .await
            .expect("Failed to write large alias");

        let exists = alias_exists(test_dir.clone(), large_local_id)
            .await
            .expect("Failed to check alias");
        assert!(exists, "Large alias should exist");

        let remote_id = convert_to_remote(test_dir.clone(), large_local_id)
            .await
            .expect("Failed to convert large to remote");
        assert_eq!(remote_id, large_remote_id, "Remote ID should match");

        cleanup_test_dir("test_large_id_alias").await;
    }

    #[tokio::test]
    async fn test_multiple_aliass() {
        let test_dir = setup_test("test_multiple_aliass").await;

        let aliass = vec![(10, 20), (30, 40), (50, 60), (1000, 2000), (70000, 80000)];

        for (local, remote) in &aliass {
            write_alias(test_dir.clone(), *local, *remote)
                .await
                .expect("Failed to write alias");
        }

        for (local, remote) in &aliass {
            let exists = alias_exists(test_dir.clone(), *local)
                .await
                .expect("Failed to check alias");
            assert!(exists, "alias for local {} should exist", local);

            let converted_remote = convert_to_remote(test_dir.clone(), *local)
                .await
                .expect("Failed to convert to remote");
            assert_eq!(
                converted_remote, *remote,
                "Remote ID should match for local {}",
                local
            );
        }

        let result = alias_exists(test_dir.clone(), 9999)
            .await
            .expect("Failed to check non-existent alias");
        assert!(!result, "Non-existent alias should return false");

        cleanup_test_dir("test_multiple_aliass").await;
    }

    #[tokio::test]
    async fn test_file_creation_and_size() {
        let test_dir = setup_test("test_file_creation_and_size").await;

        write_alias(test_dir.clone(), 500, 600)
            .await
            .expect("Failed to write alias");

        let file_path = test_dir.join("0");
        assert!(file_path.exists(), "File should exist");

        let metadata = tokio::fs::metadata(&file_path)
            .await
            .expect("Failed to get file metadata");
        assert_eq!(
            metadata.len(),
            FILE_SIZE,
            "File size should be {}",
            FILE_SIZE
        );

        write_alias(test_dir.clone(), 70000, 80000)
            .await
            .expect("Failed to write alias");

        let file_path_1 = test_dir.join("1");
        assert!(file_path_1.exists(), "File 1 should exist");

        let metadata_1 = tokio::fs::metadata(&file_path_1)
            .await
            .expect("Failed to get file metadata");
        assert_eq!(
            metadata_1.len(),
            FILE_SIZE,
            "File 1 size should be {}",
            FILE_SIZE
        );

        cleanup_test_dir("test_file_creation_and_size").await;
    }

    #[tokio::test]
    async fn test_error_handling() {
        let test_dir = setup_test("test_error_handling").await;

        let result = convert_to_remote(test_dir.clone(), 100).await;
        assert!(
            matches!(result, Err(IDAliasError::AliasNotFound(100))),
            "Should return aliasNotFound error"
        );

        write_alias(test_dir.clone(), 200, 300)
            .await
            .expect("Failed to write alias");
        delete_alias(test_dir.clone(), 200)
            .await
            .expect("Failed to delete alias");

        let result = convert_to_remote(test_dir.clone(), 200).await;
        assert!(
            matches!(result, Err(IDAliasError::AliasNotFound(200))),
            "Should return aliasNotFound after deletion"
        );

        cleanup_test_dir("test_error_handling").await;
    }
}
