use proc_macro::TokenStream;
use quote::quote;
use string_proc::snake_case;
use syn::parse_macro_input;

#[proc_macro_derive(RWDataTest)]
pub fn rw_data_test_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let name = &input.ident;
    let test_mod_name = syn::Ident::new(
        &format!("{}_rw_test", snake_case!(name.to_string())),
        name.span(),
    );

    let expanded = quote! {
        #[cfg(test)]
        mod #test_mod_name {
            use super::*;
            use asset_system::rw::RWData;

            #[tokio::test]
            async fn test() {
                let temp_dir = std::env::current_dir().unwrap().join(".temp/");
                tokio::fs::create_dir_all(&temp_dir)
                    .await
                    .expect("Create dir failed!");
                let temp_file = temp_dir.join(stringify!(#name));

                #name::write(#name::test_data(), &temp_file)
                    .await
                    .expect("Write test data failed!");

                let read_data = #name::read(&temp_file)
                    .await
                    .expect("Re-read data failed!");

                assert!(#name::verify_data(#name::test_data(), read_data));
            }
        }
    };

    TokenStream::from(expanded)
}
