use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::parse_str;

const INDEX_SOURCE: &str = "just_enough_vcs::system::sheet_system::index_source::IndexSource";

const LOCAL_MAPPING_PATH: &str = "just_enough_vcs::system::sheet_system::mapping::LocalMapping";

const MAPPING_BUF_PATH: &str = "just_enough_vcs::system::sheet_system::mapping::MappingBuf";
const MAPPING_PATH: &str = "just_enough_vcs::system::sheet_system::mapping::Mapping";

const LOCAL_MAPPING_FORWARD_PATH: &str =
    "just_enough_vcs::system::sheet_system::mapping::LocalMappingForward";

/// Parse strings in the format "sheet:/path"
fn parse_sheet_path(input: &str) -> Result<(String, Vec<String>), String> {
    let parts: Vec<&str> = input.split(":/").collect();
    if parts.len() != 2 {
        return Err(format!(
            "Invalid sheet path syntax. Expected: sheet:/path, got: {}",
            input
        ));
    }

    let sheet = parts[0].to_string();
    let path = parts[1];

    if path.is_empty() {
        return Err("Path cannot be empty".to_string());
    }

    let path_parts: Vec<String> = path.split('/').map(|s| s.to_string()).collect();

    Ok((sheet, path_parts))
}

/// Parse strings in the format "id/ver" or "~id/ver"
/// Returns (remote, id, ver)
fn parse_id_version(input: &str) -> Result<(bool, u32, u16), String> {
    let trimmed = input.trim();

    // Check if it starts with ~ for local
    let (remote, id_part) = if trimmed.starts_with('~') {
        (false, &trimmed[1..])
    } else {
        (true, trimmed)
    };

    let parts: Vec<&str> = id_part.split('/').collect();
    if parts.len() != 2 {
        return Err(format!(
            "Invalid id/version syntax. Expected: id/ver or ~id/ver, got: {}",
            input
        ));
    }

    let id_str = parts[0].trim();
    let ver_str = parts[1].trim();

    if id_str.is_empty() {
        return Err("ID cannot be empty".to_string());
    }
    if ver_str.is_empty() {
        return Err("Version cannot be empty".to_string());
    }

    let id = id_str
        .parse::<u32>()
        .map_err(|e| format!("Failed to parse id as u32: {}", e))?;
    let ver = ver_str
        .parse::<u16>()
        .map_err(|e| format!("Failed to parse version as u16: {}", e))?;

    Ok((remote, id, ver))
}

/// Parse a path string into a vector of strings
fn parse_path_string(input: &str) -> Vec<String> {
    input.split('/').map(|s| s.trim().to_string()).collect()
}

/// Generate token stream for path vector
fn path_vec_to_tokens(path_vec: &[String]) -> TokenStream2 {
    let path_items: Vec<_> = path_vec.iter().map(|s| quote! { #s.to_string() }).collect();

    quote! { vec![#(#path_items),*] }
}

/// Create a MappingBuf
///
/// Use the following syntax to create a MappingBuf
/// ```ignore
/// let mapping_buf = mapping_buf!(
///     // Map the `version` of index `index_id`
///     // to `your_dir/your_file.suffix` in `your_sheet`
///     "your_sheet:/your_dir/your_file.suffix" => "index_id/version"
/// );
/// ```
#[proc_macro]
pub fn mapping_buf(input: TokenStream) -> TokenStream {
    let input_str = input.to_string();
    let parts: Vec<&str> = input_str.split("=>").collect();

    if parts.len() != 2 {
        return syn::Error::new(
            Span::call_site(),
            "Invalid mapping_buf syntax. Expected: mapping_buf!(\"sheet:/path\" => \"id/ver\")",
        )
        .to_compile_error()
        .into();
    }

    let left = parts[0].trim().trim_matches('"').trim();
    let right = parts[1].trim().trim_matches('"').trim();

    let (sheet, path_vec) = match parse_sheet_path(left) {
        Ok(result) => result,
        Err(err) => {
            return syn::Error::new(Span::call_site(), err)
                .to_compile_error()
                .into();
        }
    };

    let (remote, id, ver) = match parse_id_version(right) {
        Ok(result) => result,
        Err(err) => {
            return syn::Error::new(Span::call_site(), err)
                .to_compile_error()
                .into();
        }
    };

    let path_vec_tokens = path_vec_to_tokens(&path_vec);

    let mapping_buf_path: syn::Path =
        parse_str(MAPPING_BUF_PATH).expect("Failed to parse MAPPING_BUF_PATH");
    let index_source_path: syn::Path =
        parse_str(INDEX_SOURCE).expect("Failed to parse INDEX_SOURCE");

    let expanded = quote! {
        #mapping_buf_path::new(
            #sheet.to_string(),
            #path_vec_tokens,
            #index_source_path::new(#remote, #id, #ver)
        )
    };

    expanded.into()
}

/// Create a Mapping
///
/// Use the following syntax to create a Mapping
/// ```ignore
/// let mapping = mapping!(
///     // Map the `version` of index `index_id`
///     // to `your_dir/your_file.suffix` in `your_sheet`
///     "your_sheet:/your_dir/your_file.suffix" => "id/ver"
/// );
/// ```
#[proc_macro]
pub fn mapping(input: TokenStream) -> TokenStream {
    let input_str = input.to_string();
    let parts: Vec<&str> = input_str.split("=>").collect();

    if parts.len() != 2 {
        return syn::Error::new(
            Span::call_site(),
            "Invalid mapping syntax. Expected: mapping!(\"sheet:/path\" => \"id/ver\")",
        )
        .to_compile_error()
        .into();
    }

    let left = parts[0].trim().trim_matches('"').trim();
    let right = parts[1].trim().trim_matches('"').trim();

    let (sheet, path_vec) = match parse_sheet_path(left) {
        Ok(result) => result,
        Err(err) => {
            return syn::Error::new(Span::call_site(), err)
                .to_compile_error()
                .into();
        }
    };

    let (remote, id, ver) = match parse_id_version(right) {
        Ok(result) => result,
        Err(err) => {
            return syn::Error::new(Span::call_site(), err)
                .to_compile_error()
                .into();
        }
    };

    let path = path_vec.join("/");

    let mapping_path: syn::Path = parse_str(MAPPING_PATH).expect("Failed to parse MAPPING_PATH");
    let index_source_path: syn::Path =
        parse_str(INDEX_SOURCE).expect("Failed to parse INDEX_SOURCE");

    let expanded = quote! {
        #mapping_path::new(
            #sheet,
            #path,
            #index_source_path::new(#remote, #id, #ver)
        )
    };

    expanded.into()
}

enum LocalMappingParts {
    Latest(String, bool, u32, u16),
    Version(String, bool, u32, u16),
    WithRef(String, bool, u32, u16, String),
    VersionForward(String, bool, u32, u16, u16),
}

impl LocalMappingParts {
    fn parse(input: TokenStream) -> Result<Self, syn::Error> {
        let input_str = input.to_string();

        // LocalMapping does not have a sheet_name definition
        // So when the user specifies a sheet prefix, an error should be reported
        if input_str.contains(":/") {
            return Err(syn::Error::new(
                Span::call_site(),
                "local_mapping is not related to sheets. Do not use 'sheet:/' prefix.",
            ));
        }

        // Check for "=>" followed by "==" syntax: "path" => "id/ver" == "ver2"
        if input_str.contains("=>") && input_str.contains("==") {
            // Count occurrences to determine the pattern
            let arrow_count = input_str.matches("=>").count();
            let equal_count = input_str.matches("==").count();

            if arrow_count == 1 && equal_count == 1 {
                // Try to parse as "path" => "id/ver" == "ver2"
                let parts: Vec<&str> = input_str.split("=>").collect();
                if parts.len() == 2 {
                    let left = parts[0].trim().trim_matches('"').trim();
                    let right_part = parts[1].trim();

                    // Split the right part by "=="
                    let right_parts: Vec<&str> = right_part.split("==").collect();
                    if right_parts.len() == 2 {
                        let middle = right_parts[0].trim().trim_matches('"').trim();
                        let version_str = right_parts[1].trim().trim_matches('"').trim();

                        let (remote, id, ver) = parse_id_version(middle)
                            .map_err(|err| syn::Error::new(Span::call_site(), err))?;

                        let target_ver = version_str.parse::<u16>().map_err(|err| {
                            syn::Error::new(
                                Span::call_site(),
                                format!("Failed to parse target version as u16: {}", err),
                            )
                        })?;

                        return Ok(LocalMappingParts::VersionForward(
                            left.to_string(),
                            remote,
                            id,
                            ver,
                            target_ver,
                        ));
                    }
                }
            }
        }

        // When both "==" and "=>" appear but not in the expected pattern
        // It's impossible to determine whether to match the current version or point to a Ref
        // Should report an error
        if input_str.contains("==") && input_str.contains("=>") {
            return Err(syn::Error::new(
                Span::call_site(),
                "Ambiguous forward direction. Use either '==' for version or '=>' for ref, or use 'path' => 'id/ver' == 'ver2' syntax.",
            ));
        }

        if input_str.contains("==") {
            let parts: Vec<&str> = input_str.split("==").collect();
            if parts.len() != 2 {
                return Err(syn::Error::new(
                    Span::call_site(),
                    "Invalid local_mapping syntax with '=='. Expected: local_mapping!(\"path\" == \"id/ver\")",
                ));
            }

            let left = parts[0].trim().trim_matches('"').trim();
            let right = parts[1].trim().trim_matches('"').trim();

            let (remote, id, ver) =
                parse_id_version(right).map_err(|err| syn::Error::new(Span::call_site(), err))?;

            return Ok(LocalMappingParts::Version(
                left.to_string(),
                remote,
                id,
                ver,
            ));
        }

        let parts: Vec<&str> = input_str.split("=>").collect();

        match parts.len() {
            2 => {
                // local_mapping!("path" => "id/ver") - Latest
                let left = parts[0].trim().trim_matches('"').trim();
                let right = parts[1].trim().trim_matches('"').trim();

                let (remote, id, ver) = parse_id_version(right)
                    .map_err(|err| syn::Error::new(Span::call_site(), err))?;

                Ok(LocalMappingParts::Latest(left.to_string(), remote, id, ver))
            }
            3 => {
                // Check if the third part is a ref (string) or a version number (u16)
                let left = parts[0].trim().trim_matches('"').trim();
                let middle = parts[1].trim().trim_matches('"').trim();
                let right = parts[2].trim().trim_matches('"').trim();

                let (remote, id, ver) = parse_id_version(middle)
                    .map_err(|err| syn::Error::new(Span::call_site(), err))?;

                // Try to parse right as u16 (version number)
                if let Ok(target_ver) = right.parse::<u16>() {
                    // This is "path" => "id/ver" => "ver2" syntax
                    Ok(LocalMappingParts::VersionForward(
                        left.to_string(),
                        remote,
                        id,
                        ver,
                        target_ver,
                    ))
                } else {
                    // This is "path" => "id/ver" => "ref" syntax
                    Ok(LocalMappingParts::WithRef(
                        left.to_string(),
                        remote,
                        id,
                        ver,
                        right.to_string(),
                    ))
                }
            }
            _ => Err(syn::Error::new(
                Span::call_site(),
                "Invalid local_mapping syntax. Expected: local_mapping!(\"path\" => \"id/ver\") or local_mapping!(\"path\" == \"id/ver\") or local_mapping!(\"path\" => \"id/ver\" => \"ref\") or local_mapping!(\"path\" => \"id/ver\" => \"ver2\") or local_mapping!(\"path\" => \"id/ver\" == \"ver2\")",
            )),
        }
    }
}

/// Create a LocalMapping
///
/// Use the following syntax to create a LocalMapping
/// ```ignore
/// let lcoal_mapping_to_latest = local_mapping!(
///     // Map the `version` of index `index_id`
///     // to `your_dir/your_file.suffix`
///     // and expects to keep the latest version
///     "your_dir/your_file.suffix" => "index_id/version"
/// );
///
/// let lcoal_mapping_to_version = local_mapping!(
///     // Map the `version` of index `index_id`
///     // to `your_dir/your_file.suffix`
///     // and expects to keep the current version
///     "your_dir/your_file.suffix" == "index_id/version"
/// );
///
/// let lcoal_mapping_latest = local_mapping!(
///     // Map the `version` of index `index_id`
///     // to `your_dir/your_file.suffix`
///     // and expects to match the version declared in `ref`
///     "your_dir/your_file.suffix" => "index_id/version" => "ref"
/// );
///
/// let lcoal_mapping_version_forward = local_mapping!(
///     // Map the `version` of index `index_id`
///     // to `your_dir/your_file.suffix`
///     // but expects to point to a specific version `ver2`
///     "your_dir/your_file.suffix" => "index_id/version" => "ver2"
/// );
///
/// let lcoal_mapping_version_forward_alt = local_mapping!(
///     // Alternative syntax for the same behavior
///     "your_dir/your_file.suffix" => "index_id/version" == "ver2"
/// );
/// ```
#[proc_macro]
pub fn local_mapping(input: TokenStream) -> TokenStream {
    let parts = match LocalMappingParts::parse(input) {
        Ok(parts) => parts,
        Err(err) => return err.to_compile_error().into(),
    };

    let local_mapping_path: syn::Path =
        parse_str(LOCAL_MAPPING_PATH).expect("Failed to parse LOCAL_MAPPING_PATH");
    let local_mapping_forward_path: syn::Path =
        parse_str(LOCAL_MAPPING_FORWARD_PATH).expect("Failed to parse LOCAL_MAPPING_FORWARD_PATH");
    let index_source_path: syn::Path =
        parse_str(INDEX_SOURCE).expect("Failed to parse INDEX_SOURCE");

    match parts {
        LocalMappingParts::Latest(path_str, remote, id, ver) => {
            let path_vec = parse_path_string(&path_str);
            let path_vec_tokens = path_vec_to_tokens(&path_vec);

            let expanded = quote! {
                #local_mapping_path::new(
                    #path_vec_tokens,
                    #index_source_path::new(#remote, #id, #ver),
                    #local_mapping_forward_path::Latest
                )
            };

            expanded.into()
        }
        LocalMappingParts::Version(path_str, remote, id, ver) => {
            let path_vec = parse_path_string(&path_str);
            let path_vec_tokens = path_vec_to_tokens(&path_vec);

            let expanded = quote! {
                #local_mapping_path::new(
                    #path_vec_tokens,
                    #index_source_path::new(#remote, #id, #ver),
                    #local_mapping_forward_path::Version {
                        version: #ver
                    }
                )
            };

            expanded.into()
        }
        LocalMappingParts::WithRef(path_str, remote, id, ver, ref_name) => {
            let path_vec = parse_path_string(&path_str);
            let path_vec_tokens = path_vec_to_tokens(&path_vec);

            let expanded = quote! {
                #local_mapping_path::new(
                    #path_vec_tokens,
                    #index_source_path::new(#remote, #id, #ver),
                    #local_mapping_forward_path::Ref {
                        sheet_name: #ref_name.to_string()
                    }
                )
            };

            expanded.into()
        }
        LocalMappingParts::VersionForward(path_str, remote, id, ver, target_ver) => {
            let path_vec = parse_path_string(&path_str);
            let path_vec_tokens = path_vec_to_tokens(&path_vec);

            let expanded = quote! {
                #local_mapping_path::new(
                    #path_vec_tokens,
                    #index_source_path::new(#remote, #id, #ver),
                    #local_mapping_forward_path::Version {
                        version: #target_ver
                    }
                )
            };

            expanded.into()
        }
    }
}
