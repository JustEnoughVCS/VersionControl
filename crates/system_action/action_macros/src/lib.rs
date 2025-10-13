use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

/// A procedural macro for generating structs that implement the Action trait
///
/// Usage:
/// #[action_gen] or #[action_gen(local)]
/// pub fn my_action(ctx: ActionContext, arg: MyArg) -> Result<MyReturn, TcpTargetError> {
///     todo!()
/// }
#[proc_macro_attribute]
pub fn action_gen(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let is_local = if attr.is_empty() {
        false
    } else {
        let attr_str = attr.to_string();
        attr_str == "local" || attr_str.contains("local")
    };

    generate_action_struct(input_fn, is_local).into()
}

fn generate_action_struct(input_fn: ItemFn, _is_local: bool) -> proc_macro2::TokenStream {
    let fn_vis = &input_fn.vis;
    let fn_sig = &input_fn.sig;
    let fn_name = &fn_sig.ident;
    let fn_block = &input_fn.block;

    validate_function_signature(fn_sig);

    let (context_param_name, arg_param_name, arg_type, return_type) =
        extract_parameters_and_types(fn_sig);

    let struct_name = quote::format_ident!("{}", convert_to_pascal_case(&fn_name.to_string()));

    let action_name_ident = &fn_name;

    let register_this_action = quote::format_ident!("register_{}", action_name_ident);
    let proc_this_action = quote::format_ident!("proc_{}", action_name_ident);

    quote! {
        #[derive(Debug, Clone, Default)]
        #fn_vis struct #struct_name;

        impl action_system::action::Action<#arg_type, #return_type> for #struct_name {
            fn action_name() -> &'static str {
                Box::leak(string_proc::snake_case!(stringify!(#action_name_ident)).into_boxed_str())
            }

            fn is_remote_action() -> bool {
                !#_is_local
            }

            async fn process(#context_param_name: action_system::action::ActionContext, #arg_param_name: #arg_type) -> Result<#return_type, tcp_connection::error::TcpTargetError> {
                #fn_block
            }
        }

        #fn_vis fn #register_this_action(pool: &mut action_system::action_pool::ActionPool) {
            pool.register::<#struct_name, #arg_type, #return_type>();
        }

        #fn_vis async fn #proc_this_action(
            pool: &action_system::action_pool::ActionPool,
            ctx: action_system::action::ActionContext,
            #arg_param_name: #arg_type
        ) -> Result<#return_type, tcp_connection::error::TcpTargetError> {
            let args_json = serde_json::to_string(&#arg_param_name)
                .map_err(|e| {
                    tcp_connection::error::TcpTargetError::Serialization(e.to_string())
                })?;
            let result_json = pool.process_json(
                Box::leak(string_proc::snake_case!(stringify!(#action_name_ident)).into_boxed_str()),
                ctx,
                args_json,
            ).await?;
            serde_json::from_str(&result_json)
                .map_err(|e| {
                    tcp_connection::error::TcpTargetError::Serialization(e.to_string())
                })
        }

        #[allow(dead_code)]
        #[deprecated = "This function is used by #[action_gen] as a template."]
        #[doc = "Template function for #[[action_gen]] - do not call directly."]
        #[doc = "Use the generated struct instead."]
        #[doc = ""]
        #[doc = "Register the action to the pool."]
        #[doc = "```ignore"]
        #[doc = "register_your_func(&mut pool);"]
        #[doc = "```"]
        #[doc = ""]
        #[doc = "Process the action at the pool."]
        #[doc = "```ignore"]
        #[doc = "let result = proc_your_func(&pool, ctx, arg).await?;"]
        #[doc = "```"]
        #fn_vis #fn_sig #fn_block
    }
}

fn validate_function_signature(fn_sig: &syn::Signature) {
    if fn_sig.asyncness.is_none() {
        panic!("Expected async function for Action, but found synchronous function");
    }

    if fn_sig.inputs.len() != 2 {
        panic!(
            "Expected exactly 2 arguments for Action function: ctx: ActionContext and arg: T, but found {} arguments",
            fn_sig.inputs.len()
        );
    }

    let return_type = match &fn_sig.output {
        syn::ReturnType::Type(_, ty) => ty,
        _ => panic!(
            "Expected Action function to return Result<T, TcpTargetError>, but found no return type"
        ),
    };

    if let syn::Type::Path(type_path) = return_type.as_ref() {
        if let Some(segment) = type_path.path.segments.last()
            && segment.ident != "Result" {
                panic!(
                    "Expected Action function to return Result<T, TcpTargetError>, but found different return type"
                );
            }
    } else {
        panic!(
            "Expected Action function to return Result<T, TcpTargetError>, but found no return type"
        );
    }
}

fn convert_to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

fn extract_parameters_and_types(
    fn_sig: &syn::Signature,
) -> (
    proc_macro2::TokenStream,
    proc_macro2::TokenStream,
    proc_macro2::TokenStream,
    proc_macro2::TokenStream,
) {
    let mut inputs = fn_sig.inputs.iter();

    let context_param = match inputs.next() {
        Some(syn::FnArg::Typed(pat_type)) => {
            let pat = &pat_type.pat;
            quote::quote!(#pat)
        }
        _ => {
            panic!("Expected the first argument to be a typed parameter, but found something else")
        }
    };

    let arg_param = match inputs.next() {
        Some(syn::FnArg::Typed(pat_type)) => {
            let pat = &pat_type.pat;
            let ty = &pat_type.ty;
            (quote::quote!(#pat), quote::quote!(#ty))
        }
        _ => {
            panic!("Expected the second argument to be a typed parameter, but found something else")
        }
    };

    let (arg_param_name, arg_type) = arg_param;

    let return_type = match &fn_sig.output {
        syn::ReturnType::Type(_, ty) => {
            if let syn::Type::Path(type_path) = ty.as_ref() {
                if let syn::PathArguments::AngleBracketed(args) =
                    &type_path.path.segments.last().unwrap().arguments
                {
                    if let Some(syn::GenericArgument::Type(ty)) = args.args.first() {
                        quote::quote!(#ty)
                    } else {
                        panic!("Expected to extract the success type of Result, but failed");
                    }
                } else {
                    panic!("Expected Result type to have generic parameters, but found none");
                }
            } else {
                panic!("Expected return type to be Result, but found different type");
            }
        }
        _ => panic!("Expected function to have return type, but found none"),
    };

    (context_param, arg_param_name, arg_type, return_type)
}
