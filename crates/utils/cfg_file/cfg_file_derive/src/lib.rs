extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::ParseStream;
use syn::{Attribute, DeriveInput, Expr, parse_macro_input};

#[proc_macro_derive(ConfigFile, attributes(cfg_file))]
pub fn derive_config_file(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Process 'cfg_file'
    let path_expr = match find_cfg_file_path(&input.attrs) {
        Some(PathExpr::StringLiteral(path)) => {
            if let Some(path_str) = path.strip_prefix("./") {
                quote! {
                    std::env::current_dir()?.join(#path_str)
                }
            } else {
                // Using Absolute Path
                quote! {
                    std::path::PathBuf::from(#path)
                }
            }
        }
        Some(PathExpr::PathExpression(path_expr)) => {
            // For path expressions (constants), generate code that references the constant
            quote! {
                std::path::PathBuf::from(#path_expr)
            }
        }
        None => {
            let default_file = to_snake_case(&name.to_string()) + ".json";
            quote! {
                std::env::current_dir()?.join(#default_file)
            }
        }
    };

    let expanded = quote! {
        impl cfg_file::config::ConfigFile for #name {
            type DataType = #name;

            fn default_path() -> Result<std::path::PathBuf, std::io::Error> {
                Ok(#path_expr)
            }
        }
    };

    TokenStream::from(expanded)
}

enum PathExpr {
    StringLiteral(String),
    PathExpression(syn::Expr),
}

fn find_cfg_file_path(attrs: &[Attribute]) -> Option<PathExpr> {
    for attr in attrs {
        if attr.path().is_ident("cfg_file") {
            let parser = |meta: ParseStream| {
                let path_meta: syn::MetaNameValue = meta.parse()?;
                if path_meta.path.is_ident("path") {
                    match &path_meta.value {
                        // String literal case: path = "./vault.toml"
                        Expr::Lit(expr_lit) if matches!(expr_lit.lit, syn::Lit::Str(_)) => {
                            if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                                return Ok(PathExpr::StringLiteral(lit_str.value()));
                            }
                        }
                        // Path expression case: path = SERVER_FILE_VAULT or crate::constants::SERVER_FILE_VAULT
                        expr @ (Expr::Path(_) | Expr::Macro(_)) => {
                            return Ok(PathExpr::PathExpression(expr.clone()));
                        }
                        _ => {}
                    }
                }
                Err(meta.error("expected `path = \"...\"` or `path = CONSTANT`"))
            };

            if let Ok(path_expr) = attr.parse_args_with(parser) {
                return Some(path_expr);
            }
        }
    }
    None
}

fn to_snake_case(s: &str) -> String {
    let mut snake = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i != 0 {
                snake.push('_');
            }
            snake.push(c.to_ascii_lowercase());
        } else {
            snake.push(c);
        }
    }
    snake
}
