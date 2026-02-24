use std::{
    borrow::Cow,
    collections::HashSet,
    ffi::OsStr,
    marker::PhantomData,
    path::{Path, PathBuf},
};

use constants::{LOCK_FILE_PREFIX, TEMP_FILE_PREFIX};
use string_proc::format_path::format_path;

use crate::{
    error::{DataApplyError, DataReadError, DataWriteError, HandleLockError, PrecheckFailed},
    rw::RWData,
};

pub struct ReadOnlyAsset<RWDataType>
where
    RWDataType: RWData<RWDataType>,
{
    _data_type: PhantomData<RWDataType>,
    path: PathBuf,
}

/// Nothing special, I'm just too lazy
macro_rules! asset_from {
    (|$v:ident| $src:ty => $expr:expr) => {
        impl<RWDataType> From<$src> for ReadOnlyAsset<RWDataType>
        where
            RWDataType: RWData<RWDataType>,
        {
            fn from($v: $src) -> Self {
                ReadOnlyAsset {
                    _data_type: PhantomData,
                    path: $expr,
                }
            }
        }
    };
}

asset_from!(|v| PathBuf => v);
asset_from!(|v| &Path => v.to_path_buf());
asset_from!(|v| String => PathBuf::from(v));
asset_from!(|v| &str => PathBuf::from(v));

impl<RWDataType> AsRef<Path> for ReadOnlyAsset<RWDataType>
where
    RWDataType: RWData<RWDataType>,
{
    fn as_ref(&self) -> &Path {
        self.path.as_ref()
    }
}

impl<RWDataType> From<ReadOnlyAsset<RWDataType>> for PathBuf
where
    RWDataType: RWData<RWDataType>,
{
    fn from(value: ReadOnlyAsset<RWDataType>) -> Self {
        value.path
    }
}

impl<RWDataType> ReadOnlyAsset<RWDataType>
where
    RWDataType: RWData<RWDataType>,
{
    /// Read asset content from `ReadOnlyAsset`
    /// ```ignore
    /// let sheet_asset: ReadOnlyAsset<Sheet> = "my_sheet.sheet".into();
    /// let sheet = sheet_asset.read().await.unwrap();
    /// ```
    pub async fn read(&self) -> Result<RWDataType, DataReadError> {
        RWDataType::read(&self.path).await
    }

    /// Create a `Handle` from `ReadOnlyAsset` to exclusively edit this `ReadOnlyAsset`
    /// ```ignore
    /// let sheet_asset: ReadOnlyAsset<Sheet> = "my_sheet.sheet".into();
    /// let mut sheet_handle = sheet_asset.get_handle().await.unwrap();
    /// ```
    pub async fn get_handle(&self) -> Result<Handle<RWDataType>, HandleLockError> {
        let asset_path = &self.path;

        // If the asset file does not exist, cannot create an editable handle
        if !asset_path.exists() {
            return Err(HandleLockError::AssetFileNotFound(asset_path.clone()));
        }

        // Get the lock path and temp path
        let lock_path = self.get_lock_path()?;
        let temp_path = self.get_temp_path()?;

        // Attempt to create the lock file; success means the lock does not exist and editing is allowed
        match tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
            .await
        {
            Ok(_) => {
                return Ok(Handle {
                    _data_type: PhantomData,
                    writed: false,
                    asset_path: self.path.clone(),
                    lock_path,
                    temp_path,
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(HandleLockError::AssetLocked);
            }
            Err(e) => {
                return Err(HandleLockError::IoError(e));
            }
        }
    }

    /// Get the lock file name for the current `ReadOnlyAsset`
    /// ```
    /// # use asset::rw::FooData;
    /// # use asset::asset::ReadOnlyAsset;
    /// let foo_asset = ReadOnlyAsset::<FooData>::from("my/foo.txt");
    /// let lock_path = foo_asset
    ///     .get_lock_path()
    ///     .unwrap();
    /// assert_eq!(lock_path.to_string_lossy(), "my/~foo.txt");
    /// ```
    pub fn get_lock_path(&self) -> Result<PathBuf, HandleLockError> {
        // Get the file name
        let Some(file_name) = self.path.file_name() else {
            return Err(HandleLockError::ReadFileNameFailed);
        };

        let file_name_str = check_path(file_name)?;

        let mut edit_path = self.path.clone();
        edit_path.set_file_name(format!("{}{}", LOCK_FILE_PREFIX, file_name_str));

        let Ok(edit_path) = format_path(edit_path) else {
            return Err(HandleLockError::ParsePathFailed);
        };

        Ok(edit_path)
    }

    /// Get the name of the temporary editing file for the current `ReadOnlyAsset`
    /// ```
    /// # use asset::rw::FooData;
    /// # use asset::asset::ReadOnlyAsset;
    /// let foo_asset = ReadOnlyAsset::<FooData>::from("my/foo.txt");
    /// let temp_path = foo_asset
    ///     .get_temp_path()
    ///     .unwrap();
    /// assert_eq!(temp_path.to_string_lossy(), "my/.tmp_foo.txt");
    /// ```
    pub fn get_temp_path(&self) -> Result<PathBuf, HandleLockError> {
        // Get the file name
        let Some(file_name) = self.path.file_name() else {
            return Err(HandleLockError::ReadFileNameFailed);
        };

        let file_name_str = check_path(file_name)?;

        let mut temp_path = self.path.clone();
        temp_path.set_file_name(format!("{}{}", TEMP_FILE_PREFIX, file_name_str));

        let Ok(edit_path) = format_path(temp_path) else {
            return Err(HandleLockError::ParsePathFailed);
        };

        Ok(edit_path)
    }
}

pub struct Handle<RWDataType>
where
    RWDataType: RWData<RWDataType>,
{
    _data_type: PhantomData<RWDataType>,
    writed: bool,
    asset_path: PathBuf,
    lock_path: PathBuf,
    temp_path: PathBuf,
}

impl<RWDataType> Drop for Handle<RWDataType>
where
    RWDataType: RWData<RWDataType>,
{
    fn drop(&mut self) {
        if self.lock_path.exists() {
            let _ = std::fs::remove_file(&self.lock_path);
        }
    }
}

impl<RWDataType> Handle<RWDataType>
where
    RWDataType: RWData<RWDataType>,
{
    /// Write content into the `Handle`, this will not affect the associated `ReadOnlyAsset`
    pub async fn write(&mut self, data: RWDataType) -> Result<(), DataWriteError> {
        self.writed = true;
        RWDataType::write(data, &self.temp_path).await
    }

    /// Read the content of this `Handle`;
    /// if the `Handle` has never been written to, read the content of the original `ReadOnlyAsset`
    pub async fn read(&self) -> Result<RWDataType, DataReadError> {
        let path = if self.writed {
            &self.temp_path
        } else {
            &self.asset_path
        };
        RWDataType::read(path).await
    }

    /// Atomically write the content of the `Handle` to the `ReadOnlyAsset`
    pub async fn apply(self) -> Result<(), DataApplyError> {
        let from = &self.temp_path;
        let to = &self.asset_path;

        // Cannot perform Apply operation when the target file does not exist
        // Reason: Cannot apply modifications to a non-existent file, even if editing was declared
        if !to.exists() {
            return Err(DataApplyError::AssetFileNotFound(to.clone()));
        }

        if self.writed {
            tokio::fs::rename(&from, &to)
                .await
                .map_err(|e| DataApplyError::IoError(e))?;
        }
        Ok(())
    }

    pub fn get_rename_operation(&self) -> (PathBuf, PathBuf) {
        (self.temp_path.clone(), self.asset_path.clone())
    }
}

/// Strict Apply precheck
///
/// It checks:
/// 1. Whether related files exist
/// 2. If the Handle has been written to, whether the written content can be found
/// 3. Whether the Handle and its ReadOnlyAsset are in the same directory
/// 4. Whether the ReadOnlyAsset is locked (preventing writes/renames)
/// 5. Whether the Handle is locked (preventing writes/renames)
///
/// > This is only necessary when you need to ensure `multiple Handles must either all succeed or all fail together`
///
/// If you only need to ensure a single Handle can Apply correctly,
/// use:
/// ```ignore
/// // ...
/// let mut handle = my_asset.get_handle().await?;
/// handle.write(/* Your Data */).await?;
/// handle.apply().await?;
/// ```
pub async fn apply_precheck<D>(handle: &Handle<D>) -> Result<(), PrecheckFailed>
where
    D: RWData<D>,
{
    let Ok(from) = format_path(&handle.temp_path) else {
        return Err(PrecheckFailed::FormatPathFailed);
    };
    let Ok(to) = format_path(&handle.asset_path) else {
        return Err(PrecheckFailed::FormatPathFailed);
    };

    tokio::try_join!(
        check_lock_file_exist(handle),
        check_asset_file_exist(handle),
        check_temp_file_exist_when_writed(handle),
        check_asset_path(handle),
        check_handle_is_cross_directory(&from, &to),
    )?;
    Ok(())
}

/// Check if all rename operations to be performed are valid
pub async fn apply_precheck_rename_operations(
    rename_operations: &[(PathBuf, PathBuf)],
) -> Result<(), PrecheckFailed> {
    let mut seen = HashSet::new();

    for (from, to) in rename_operations {
        if !seen.insert(from) {
            return Err(PrecheckFailed::HasSamePath);
        }
        if !seen.insert(to) {
            return Err(PrecheckFailed::HasSamePath);
        }
    }

    // This validation must be executed sequentially
    // Because on Unix, we need to attempt writing files to directories
    // If executed in parallel, conflicts may arise

    for (from, to) in rename_operations {
        #[cfg(windows)]
        check_temp_file_moveable_windows(from, to).await?;

        #[cfg(windows)]
        check_asset_file_writeable_windows(to).await?;

        #[cfg(unix)]
        check_temp_file_moveable_unix(from, to).await?;

        #[cfg(unix)]
        check_asset_file_writeable_unix(to).await?;
    }
    Ok(())
}

async fn check_lock_file_exist<D>(handle: &Handle<D>) -> Result<(), PrecheckFailed>
where
    D: RWData<D>,
{
    if !handle.lock_path.exists() {
        return Err(PrecheckFailed::LockNotFound);
    }
    Ok(())
}

async fn check_asset_file_exist<D>(handle: &Handle<D>) -> Result<(), PrecheckFailed>
where
    D: RWData<D>,
{
    if !handle.asset_path.exists() {
        return Err(PrecheckFailed::AssetNotFound);
    }
    Ok(())
}

async fn check_temp_file_exist_when_writed<D>(handle: &Handle<D>) -> Result<(), PrecheckFailed>
where
    D: RWData<D>,
{
    if handle.writed && !handle.temp_path.exists() {
        return Err(PrecheckFailed::WritedButTempNotFound);
    }
    Ok(())
}

async fn check_asset_path<D>(handle: &Handle<D>) -> Result<(), PrecheckFailed>
where
    D: RWData<D>,
{
    if let Some(file_name) = handle.asset_path.file_name() {
        if check_path(file_name).is_ok() {
            return Ok(());
        }
    }
    Err(PrecheckFailed::AssetPathInvalid)
}

async fn check_handle_is_cross_directory(
    from: &PathBuf,
    to: &PathBuf,
) -> Result<(), PrecheckFailed> {
    let from_parent = from.parent();
    let to_parent = to.parent();

    // "Huh? You ask why we report an error when the parent directory doesn't exist?"
    //
    // Operations are not allowed in the root directory or when there is no parent directory.
    // 1. This indicates they are not in the correct location
    //    (ideally there should be at least one parent directory).
    // 2. This makes it impossible to compare the two directories.
    if from_parent.is_none() || to_parent.is_none() {
        return Err(PrecheckFailed::HandleFileIsNoParent);
    }

    if from_parent.unwrap() != to_parent.unwrap() {
        return Err(PrecheckFailed::HandleIsCrossDirectory);
    }

    Ok(())
}

fn check_path(file_name: &OsStr) -> Result<Cow<'_, str>, PrecheckFailed> {
    let file_name_str = file_name.to_string_lossy();

    // When operating on a TEMP_FILE or LOCK_FILE,
    // names like `~~foo.txt` or `.tmp_.tmp_foo.txt` would be generated
    // This is not expected and should result in an error

    // Check if the file name starts with ~
    if file_name_str.starts_with(LOCK_FILE_PREFIX) {
        return Err(PrecheckFailed::LockOnLockFile);
    }

    // Check if the file name starts with .tmp_
    if file_name_str.starts_with(TEMP_FILE_PREFIX) {
        return Err(PrecheckFailed::TempForTempFile);
    }

    Ok(file_name_str)
}

#[cfg(windows)]
async fn check_asset_file_writeable_windows(path: &Path) -> Result<(), PrecheckFailed> {
    use std::os::windows::fs::MetadataExt;

    let metadata = tokio::fs::metadata(path)
        .await
        .map_err(|_| PrecheckFailed::AssetNotWritable)?;

    if metadata.file_attributes() & 0x1 != 0 {
        return Err(PrecheckFailed::AssetNotWritable);
    }

    if !check_file_can_be_deleted_windows(path) {
        return Err(PrecheckFailed::AssetNotWritable);
    }

    Ok(())
}

#[cfg(windows)]
async fn check_temp_file_moveable_windows(temp: &Path, asset: &Path) -> Result<(), PrecheckFailed> {
    if !check_file_can_be_deleted_windows(temp) {
        return Err(PrecheckFailed::TempNotMoveable);
    }
    if asset.exists() && !check_file_can_be_deleted_windows(asset) {
        return Err(PrecheckFailed::TempNotMoveable);
    }

    Ok(())
}

#[cfg(windows)]
fn check_file_can_be_deleted_windows(path: &Path) -> bool {
    use std::os::windows::fs::OpenOptionsExt;

    use winapi::um::winnt::{DELETE, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE};
    std::fs::OpenOptions::new()
        .access_mode(DELETE)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .open(path)
        .is_ok()
}

#[cfg(unix)]
async fn check_asset_file_writeable_unix(path: &Path) -> Result<(), PrecheckFailed> {
    let parent = path.parent().ok_or(PrecheckFailed::AssetPathInvalid)?;

    if !check_dir_writable_unix(parent) {
        return Err(PrecheckFailed::AssetNotWritable);
    }

    Ok(())
}

#[cfg(unix)]
async fn check_temp_file_moveable_unix(temp: &Path, asset: &Path) -> Result<(), PrecheckFailed> {
    if !temp.exists() {
        return Err(PrecheckFailed::TempNotMoveable);
    }

    let temp_parent = temp.parent().ok_or(PrecheckFailed::TempNotMoveable)?;
    if !check_dir_writable_unix(temp_parent) {
        return Err(PrecheckFailed::TempNotMoveable);
    }

    let asset_parent = asset.parent().ok_or(PrecheckFailed::TempNotMoveable)?;
    if !check_dir_writable_unix(asset_parent) {
        return Err(PrecheckFailed::TempNotMoveable);
    }

    Ok(())
}

#[cfg(unix)]
fn check_dir_writable_unix(dir: &Path) -> bool {
    use std::fs::OpenOptions;

    let test_path = dir.join(".write_test");

    let result = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&test_path);

    match result {
        Ok(_) => {
            let _ = std::fs::remove_file(test_path);
            true
        }
        Err(_) => false,
    }
}

#[macro_export]
macro_rules! apply {
    // Single handle
    ($handle:expr $(,)?) => {{
        async {
            // Single-handle precheck
            if let Err(e) = asset::asset::apply_precheck(&$handle).await {
                return Err(asset::error::DataApplyError::PrecheckFailed(e));
            }

            // Direct apply
            $handle.apply().await
        }
    }};

    // Multiple handles
    ($first:expr, $($rest:expr),+ $(,)?) => {{
        async {
            // Collect rename ops
            let rename_ops: Vec<(std::path::PathBuf, std::path::PathBuf)> = vec![
                $first.get_rename_operation(),
                $(
                    $rest.get_rename_operation(),
                )+
            ];

            // Batch precheck
            if let Err(e) = asset_system::asset::apply_precheck_rename_operations(&rename_ops).await {
                return Err(asset_system::error::DataApplyError::PrecheckFailed(e));
            }

            // Per-handle precheck
            if let Err(e) = asset_system::asset::apply_precheck(&$first).await {
                return Err(asset_system::error::DataApplyError::PrecheckFailed(e));
            }
            $(
                if let Err(e) = asset_system::asset::apply_precheck(&$rest).await {
                    return Err(asset_system::error::DataApplyError::PrecheckFailed(e));
                }
            )+

            // Apply
            if let Err(e) = $first.apply().await {
                return Err(e);
            }
            $(
                if let Err(e) = $rest.apply().await {
                    return Err(e);
                }
            )+

            Ok(())
        }
    }};
}
