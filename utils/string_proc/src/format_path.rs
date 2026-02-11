use std::path::{Path, PathBuf};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct PathFormatConfig {
    pub strip_ansi: bool,
    pub strip_unfriendly_chars: bool,
    pub resolve_parent_dirs: bool,
    pub collapse_consecutive_slashes: bool,
    pub escape_backslashes: bool,
}

impl Default for PathFormatConfig {
    fn default() -> Self {
        Self {
            strip_ansi: true,
            strip_unfriendly_chars: true,
            resolve_parent_dirs: true,
            collapse_consecutive_slashes: true,
            escape_backslashes: true,
        }
    }
}

/// Normalize an input path string into a canonical, platform‑agnostic form.
///
/// This function removes ANSI escape sequences, unifies separators to `/`,
/// collapses duplicate slashes, strips unfriendly characters (`*`, `?`, `"`, `<`, `>`, `|`),
/// resolves simple `..` components, and preserves a trailing slash when present.
///
/// See examples below for the exact normalization behavior.
///
/// # Examples
///
/// ```
/// # use string_proc::format_path::format_path_str;
/// use std::io::Error;
///
/// # fn main() -> Result<(), Error> {
/// assert_eq!(format_path_str("C:\\Users\\\\test")?, "C:/Users/test");
/// assert_eq!(
///     format_path_str("/path/with/*unfriendly?chars")?,
///     "/path/with/unfriendlychars"
/// );
/// assert_eq!(format_path_str("\x1b[31m/path\x1b[0m")?, "/path");
/// assert_eq!(format_path_str("/home/user/dir/")?, "/home/user/dir/");
/// assert_eq!(
///     format_path_str("/home/user/file.txt")?,
///     "/home/user/file.txt"
/// );
/// assert_eq!(
///     format_path_str("/home/my_user/DOCS/JVCS_TEST/Workspace/../Vault/")?,
///     "/home/my_user/DOCS/JVCS_TEST/Vault/"
/// );
/// assert_eq!(format_path_str("./home/file.txt")?, "home/file.txt");
/// assert_eq!(format_path_str("./home/path/")?, "home/path/");
/// assert_eq!(format_path_str("./")?, "");
/// # Ok(())
/// # }
/// ```
pub fn format_path_str(path: impl Into<String>) -> Result<String, std::io::Error> {
    format_path_str_with_config(path, &PathFormatConfig::default())
}

/// Normalize an input path string into a canonical, platform‑agnostic form.
///
/// This function removes ANSI escape sequences, unifies separators to `/`,
/// collapses duplicate slashes, strips unfriendly characters (`*`, `?`, `"`, `<`, `>`, `|`),
/// resolves simple `..` components, and preserves a trailing slash when present.
///
/// Unlike `format_path_str`,
/// this method uses `PathFormatConfig` to precisely control
/// what should be processed
pub fn format_path_str_with_config(
    path: impl Into<String>,
    config: &PathFormatConfig,
) -> Result<String, std::io::Error> {
    let path_str = path.into();
    let ends_with_slash = path_str.ends_with('/');

    // ANSI Strip
    let cleaned = if config.strip_ansi {
        strip_ansi_escapes::strip(&path_str)
    } else {
        path_str.as_bytes().to_vec()
    };
    let path_without_ansi = String::from_utf8(cleaned)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let path_with_forward_slash = if config.escape_backslashes {
        path_without_ansi.replace('\\', "/")
    } else {
        path_without_ansi
    };
    let mut result = String::new();
    let mut prev_char = '\0';

    for c in path_with_forward_slash.chars() {
        if config.collapse_consecutive_slashes && c == '/' && prev_char == '/' {
            continue;
        }
        result.push(c);
        prev_char = c;
    }

    if config.strip_unfriendly_chars {
        let unfriendly_chars = ['*', '?', '"', '<', '>', '|'];
        result = result
            .chars()
            .filter(|c| !unfriendly_chars.contains(c))
            .collect();
    }

    // Handle ".." path components
    let path_buf = PathBuf::from(&result);
    let normalized_path = if config.resolve_parent_dirs {
        normalize_path(&path_buf)
    } else {
        path_buf
    };
    result = normalized_path.to_string_lossy().replace('\\', "/");

    // Restore trailing slash if original path had one
    if ends_with_slash && !result.ends_with('/') {
        result.push('/');
    }

    // Special case: when result is only "./", return ""
    if result == "./" {
        return Ok(String::new());
    }

    Ok(result)
}

/// Normalize path by resolving ".." components without requiring file system access
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                if !components.is_empty() {
                    components.pop();
                }
            }
            std::path::Component::CurDir => {
                // Skip current directory components
            }
            _ => {
                components.push(component);
            }
        }
    }

    if components.is_empty() {
        PathBuf::from(".")
    } else {
        components.iter().collect()
    }
}

/// Format a [`PathBuf`] into its canonical string form and convert it back.
///
/// This is a convenience wrapper around [`format_path_str`], preserving
/// the semantics of [`PathBuf`] while applying the same normalization rules:
/// - normalize separators to `/`
/// - remove duplicated separators
/// - strip ANSI escape sequences
/// - remove unfriendly characters (`*`, `?`, etc.)
/// - resolve simple `..` segments
pub fn format_path(path: impl Into<PathBuf>) -> Result<PathBuf, std::io::Error> {
    let path_str = format_path_str(path.into().display().to_string())?;
    Ok(PathBuf::from(path_str))
}

/// Format a [`PathBuf`] into its canonical string form and convert it back.
///
/// Unlike `format_path`,
/// this method uses `PathFormatConfig` to precisely control
/// what should be processed
pub fn format_path_with_config(
    path: impl Into<PathBuf>,
    config: &PathFormatConfig,
) -> Result<PathBuf, std::io::Error> {
    let path_str = format_path_str_with_config(path.into().display().to_string(), config)?;
    Ok(PathBuf::from(path_str))
}
