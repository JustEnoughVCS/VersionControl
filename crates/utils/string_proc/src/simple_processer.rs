/// Sanitizes a file path by replacing special characters with underscores.
///
/// This function takes a file path as input and returns a sanitized version
/// where characters that are not allowed in file paths (such as path separators
/// and other reserved characters) are replaced with underscores.
pub fn sanitize_file_path<P: AsRef<str>>(path: P) -> String {
    let path_str = path.as_ref();
    path_str
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}
