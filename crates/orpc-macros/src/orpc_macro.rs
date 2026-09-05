//! Implementation of the `#[orpc(method, path)]` attribute macro.
//!
//! Annotates a plain Axum handler function and registers a `HandlerMetadata`
//! entry via `inventory::submit!` at link time.
//!
//! # Example expansion
//!
//! ```rust,ignore
//! #[orpc(method = "POST", path = "/planet/list")]
//! async fn list_planets(State(db): State<Db>) -> Json<Vec<Planet>> {
//!     Json(db.list().await)
//! }
//! ```
//!
//! Expands to:
//!
//! ```rust,ignore
//! async fn list_planets(State(db): State<Db>) -> Json<Vec<Planet>> {
//!     Json(db.list().await)
//! }
//!
//! inventory::submit! {
//!     ::orpc::HandlerMetadata {
//!         name: "list_planets",
//!         method: "POST",
//!         path: "/planet/list",
//!         input_type_name: "Db",          // extracted from State<T>
//!         output_type_name: "Vec<Planet>",// extracted from Json<T>
//!         module_path: std::module_path!(),
//!     }
//! }
//! ```

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Expr, ExprLit, ItemFn, Lit, MetaNameValue, Result, ReturnType, Token, Type,
};

/// Parsed arguments from `#[orpc(method = "...", path = "...")]`
pub struct OrpcArgs {
    pub method: String,
    pub path: String,
}

impl Parse for OrpcArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let pairs = Punctuated::<MetaNameValue, Token![,]>::parse_terminated(input)?;

        let mut method = None;
        let mut path = None;

        for pair in pairs {
            let key = pair
                .path
                .get_ident()
                .map(|i| i.to_string())
                .unwrap_or_default();
            let value = match &pair.value {
                Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) => s.value(),
                _ => {
                    return Err(syn::Error::new_spanned(
                        &pair.value,
                        "#[orpc] attribute values must be string literals",
                    ))
                }
            };

            match key.as_str() {
                "method" => method = Some(value.to_uppercase()),
                "path" => path = Some(value),
                other => {
                    return Err(syn::Error::new_spanned(
                        &pair.path,
                        format!("unknown #[orpc] key `{other}` — expected `method` or `path`"),
                    ))
                }
            }
        }

        Ok(OrpcArgs {
            method: method.ok_or_else(|| {
                syn::Error::new(proc_macro2::Span::call_site(), "#[orpc] requires `method`")
            })?,
            path: path.ok_or_else(|| {
                syn::Error::new(proc_macro2::Span::call_site(), "#[orpc] requires `path`")
            })?,
        })
    }
}

/// Generate expanded tokens for `#[orpc(method, path)]`.
pub fn expand_orpc(args: OrpcArgs, func: ItemFn) -> TokenStream {
    let fn_name = &func.sig.ident;
    let fn_name_str = fn_name.to_string();
    let method = &args.method;
    let path = &args.path;

    // Extract output type name from return type
    let output_type_name = extract_output_type_name(&func.sig.output);

    // Extract error type name from Result<_, E> return type
    let error_type_name = extract_error_type_name(&func.sig.output);
    let error_type_name_token = match error_type_name {
        Some(ref name) => quote! { Some(#name) },
        None => quote! { None },
    };

    // Extract input type name from first Json<T> parameter
    let input_type_name = extract_input_type_name(&func.sig.inputs);

    // Extract state type from State<T> parameter — used in factory downcast
    let state_type = extract_state_type(&func.sig.inputs);

    // Collect non-primitive input/output types for schema registration
    let schema_registrations = generate_schema_registrations(&func.sig.inputs, &func.sig.output);

    let registration = if let Some(state_ty) = state_type {
        // Handler extracts State<S> — downcast to S before building router
        quote! {
            ::orpc::inventory::submit! {
                ::orpc::HandlerRegistration {
                    path: #path,
                    method: #method,
                    factory: |state: ::std::sync::Arc<dyn ::std::any::Any + Send + Sync>| {
                        use ::axum::routing::{delete, get, patch, post, put};
                        let method_router = match #method {
                            "GET"    => get(#fn_name),
                            "POST"   => post(#fn_name),
                            "PUT"    => put(#fn_name),
                            "PATCH"  => patch(#fn_name),
                            "DELETE" => delete(#fn_name),
                            _        => post(#fn_name),
                        };
                        if let Some(typed_state) = state.downcast_ref::<#state_ty>() {
                            ::axum::Router::new()
                                .route(#path, method_router)
                                .with_state(typed_state.clone())
                        } else {
                            ::axum::Router::new()
                        }
                    },
                }
            }
        }
    } else {
        // Stateless handler — no State<S> parameter
        quote! {
            ::orpc::inventory::submit! {
                ::orpc::HandlerRegistration {
                    path: #path,
                    method: #method,
                    factory: |_state: ::std::sync::Arc<dyn ::std::any::Any + Send + Sync>| {
                        use ::axum::routing::{delete, get, patch, post, put};
                        let method_router = match #method {
                            "GET"    => get(#fn_name),
                            "POST"   => post(#fn_name),
                            "PUT"    => put(#fn_name),
                            "PATCH"  => patch(#fn_name),
                            "DELETE" => delete(#fn_name),
                            _        => post(#fn_name),
                        };
                        ::axum::Router::new().route(#path, method_router)
                    },
                }
            }
        }
    };

    quote! {
        // Original function — completely unchanged
        #func

        // Register metadata for TypeScript contract generation
        ::orpc::inventory::submit! {
            ::orpc::HandlerMetadata {
                name: #fn_name_str,
                method: #method,
                path: #path,
                input_type_name: #input_type_name,
                output_type_name: #output_type_name,
                module_path: ::std::module_path!(),
                error_type_name: #error_type_name_token,
            }
        }

        // Register handler factory for auto-router construction
        #registration

        // Register Zod schemas for TypeScript contract generation.
        // Types must implement ZodTs (from zod-rs-ts crate).
        // Add #[derive(ZodTs)] to your input/output types.
        #schema_registrations
    }
}

/// Extract the state type `S` from a `State<S>` parameter in the function signature.
///
/// Returns `Some(Type)` if found, `None` if the handler doesn't extract state.
fn extract_state_type(inputs: &syn::punctuated::Punctuated<syn::FnArg, Token![,]>) -> Option<Type> {
    for input in inputs {
        if let syn::FnArg::Typed(pat_type) = input {
            if let Type::Path(type_path) = &*pat_type.ty {
                let last = type_path.path.segments.last()?;
                if last.ident == "State" {
                    if let syn::PathArguments::AngleBracketed(args) = &last.arguments {
                        if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                            return Some(inner.clone());
                        }
                    }
                }
            }
        }
    }
    None
}
/// Extract the error type name from `-> Result<T, E>`.
///
/// Returns `Some("E")` if the return type is `Result<_, E>`, `None` otherwise.
fn extract_error_type_name(return_type: &ReturnType) -> Option<String> {
    if let ReturnType::Type(_, ty) = return_type {
        if let Type::Path(type_path) = &**ty {
            let last = type_path.path.segments.last()?;
            if last.ident == "Result" {
                if let syn::PathArguments::AngleBracketed(args) = &last.arguments {
                    // Result<T, E> — get the second type argument (E)
                    if let Some(syn::GenericArgument::Type(error_ty)) = args.args.iter().nth(1) {
                        return Some(type_to_string(error_ty));
                    }
                }
            }
        }
    }
    None
}

/// Extract the output type name from `-> Json<T>` or `-> Result<Json<T>, E>`.
///
/// Returns a string representation of `T`.
fn extract_output_type_name(return_type: &ReturnType) -> String {
    match return_type {
        ReturnType::Default => "()".to_string(),
        ReturnType::Type(_, ty) => type_to_string(ty),
    }
}

/// Extract the JSON body input type from function parameters.
///
/// Looks for a parameter of type `Json<T>` or `axum::Json<T>` and returns
/// the string name of `T`. Returns `"()"` if not found.
fn extract_input_type_name(inputs: &syn::punctuated::Punctuated<syn::FnArg, Token![,]>) -> String {
    for input in inputs {
        if let syn::FnArg::Typed(pat_type) = input {
            let type_str = type_to_string(&pat_type.ty);
            // Match Json<T> patterns
            if let Some(inner) = extract_json_inner(&type_str) {
                return inner;
            }
        }
    }
    "()".to_string()
}

/// Extracts `T` from `Json<T>` or `axum::extract::Json<T>` type string.
fn extract_json_inner(type_str: &str) -> Option<String> {
    // Handles: Json<T>, axum::Json<T>, axum::extract::Json<T>
    let suffix = type_str
        .trim_start_matches("axum::extract::")
        .trim_start_matches("axum::");

    if let Some(inner) = suffix.strip_prefix("Json<") {
        let inner = inner.trim_end_matches('>');
        return Some(inner.to_string());
    }
    None
}

/// Render a `syn::Type` as a simplified string for metadata storage.
fn type_to_string(ty: &Type) -> String {
    quote!(#ty).to_string().replace(' ', "")
}

/// Generate `inventory::submit! { SchemaRegistration { ... } }` blocks
/// for non-primitive input and output types.
///
/// Each unique non-primitive type gets one registration that calls
/// `<T as ::zod_rs_ts::ZodTs>::zod_ts()` at runtime.
fn generate_schema_registrations(
    inputs: &syn::punctuated::Punctuated<syn::FnArg, Token![,]>,
    output: &ReturnType,
) -> TokenStream {
    let mut registrations = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // Collect candidate types: Json<T> from input, Json<T> from output
    let mut candidate_types: Vec<Type> = Vec::new();

    // From input parameters: Json<T>
    for input in inputs {
        if let syn::FnArg::Typed(pat_type) = input {
            if let Some(inner) = extract_inner_type(&pat_type.ty, "Json") {
                candidate_types.push(inner);
            }
        }
    }

    // From output: Json<T> or Result<Json<T>, _>
    if let ReturnType::Type(_, ty) = output {
        if let Some(inner) = extract_json_from_return(ty) {
            candidate_types.push(inner);
        }
    }

    for ty in candidate_types {
        let type_str = type_to_string(&ty);
        // Skip primitives and unit type
        if is_primitive_str(&type_str) {
            continue;
        }
        // Skip Vec<T> wrapper — register the inner T
        let (reg_type, reg_name) = if type_str.starts_with("Vec<") {
            let inner_str = type_str.trim_start_matches("Vec<").trim_end_matches('>');
            // Parse the inner type string back to a syn::Type
            if let Ok(inner_ty) = syn::parse_str::<Type>(inner_str) {
                let name = inner_str.to_string();
                (inner_ty, name)
            } else {
                continue;
            }
        } else {
            let name = type_str.clone();
            (ty.clone(), name)
        };

        if !seen.insert(reg_name.clone()) {
            continue; // deduplicate
        }

        let type_name_str = reg_name.clone();
        registrations.push(quote! {
            ::orpc::inventory::submit! {
                ::orpc::SchemaRegistration {
                    type_name: #type_name_str,
                    zod_ts: <#reg_type>::zod_ts,
                    dependent_types: <#reg_type>::dependent_types,
                }
            }
        });
    }

    quote! { #(#registrations)* }
}

/// Extract `T` from a `Wrapper<T>` type by wrapper name.
fn extract_inner_type(ty: &Type, wrapper: &str) -> Option<Type> {
    if let Type::Path(type_path) = ty {
        let last = type_path.path.segments.last()?;
        if last.ident == wrapper {
            if let syn::PathArguments::AngleBracketed(args) = &last.arguments {
                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                    return Some(inner.clone());
                }
            }
        }
    }
    None
}

/// Extract the output type from `-> Json<T>` or `-> Result<Json<T>, E>`.
fn extract_json_from_return(ty: &Type) -> Option<Type> {
    // Direct: Json<T>
    if let Some(inner) = extract_inner_type(ty, "Json") {
        return Some(inner);
    }
    // Result<Json<T>, E>
    if let Type::Path(type_path) = ty {
        let last = type_path.path.segments.last()?;
        if last.ident == "Result" {
            if let syn::PathArguments::AngleBracketed(args) = &last.arguments {
                if let Some(syn::GenericArgument::Type(first_arg)) = args.args.first() {
                    return extract_inner_type(first_arg, "Json");
                }
            }
        }
    }
    None
}

/// Returns true for Rust primitive type name strings.
fn is_primitive_str(type_name: &str) -> bool {
    matches!(
        type_name,
        "()" | "String"
            | "str"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "f32"
            | "f64"
            | "bool"
            | "usize"
            | "isize"
    )
}
