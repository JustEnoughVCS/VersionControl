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
                    SpaceRootFindPattern::AbsolutePath(path_buf) => space.set_override_pattern(Some(
                        SpaceRootFindPattern::AbsolutePath(temp_dir.join(path_buf)),
                    )),
                    _ => {}
                }

                assert!(space.space_dir_current().is_err());

                space.init_here().await.unwrap();

                assert!(space.space_dir_current().is_ok());
            }
        }
    };

    TokenStream::from(expanded)
}
