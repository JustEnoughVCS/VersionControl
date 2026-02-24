use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::parse_str;

const INDEX_SOURCE_BUF: &str =
    "just_enough_vcs::system::sheet_system::index_source::IndexSourceBuf";
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

/// Parse strings in the format "id/ver"
fn parse_id_version(input: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = input.split('/').collect();
    if parts.len() != 2 {
        return Err(format!(
            "Invalid id/version syntax. Expected: id/ver, got: {}",
            input
        ));
    }

    let id = parts[0].trim().to_string();
    let ver = parts[1].trim().to_string();

    if id.is_empty() {
        return Err("ID cannot be empty".to_string());
    }
    if ver.is_empty() {
        return Err("Version cannot be empty".to_string());
    }

    Ok((id, ver))
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

    let (id, ver) = match parse_id_version(right) {
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
    let index_source_buf_path: syn::Path =
        parse_str(INDEX_SOURCE_BUF).expect("Failed to parse INDEX_SOURCE_BUF");

    let expanded = quote! {
        #mapping_buf_path::new(
            #sheet.to_string(),
            #path_vec_tokens,
            #index_source_buf_path::new(#id.to_string(), #ver.to_string())
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

    let (id, ver) = match parse_id_version(right) {
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
            #index_source_path::new(#id, #ver)
        )
    };

    expanded.into()
}

enum LocalMappingParts {
    Latest(String, String, String),
    Version(String, String, String),
    WithRef(String, String, String, String),
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

        // When both "==" and "=>" appear
        // It's impossible to determine whether to match the current version or point to a Ref
        // Should report an error
        if input_str.contains("==") && input_str.contains("=>") {
            return Err(syn::Error::new(
                Span::call_site(),
                "Ambiguous forward direction. Use either '==' for version or '=>' for ref, not both.",
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

            let (id, ver) =
                parse_id_version(right).map_err(|err| syn::Error::new(Span::call_site(), err))?;

            return Ok(LocalMappingParts::Version(left.to_string(), id, ver));
        }

        let parts: Vec<&str> = input_str.split("=>").collect();

        match parts.len() {
            2 => {
                // local_mapping!("path" => "id/ver") - Latest
                let left = parts[0].trim().trim_matches('"').trim();
                let right = parts[1].trim().trim_matches('"').trim();

                let (id, ver) = parse_id_version(right)
                    .map_err(|err| syn::Error::new(Span::call_site(), err))?;

                Ok(LocalMappingParts::Latest(left.to_string(), id, ver))
            }
            3 => {
                // local_mapping!("path" => "id/ver" => "ref") - Ref
                let left = parts[0].trim().trim_matches('"').trim();
                let middle = parts[1].trim().trim_matches('"').trim();
                let right = parts[2].trim().trim_matches('"').trim();

                let (id, ver) = parse_id_version(middle)
                    .map_err(|err| syn::Error::new(Span::call_site(), err))?;

                Ok(LocalMappingParts::WithRef(
                    left.to_string(),
                    id,
                    ver,
                    right.to_string(),
                ))
            }
            _ => Err(syn::Error::new(
                Span::call_site(),
                "Invalid local_mapping syntax. Expected: local_mapping!(\"path\" => \"id/ver\") or local_mapping!(\"path\" == \"id/ver\") or local_mapping!(\"path\" => \"id/ver\" => \"ref\")",
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
    let index_source_buf_path: syn::Path =
        parse_str(INDEX_SOURCE_BUF).expect("Failed to parse INDEX_SOURCE_BUF");

    match parts {
        LocalMappingParts::Latest(path_str, id, ver) => {
            let path_vec = parse_path_string(&path_str);
            let path_vec_tokens = path_vec_to_tokens(&path_vec);

            let expanded = quote! {
                #local_mapping_path::new(
                    #path_vec_tokens,
                    #index_source_buf_path::new(#id.to_string(), #ver.to_string()),
                    #local_mapping_forward_path::Latest
                )
            };

            expanded.into()
        }
        LocalMappingParts::Version(path_str, id, ver) => {
            let path_vec = parse_path_string(&path_str);
            let path_vec_tokens = path_vec_to_tokens(&path_vec);

            let expanded = quote! {
                #local_mapping_path::new(
                    #path_vec_tokens,
                    #index_source_buf_path::new(#id.to_string(), #ver.to_string()),
                    #local_mapping_forward_path::Version {
                        version_name: #ver.to_string()
                    }
                )
            };

            expanded.into()
        }
        LocalMappingParts::WithRef(path_str, id, ver, ref_name) => {
            let path_vec = parse_path_string(&path_str);
            let path_vec_tokens = path_vec_to_tokens(&path_vec);

            let expanded = quote! {
                #local_mapping_path::new(
                    #path_vec_tokens,
                    #index_source_buf_path::new(#id.to_string(), #ver.to_string()),
                    #local_mapping_forward_path::Ref {
                        sheet_name: #ref_name.to_string()
                    }
                )
            };

            expanded.into()
        }
    }
}
