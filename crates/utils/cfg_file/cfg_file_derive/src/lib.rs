extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::ParseStream;
use syn::{
    parse_macro_input,
    DeriveInput,
    Attribute
};

#[proc_macro_derive(ConfigFile, attributes(cfg_file))]
pub fn derive_config_file(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Process 'cfg_file'
    let path_expr = match find_cfg_file_path(&input.attrs) {
        Some(path) => {
            if path.starts_with("./") {
                let path_str = &path[2..];
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

fn find_cfg_file_path(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("cfg_file") {
            let parser = |meta: ParseStream| {
                let path_meta: syn::MetaNameValue = meta.parse()?;
                if path_meta.path.is_ident("path") {
                    if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(lit), .. }) = path_meta.value {
                        return Ok(lit.value());
                    }
                }
                Err(meta.error("expected `path = \"...\"`"))
            };

            if let Ok(path) = attr.parse_args_with(parser) {
                return Some(path);
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