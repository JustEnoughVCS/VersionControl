use just_fmt::pascal_case;
use proc_macro::TokenStream;
use quote::quote;
use syn::{Expr, ExprLit, Ident, Item, ItemMod, Lit, LitStr, parse_macro_input};

#[proc_macro_attribute]
pub fn constants(attr: TokenStream, item: TokenStream) -> TokenStream {
    let prefix = parse_macro_input!(attr as LitStr).value();
    let mut input_mod = parse_macro_input!(item as ItemMod);

    if let Some((_, items)) = &mut input_mod.content {
        // Collect functions to generate
        let mut generated_functions = Vec::new();
        let mut generated_ffi_functions = Vec::new();

        for item in items.iter_mut() {
            if let Item::Macro(macro_item) = item {
                // Check if it's a c! macro call
                if macro_item.mac.path.is_ident("c") {
                    // Parse macro content
                    let macro_tokens = macro_item.mac.tokens.clone();
                    let parsed: syn::Result<Expr> = syn::parse2(macro_tokens);

                    if let Ok(Expr::Assign(assign)) = parsed {
                        // Get constant name and value
                        if let Some((const_name, const_value)) =
                            extract_const_name_and_value(&assign)
                        {
                            // Process constant and generate functions
                            if let Some((rust_func, ffi_func)) =
                                process_constant(&prefix, const_name, const_value)
                            {
                                generated_functions.push(rust_func);
                                generated_ffi_functions.push(ffi_func);
                            }
                        }
                    }
                }
            }
        }

        for func in generated_functions {
            items.push(Item::Verbatim(func));
        }
        for func in generated_ffi_functions {
            items.push(Item::Verbatim(func));
        }
    }

    TokenStream::from(quote! { #input_mod })
}

fn extract_const_name_and_value(assign: &syn::ExprAssign) -> Option<(Ident, Box<Expr>)> {
    if let Expr::Path(path) = &*assign.left {
        if let Some(ident) = path.path.get_ident() {
            let const_name = ident.clone();
            let const_value = assign.right.clone();
            return Some((const_name, const_value));
        }
    }
    None
}

fn process_constant(
    prefix: &str,
    const_name: Ident,
    const_value: Box<Expr>,
) -> Option<(proc_macro2::TokenStream, proc_macro2::TokenStream)> {
    if let Expr::Lit(ExprLit {
        lit: Lit::Str(lit_str),
        ..
    }) = *const_value
    {
        let value_str = lit_str.value();
        let value_span = lit_str.span();

        if value_str.contains('{') && value_str.contains('}') {
            let params = extract_placeholder_params(&value_str);

            if !params.is_empty() {
                // With parameters
                Some(generate_functions_with_params(
                    prefix,
                    &const_name,
                    &value_str,
                    &params,
                    value_span,
                ))
            } else {
                // Without parameters
                Some(generate_functions_without_params(
                    prefix,
                    &const_name,
                    &value_str,
                ))
            }
        } else {
            // Without parameters
            Some(generate_functions_without_params(
                prefix,
                &const_name,
                &value_str,
            ))
        }
    } else {
        None
    }
}

fn extract_placeholder_params(value_str: &str) -> Vec<String> {
    let mut params = Vec::new();
    let chars = value_str.chars().collect::<Vec<_>>();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '{' {
            let start = i;
            i += 1;

            while i < chars.len() && chars[i] != '}' {
                i += 1;
            }

            if i < chars.len() && chars[i] == '}' {
                let param: String = chars[start + 1..i].iter().collect();
                if !param.is_empty() {
                    params.push(param);
                }
            }
        }
        i += 1;
    }

    params
}

fn generate_functions_with_params(
    prefix: &str,
    const_name: &Ident,
    value_str: &str,
    params: &[String],
    value_span: proc_macro2::Span,
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    let fn_name = format!("{}_{}", prefix, const_name.to_string().to_lowercase());
    let fn_ident = Ident::new(&fn_name, const_name.span());

    // Generate parameter list
    let param_idents: Vec<Ident> = params.iter().map(|p| Ident::new(p, value_span)).collect();
    let format_args = param_idents.iter().map(|ident| quote! { #ident });

    // Generate format! code
    let doc_format_value = params.join(", ");

    let mut doc_format_template = value_str.to_string();
    for param in params {
        let placeholder = format!("{{{}}}", param);
        doc_format_template = doc_format_template.replace(&placeholder, "{}");
    }

    let doc_format_code = format!(
        "format!(\"{}\", {});",
        doc_format_template, doc_format_value
    );

    // Replace {xxx} with {} format
    let mut format_str = value_str.to_string();
    for param in params {
        format_str = format_str.replace(&format!("{{{}}}", param), "{}");
    }

    // Generate Rust function
    let rust_func = quote! {
        #[doc = "```ignore"]
        #[doc = #doc_format_code]
        #[doc = "```"]
        pub fn #fn_ident(#(#param_idents: &str),*) -> String {
            format!(#format_str, #(#format_args),*)
        }
    };

    // Generate FFI function
    let pascal_name = pascal_case!(const_name.to_string());
    let pascal_prefix = pascal_case!(prefix);
    let ffi_fn_name = format!("JV_Const_{}{}", pascal_prefix, pascal_name);
    let ffi_fn_ident = Ident::new(&ffi_fn_name, const_name.span());

    // Generate parameter list for FFI
    let ffi_param_idents: Vec<Ident> = (0..params.len())
        .map(|i| Ident::new(&format!("p{}", i), value_span))
        .collect();

    let ffi_param_decls = ffi_param_idents.iter().map(|ident| {
        quote! { #ident: *const libc::c_char }
    });

    let ffi_param_checks = ffi_param_idents.iter().map(|ident| {
        quote! { if #ident.is_null() { return std::ptr::null_mut(); } }
    });

    let ffi_param_conversions = ffi_param_idents.iter().map(|ident| {
        quote! {
            let #ident = match std::ffi::CStr::from_ptr(#ident).to_str() {
                Ok(s) => s,
                Err(_) => return std::ptr::null_mut(),
            };
        }
    });

    let ffi_format_args = ffi_param_idents.iter().map(|ident| quote! { #ident });

    let ffi_func = quote! {
        #[unsafe(no_mangle)]
        #[allow(nonstandard_style)]
        pub extern "C" fn #ffi_fn_ident(#(#ffi_param_decls),*) -> *mut libc::c_char {
            unsafe {
                #(#ffi_param_checks)*

                #(#ffi_param_conversions)*

                let result = format!(#format_str, #(#ffi_format_args),*);

                match std::ffi::CString::new(result) {
                    Ok(c) => c.into_raw(),
                    Err(_) => std::ptr::null_mut(),
                }
            }
        }
    };

    (rust_func, ffi_func)
}

fn generate_functions_without_params(
    prefix: &str,
    const_name: &Ident,
    value_str: &str,
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    let fn_name = format!("{}_{}", prefix, const_name.to_string().to_lowercase());
    let fn_ident = Ident::new(&fn_name, const_name.span());

    // Generate Rust function
    let rust_func = quote! {
        #[doc = "`"]
        #[doc = #value_str]
        #[doc = "`"]
        #[inline(always)]
        pub fn #fn_ident() -> &'static str {
            #value_str
        }
    };

    // Generate FFI function
    let pascal_name = pascal_case!(const_name.to_string());
    let pascal_prefix = pascal_case!(prefix);
    let ffi_fn_name = format!("JV_Const_{}{}", pascal_prefix, pascal_name);
    let ffi_fn_ident = Ident::new(&ffi_fn_name, const_name.span());

    let ffi_func = quote! {
        #[unsafe(no_mangle)]
        #[allow(nonstandard_style)]
        pub extern "C" fn #ffi_fn_ident() -> *mut libc::c_char {
            let s = #value_str;

            match std::ffi::CString::new(s) {
                Ok(c) => c.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
    };

    (rust_func, ffi_func)
}
