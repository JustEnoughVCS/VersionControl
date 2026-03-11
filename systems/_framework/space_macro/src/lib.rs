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

            #[tokio::test]
            async fn test_create_space() {
                let temp_dir = current_dir().unwrap().join(".temp").join(stringify!(#name));
                remove_dir_all(&temp_dir).await.ok();
                create_dir_all(&temp_dir).await.unwrap();
                set_current_dir(&temp_dir).unwrap();

                let space = Space::new(#name::default());

                assert!(space.space_dir_current().is_err());

                space.init_here().await.unwrap();

                assert!(space.space_dir_current().is_ok());

                let space_dir = space.space_dir_current().unwrap();
                let pattern = <#name as framework::space::SpaceRoot>::get_pattern();

                match pattern {
                    framework::space::SpaceRootFindPattern::IncludeDotDir(dir_name) => {
                        let expected_dir = space_dir.join(dir_name);
                        assert!(expected_dir.exists(), "Space directory {:?} should exist", expected_dir);
                    }
                    framework::space::SpaceRootFindPattern::IncludeFile(file_name) => {
                        let expected_file = space_dir.join(file_name);
                        assert!(expected_file.exists(), "Space file {:?} should exist", expected_file);
                    }
                }
            }
        }
    };

    TokenStream::from(expanded)
}
