use std::path::PathBuf;

/// Format path str
pub fn format_path_str(path: impl Into<String>) -> Result<String, std::io::Error> {
    let path_str = path.into();
    let ends_with_slash = path_str.ends_with('/');

    // ANSI Strip
    let cleaned = strip_ansi_escapes::strip(&path_str);
    let path_without_ansi = String::from_utf8(cleaned)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let path_with_forward_slash = path_without_ansi.replace('\\', "/");
    let mut result = String::new();
    let mut prev_char = '\0';

    for c in path_with_forward_slash.chars() {
        if c == '/' && prev_char == '/' {
            continue;
        }
        result.push(c);
        prev_char = c;
    }

    let unfriendly_chars = ['*', '?', '"', '<', '>', '|'];
    result = result
        .chars()
        .filter(|c| !unfriendly_chars.contains(c))
        .collect();

    // Handle ".." path components
    let path_buf = PathBuf::from(&result);
    let normalized_path = normalize_path(&path_buf);
    result = normalized_path.to_string_lossy().replace('\\', "/");

    // Restore trailing slash if original path had one
    if ends_with_slash && !result.ends_with('/') {
        result.push('/');
    }

    Ok(result)
}

/// Normalize path by resolving ".." components without requiring file system access
fn normalize_path(path: &PathBuf) -> PathBuf {
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

pub fn format_path(path: impl Into<PathBuf>) -> Result<PathBuf, std::io::Error> {
    let path_str = format_path_str(path.into().display().to_string())?;
    Ok(PathBuf::from(path_str))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_path() -> Result<(), std::io::Error> {
        assert_eq!(format_path_str("C:\\Users\\\\test")?, "C:/Users/test");

        assert_eq!(
            format_path_str("/path/with/*unfriendly?chars")?,
            "/path/with/unfriendlychars"
        );

        assert_eq!(format_path_str("\x1b[31m/path\x1b[0m")?, "/path");
        assert_eq!(format_path_str("/home/user/dir/")?, "/home/user/dir/");
        assert_eq!(
            format_path_str("/home/user/file.txt")?,
            "/home/user/file.txt"
        );
        assert_eq!(
            format_path_str("/home/my_user/DOCS/JVCS_TEST/Workspace/../Vault/")?,
            "/home/my_user/DOCS/JVCS_TEST/Vault/"
        );

        Ok(())
    }
}
