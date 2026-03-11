use crate::space::error::SpaceError;
use just_fmt::fmt_path::{PathFormatConfig, PathFormatError, fmt_path, fmt_path_custom};
use std::{
    cell::Cell,
    env::current_dir,
    ffi::OsString,
    ops::Deref,
    path::{Path, PathBuf},
};

pub mod error;

pub struct Space<T: SpaceContent> {
    path_format_cfg: PathFormatConfig,

    content: T,
    space_dir: Cell<Option<PathBuf>>,
    current_dir: Option<PathBuf>,
}

impl<T: SpaceContent> Space<T> {
    /// Create a new `Space` instance with the given content.
    pub fn new(content: T) -> Self {
        Space {
            path_format_cfg: PathFormatConfig {
                resolve_parent_dirs: true,
                ..Default::default()
            },
            content,
            space_dir: Cell::new(None),
            current_dir: None,
        }
    }

    /// Consume the `Space`, returning the inner content.
    pub fn into_inner(self) -> T {
        self.content
    }

    /// Get the space directory for the given current directory.
    ///
    /// If the space directory has already been found, it is returned from cache.
    /// Otherwise, it is found using the pattern from `T::get_pattern()`.
    pub fn space_dir(&self, current_dir: impl Into<PathBuf>) -> Result<PathBuf, SpaceError> {
        let pattern = T::get_pattern();
        match self.space_dir.take() {
            Some(dir) => {
                self.update_space_dir(Some(dir.clone()));
                Ok(dir)
            }
            None => {
                let result = find_space_root_with(current_dir.into(), pattern);
                match result {
                    Ok(r) => {
                        self.update_space_dir(Some(r.clone()));
                        Ok(r)
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }

    /// Get the space directory using the current directory.
    ///
    /// The current directory is either the explicitly set directory or the process's current directory.
    pub fn space_dir_current(&self) -> Result<PathBuf, SpaceError> {
        self.space_dir(self.current_dir()?)
    }

    /// Set the current directory explicitly.
    ///
    /// This clears any cached space directory.
    pub fn set_current_dir(&mut self, path: PathBuf) -> Result<(), PathFormatError> {
        self.update_space_dir(None);
        self.current_dir = Some(fmt_path(path)?);
        Ok(())
    }

    /// Reset the current directory to the process's current directory.
    ///
    /// This clears any cached space directory.
    pub fn reset_current_dir(&mut self) {
        self.update_space_dir(None);
        self.current_dir = None
    }

    /// Get the current directory.
    ///
    /// Returns the explicitly set directory if any, otherwise the process's current directory.
    fn current_dir(&self) -> Result<PathBuf, SpaceError> {
        match &self.current_dir {
            Some(d) => Ok(d.clone()),
            None => Ok(fmt_path(current_dir()?)?),
        }
    }

    /// Update the cached space directory.
    fn update_space_dir(&self, space_dir: Option<PathBuf>) {
        self.space_dir.set(space_dir);
    }
}

impl<T: SpaceContent> Space<T> {
    /// Convert a relative path to an absolute path within the space.
    ///
    /// The path is formatted according to the space's path format configuration.
    pub fn local_path(&self, relative_path: impl AsRef<Path>) -> Result<PathBuf, SpaceError> {
        let path = fmt_path_custom(relative_path.as_ref().to_path_buf(), &self.path_format_cfg)?;
        let raw_path = self.space_dir_current()?.join(path);
        Ok(fmt_path(raw_path)?)
    }

    /// Convert an absolute path to a relative path within the space, if possible.
    ///
    /// Returns `None` if the absolute path is not under the space directory.
    pub fn to_local_path(
        &self,
        absolute_path: impl AsRef<Path>,
    ) -> Result<Option<PathBuf>, SpaceError> {
        let path = fmt_path(absolute_path.as_ref())?;
        let current = self.space_dir_current()?;
        match path.strip_prefix(current) {
            Ok(result) => Ok(Some(result.to_path_buf())),
            Err(_) => Ok(None),
        }
    }

    /// Canonicalize a relative path within the space.
    pub async fn canonicalize(
        &self,
        relative_path: impl AsRef<Path>,
    ) -> Result<PathBuf, SpaceError> {
        let path = self.local_path(relative_path)?;
        Ok(tokio::fs::canonicalize(path).await?)
    }

    /// Copy a file from one relative path to another within the space.
    pub async fn copy(
        &self,
        from: impl AsRef<Path>,
        to: impl AsRef<Path>,
    ) -> Result<u64, SpaceError> {
        let from_path = self.local_path(from)?;
        let to_path = self.local_path(to)?;
        Ok(tokio::fs::copy(from_path, to_path).await?)
    }

    /// Create a directory at the given relative path within the space.
    pub async fn create_dir(&self, relative_path: impl AsRef<Path>) -> Result<(), SpaceError> {
        let path = self.local_path(relative_path)?;
        Ok(tokio::fs::create_dir(path).await?)
    }

    /// Recursively create a directory and all its parents at the given relative path within the space.
    pub async fn create_dir_all(&self, relative_path: impl AsRef<Path>) -> Result<(), SpaceError> {
        let path = self.local_path(relative_path)?;
        Ok(tokio::fs::create_dir_all(path).await?)
    }

    /// Create a hard link from `src` to `dst` within the space.
    pub async fn hard_link(
        &self,
        src: impl AsRef<Path>,
        dst: impl AsRef<Path>,
    ) -> Result<(), SpaceError> {
        let src_path = self.local_path(src)?;
        let dst_path = self.local_path(dst)?;
        Ok(tokio::fs::hard_link(src_path, dst_path).await?)
    }

    /// Get metadata for a file or directory at the given relative path within the space.
    pub async fn metadata(
        &self,
        relative_path: impl AsRef<Path>,
    ) -> Result<std::fs::Metadata, SpaceError> {
        let path = self.local_path(relative_path)?;
        Ok(tokio::fs::metadata(path).await?)
    }

    /// Read the entire contents of a file at the given relative path within the space.
    pub async fn read(&self, relative_path: impl AsRef<Path>) -> Result<Vec<u8>, SpaceError> {
        let path = self.local_path(relative_path)?;
        Ok(tokio::fs::read(path).await?)
    }

    /// Read the directory entries at the given relative path within the space.
    pub async fn read_dir(
        &self,
        relative_path: impl AsRef<Path>,
    ) -> Result<tokio::fs::ReadDir, SpaceError> {
        let path = self.local_path(relative_path)?;
        Ok(tokio::fs::read_dir(path).await?)
    }

    /// Read the target of a symbolic link at the given relative path within the space.
    pub async fn read_link(&self, relative_path: impl AsRef<Path>) -> Result<PathBuf, SpaceError> {
        let path = self.local_path(relative_path)?;
        Ok(tokio::fs::read_link(path).await?)
    }

    /// Read the entire contents of a file as a string at the given relative path within the space.
    pub async fn read_to_string(
        &self,
        relative_path: impl AsRef<Path>,
    ) -> Result<String, SpaceError> {
        let path = self.local_path(relative_path)?;
        Ok(tokio::fs::read_to_string(path).await?)
    }

    /// Remove an empty directory at the given relative path within the space.
    pub async fn remove_dir(&self, relative_path: impl AsRef<Path>) -> Result<(), SpaceError> {
        let path = self.local_path(relative_path)?;
        Ok(tokio::fs::remove_dir(path).await?)
    }

    /// Remove a directory and all its contents at the given relative path within the space.
    pub async fn remove_dir_all(&self, relative_path: impl AsRef<Path>) -> Result<(), SpaceError> {
        let path = self.local_path(relative_path)?;
        Ok(tokio::fs::remove_dir_all(path).await?)
    }

    /// Remove a file at the given relative path within the space.
    pub async fn remove_file(&self, relative_path: impl AsRef<Path>) -> Result<(), SpaceError> {
        let path = self.local_path(relative_path)?;
        Ok(tokio::fs::remove_file(path).await?)
    }

    /// Rename a file or directory from one relative path to another within the space.
    pub async fn rename(
        &self,
        from: impl AsRef<Path>,
        to: impl AsRef<Path>,
    ) -> Result<(), SpaceError> {
        let from_path = self.local_path(from)?;
        let to_path = self.local_path(to)?;
        Ok(tokio::fs::rename(from_path, to_path).await?)
    }

    /// Set permissions for a file or directory at the given relative path within the space.
    pub async fn set_permissions(
        &self,
        relative_path: impl AsRef<Path>,
        perm: std::fs::Permissions,
    ) -> Result<(), SpaceError> {
        let path = self.local_path(relative_path)?;
        Ok(tokio::fs::set_permissions(path, perm).await?)
    }

    /// Create a symbolic link from `src` to `dst` within the space (Unix only).
    #[cfg(unix)]
    pub async fn symlink(
        &self,
        src: impl AsRef<Path>,
        dst: impl AsRef<Path>,
    ) -> Result<(), SpaceError> {
        let src_path = self.local_path(src)?;
        let dst_path = self.local_path(dst)?;
        Ok(tokio::fs::symlink(src_path, dst_path).await?)
    }

    /// Create a directory symbolic link from `src` to `dst` within the space (Windows only).
    #[cfg(windows)]
    pub async fn symlink_dir(
        &self,
        src: impl AsRef<Path>,
        dst: impl AsRef<Path>,
    ) -> Result<(), SpaceError> {
        let src_path = self.local_path(src)?;
        let dst_path = self.local_path(dst)?;
        Ok(tokio::fs::symlink_dir(src_path, dst_path).await?)
    }

    /// Create a file symbolic link from `src` to `dst` within the space (Windows only).
    #[cfg(windows)]
    pub async fn symlink_file(
        &self,
        src: impl AsRef<Path>,
        dst: impl AsRef<Path>,
    ) -> Result<(), SpaceError> {
        let src_path = self.local_path(src)?;
        let dst_path = self.local_path(dst)?;
        Ok(tokio::fs::symlink_file(src_path, dst_path).await?)
    }

    /// Get metadata for a file or directory without following symbolic links.
    pub async fn symlink_metadata(
        &self,
        relative_path: impl AsRef<Path>,
    ) -> Result<std::fs::Metadata, SpaceError> {
        let path = self.local_path(relative_path)?;
        Ok(tokio::fs::symlink_metadata(path).await?)
    }

    /// Check if a file or directory exists at the given relative path within the space.
    pub async fn try_exists(&self, relative_path: impl AsRef<Path>) -> Result<bool, SpaceError> {
        let path = self.local_path(relative_path)?;
        Ok(tokio::fs::try_exists(path).await?)
    }

    /// Write data to a file at the given relative path within the space.
    pub async fn write(
        &self,
        relative_path: impl AsRef<Path>,
        contents: impl AsRef<[u8]>,
    ) -> Result<(), SpaceError> {
        let path = self.local_path(relative_path)?;
        Ok(tokio::fs::write(path, contents).await?)
    }
}

impl<T: SpaceContent> From<T> for Space<T> {
    fn from(content: T) -> Self {
        Space::<T>::new(content)
    }
}

impl<T: SpaceContent> AsRef<T> for Space<T> {
    fn as_ref(&self) -> &T {
        &self.content
    }
}

impl<T: SpaceContent> Deref for Space<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.as_ref()
    }
}

pub trait SpaceContent {
    fn get_pattern() -> SpaceRootFindPattern;
}

pub enum SpaceRootFindPattern {
    IncludeDotDir(OsString),
    IncludeFile(OsString),
}

/// Find the space directory containing the current directory,
/// Use Pattern to specify the search method
///
/// For the full implementation, see `find_space_root_with`
pub fn find_space_root(pattern: SpaceRootFindPattern) -> Result<PathBuf, SpaceError> {
    find_space_root_with(current_dir()?, pattern)
}

/// Find the space directory containing the specified directory,
/// Use Pattern to specify the search method
///
/// IncludeDotDir(OsString)
/// - Contains a specific directory, e.g., to find `.git`, use `IncludeDotDir("git".into())`
///
/// IncludeFile(OsString)
/// - Contains a specific file, e.g., to find `Cargo.toml`, use `IncludeFile("Cargo.toml".into())`
///
/// ```rust
/// # use std::env::current_dir;
/// # use std::path::PathBuf;
/// # use framework::space::SpaceRootFindPattern;
/// # use framework::space::find_space_root_with;
/// // Find the `.cargo` directory
/// let path = find_space_root_with(
///     current_dir().unwrap(),
///     SpaceRootFindPattern::IncludeDotDir(
///         "cargo".into()
///     )
/// );
/// assert!(path.is_ok());
/// assert!(path.unwrap().join(".cargo").is_dir())
/// ```
/// ```rust
/// # use std::env::current_dir;
/// # use std::path::PathBuf;
/// # use framework::space::SpaceRootFindPattern;
/// # use framework::space::find_space_root_with;
/// // Find the `Cargo.toml` file
/// let path = find_space_root_with(
///     current_dir().unwrap(),
///     SpaceRootFindPattern::IncludeFile(
///         "Cargo.toml".into()
///     )
/// );
/// assert!(path.is_ok());
/// assert!(path.unwrap().join("Cargo.toml").is_file())
/// ```
pub fn find_space_root_with(
    current_dir: PathBuf,
    pattern: SpaceRootFindPattern,
) -> Result<PathBuf, SpaceError> {
    // Get the pattern used for matching
    let match_pattern: Box<dyn Fn(&Path) -> bool> = match pattern {
        SpaceRootFindPattern::IncludeDotDir(dot_dir_name) => Box::new(move |path| {
            path.join(format!(".{}", dot_dir_name.to_string_lossy()))
                .is_dir()
        }),
        SpaceRootFindPattern::IncludeFile(file_name) => {
            Box::new(move |path| path.join(&file_name).is_file())
        }
    };
    // Match parent directories
    let mut current = current_dir;
    loop {
        if match_pattern(current.as_path()) {
            return Ok(current);
        }
        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            break;
        }
    }
    Err(SpaceError::SpaceNotFound)
}
