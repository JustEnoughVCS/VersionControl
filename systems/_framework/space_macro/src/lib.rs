use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(SpaceRootTest)]
pub fn space_root_test_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;

    let test_mod_name = syn::Ident::new(
        &format!(
            "test_{}_space_root",
            just_fmt::snake_case!(name.to_string())
        ),
        name.span(),
    );

    let expanded = quote! {
        #[cfg(test)]
        mod #test_mod_name {
            use super::*;
            use framework::space::Space;
            use std::env::{current_dir, set_current_dir};
            use tokio::fs::{create_dir_all, remove_dir_all};
            use framework::space::SpaceRootFindPattern;

            #[tokio::test]
            async fn test_create_space() {
                let temp_dir = current_dir().unwrap().join(".temp").join(stringify!(#name));
                remove_dir_all(&temp_dir).await.ok();
                create_dir_all(&temp_dir).await.unwrap();
                set_current_dir(&temp_dir).unwrap();

                let mut space = Space::new(#name::default());

                match #name::get_pattern() {
                    SpaceRootFindPattern::AbsolutePath(path_buf) => {
                        let dir = temp_dir.join("root");
                        println!("Redirect absolute path:");
                        println!("  from: `{}`", path_buf.display());
                        println!("    to: `{}`", dir.display());
                        space.set_override_pattern(Some(
                            SpaceRootFindPattern::AbsolutePath(dir.clone()),
                        ));

                        println!("Checking if {} absolute directory does not exist before initialization", stringify!(#name));
                        assert!(!dir.exists());

                        space.init_here().await.unwrap();

                        println!("Checking if {} absolute directory exists after initialization", stringify!(#name));
                        assert!(dir.exists());
                        println!("\u{001b}[33;1mwarning\u{001b}[0m: Absolute path test completed in isolated environment, may not fully represent system runtime conditions");

                        return;
                    },
                    _ => {}
                }

                println!("Checking if {} does not exist before initialization", stringify!(#name));
                assert!(space.space_dir_current().is_err());

                space.init_here().await.unwrap();

                println!("Checking if {} exists after initialization", stringify!(#name));
                assert!(space.space_dir_current().is_ok());
            }
        }
    };

    TokenStream::from(expanded)
}
