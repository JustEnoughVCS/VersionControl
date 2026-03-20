use just_fmt::fmt_path::fmt_path_str;

use crate::{
    index_source::IndexSource,
    mapping::{LocalMapping, LocalMappingForward, error::ParseMappingError},
};

impl TryFrom<&str> for LocalMapping {
    type Error = ParseMappingError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        // Remove surrounding quotes if present
        let s = s.trim_matches('"');

        // Helper function to remove quotes from a string
        fn remove_quotes(s: &str) -> String {
            // Simply remove all quotes from the string
            s.replace('"', "").trim().to_string()
        }

        // Helper function to split by operator, handling both spaced and non-spaced versions
        fn split_by_operator<'a>(s: &'a str, operator: &'a str) -> Vec<&'a str> {
            let mut result = Vec::new();
            let mut start = 0;

            // Find all occurrences of the operator
            let mut search_from = 0;
            while let Some(pos) = s[search_from..].find(operator) {
                let actual_pos = search_from + pos;
                result.push(&s[start..actual_pos]);
                start = actual_pos + operator.len();
                search_from = start;
            }

            if start < s.len() {
                result.push(&s[start..]);
            }

            result
        }

        // Helper function to find operator position
        fn find_operator<'a>(s: &'a str, operator: &'a str) -> Option<usize> {
            s.find(operator)
        }

        // Try to parse "path" => "source" == "version" pattern
        if let Some(arrow_pos) = find_operator(s, "=>") {
            let after_arrow = &s[arrow_pos + 2..];
            if let Some(equal_pos) = find_operator(after_arrow, "==") {
                // Format: "path" => "source" == "version"
                let path = remove_quotes(s[..arrow_pos].trim());
                let middle_part = after_arrow[..equal_pos].trim();
                let version_part = after_arrow[equal_pos + 2..].trim();

                let middle = remove_quotes(middle_part);
                let version_part_str = remove_quotes(version_part);

                let val = fmt_path_str(path)
                    .map_err(|_| ParseMappingError::InvalidMapping)?
                    .split('/')
                    .map(|s| s.to_string())
                    .collect();

                let source = IndexSource::try_from(middle.as_str())
                    .map_err(|_| ParseMappingError::InvalidMapping)?;

                let version = version_part_str
                    .parse::<u16>()
                    .map_err(|_| ParseMappingError::InvalidMapping)?;

                return Ok(LocalMapping {
                    val,
                    source,
                    forward: LocalMappingForward::Version { version },
                });
            }
        }

        // Split by "=>" to parse the format
        let parts = split_by_operator(s, "=>");

        match parts.len() {
            1 => {
                // Check for "==" operator
                if let Some(equal_pos) = find_operator(s, "==") {
                    // Format: "path" == "source" (when mapped_version equals version)
                    let path_raw = s[..equal_pos].trim();
                    let source_part_raw = s[equal_pos + 2..].trim();
                    let path = remove_quotes(path_raw);
                    let source_str = remove_quotes(source_part_raw);

                    let val = fmt_path_str(path)
                        .map_err(|_| ParseMappingError::InvalidMapping)?
                        .split('/')
                        .map(|s| s.to_string())
                        .collect();

                    let source = IndexSource::try_from(source_str.as_str())
                        .map_err(|_| ParseMappingError::InvalidMapping)?;

                    let version = source.version();

                    Ok(LocalMapping {
                        val,
                        source,
                        forward: LocalMappingForward::Version { version },
                    })
                } else {
                    Err(ParseMappingError::InvalidMapping)
                }
            }
            2 => {
                // Check if the second part contains "=="
                if let Some(equal_pos) = find_operator(parts[1], "==") {
                    // Format: "path" => "source" == "version"
                    let path = remove_quotes(parts[0].trim());
                    let middle_part = parts[1][..equal_pos].trim();
                    let version_part = parts[1][equal_pos + 2..].trim();

                    let middle = remove_quotes(middle_part);
                    let version_part_str = remove_quotes(version_part);

                    let val = fmt_path_str(path)
                        .map_err(|_| ParseMappingError::InvalidMapping)?
                        .split('/')
                        .map(|s| s.to_string())
                        .collect();

                    let source = IndexSource::try_from(middle.as_str())
                        .map_err(|_| ParseMappingError::InvalidMapping)?;

                    let version = version_part_str
                        .parse::<u16>()
                        .map_err(|_| ParseMappingError::InvalidMapping)?;

                    return Ok(LocalMapping {
                        val,
                        source,
                        forward: LocalMappingForward::Version { version },
                    });
                }

                // Format: "path" => "source"
                let path = remove_quotes(parts[0].trim());
                let source_str = remove_quotes(parts[1].trim());

                let val = fmt_path_str(path)
                    .map_err(|_| ParseMappingError::InvalidMapping)?
                    .split('/')
                    .map(|s| s.to_string())
                    .collect();

                let source = IndexSource::try_from(source_str.as_str())
                    .map_err(|_| ParseMappingError::InvalidMapping)?;

                Ok(LocalMapping {
                    val,
                    source,
                    forward: LocalMappingForward::Latest,
                })
            }
            3 => {
                // Format: "path" => "source" => "sheet_name"
                let path = remove_quotes(parts[0].trim());
                let source_str = remove_quotes(parts[1].trim());
                let sheet_name = remove_quotes(parts[2].trim());

                let val = fmt_path_str(path)
                    .map_err(|_| ParseMappingError::InvalidMapping)?
                    .split('/')
                    .map(|s| s.to_string())
                    .collect();

                let source = IndexSource::try_from(source_str.as_str())
                    .map_err(|_| ParseMappingError::InvalidMapping)?;

                Ok(LocalMapping {
                    val,
                    source,
                    forward: LocalMappingForward::Ref { sheet_name },
                })
            }
            _ => Err(ParseMappingError::InvalidMapping),
        }
    }
}
