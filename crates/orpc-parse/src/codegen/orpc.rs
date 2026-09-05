//! Code generation for the `#[orpc(method, path)]` attribute macro.
//!
//! Parses the attribute arguments, analyses the handler signature, and emits:
//! - The original function unchanged
//! - An `inventory::submit!` for `HandlerMetadata`
//! - An `inventory::submit!` for `HandlerRegistration` (Axum router factory)
//! - `inventory::submit!` blocks for `SchemaRegistration` fallback schemas

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Expr, ExprLit, ItemFn, Lit, MetaNameValue, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

use crate::{
    errors::{Error, Result, type_display},
    functions::extract_handler_signature,
    types::{JSON, RESULT, innermost_custom_type, is_primitive, try_extract_wrapper},
};

// ---------------------------------------------------------------------------
// OrpcArgs — parsed from #[orpc(method = "...", path = "...", stream_event = "...")]
// ---------------------------------------------------------------------------

/// Parsed arguments for the `#[orpc(...)]` attribute.
pub struct OrpcArgs {
    pub method: String,
    pub path: String,
    pub stream_event: Option<String>,
}

const VALID_KEYS: &[&str] = &["method", "path", "stream_event"];

impl Parse for OrpcArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let pairs = Punctuated::<MetaNameValue, Token![,]>::parse_terminated(input)?;

        let mut method = None;
        let mut path = None;
        let mut stream_event = None;

        for pair in &pairs {
            let key = pair
                .path
                .get_ident()
                .map(|i| i.to_string())
                .unwrap_or_default();

            let span = pair
                .path
                .get_ident()
                .map(|i| i.span())
                .unwrap_or_else(proc_macro2::Span::call_site);

            let value = match &pair.value {
                Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) => s.value(),
                _ => {
                    return Err(syn::Error::new(
                        span,
                        Error::invalid_attr_value(
                            span,
                            &key,
                            "a string literal",
                            "non-string expression",
                        )
                        .to_string(),
                    ));
                }
            };

            match key.as_str() {
                "method" => method = Some(value.to_uppercase()),
                "path" => path = Some(value),
                "stream_event" => stream_event = Some(value),
                _ => {
                    return Err(syn::Error::new(
                        span,
                        Error::unknown_key(span, &key, VALID_KEYS).to_string(),
                    ));
                }
            }
        }

        let method = method.ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                Error::missing_required_attr(
                    proc_macro2::Span::call_site(),
                    "method",
                    "add `method = \"GET\"` to #[orpc]",
                )
                .to_string(),
            )
        })?;

        let path = path.ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                Error::missing_required_attr(
                    proc_macro2::Span::call_site(),
                    "path",
                    "add `path = \"/your/route\"` to #[orpc]",
                )
                .to_string(),
            )
        })?;

        Ok(OrpcArgs {
            method,
            path,
            stream_event,
        })
    }
}

// ---------------------------------------------------------------------------
// expand_orpc
// ---------------------------------------------------------------------------

/// Generate the full expansion for `#[orpc(method, path)] async fn handler(...)`.
///
/// Returns the original function unchanged plus all inventory registrations.
pub fn expand_orpc(args: OrpcArgs, func: ItemFn) -> TokenStream {
    match try_expand_orpc(args, func) {
        Ok(ts) => ts,
        Err(e) => e.to_compile_error(),
    }
}

fn try_expand_orpc(args: OrpcArgs, func: ItemFn) -> Result<TokenStream> {
    let sig = extract_handler_signature(&func)?;

    let fn_name = &func.sig.ident;
    let fn_name_str = sig.fn_name.as_str();
    let method = &args.method;
    let path = &args.path;

    let output_type_str = type_display(&sig.output_type);

    let error_type_token = match &sig.error_type {
        Some(ty) => {
            let s = type_display(ty);
            quote! { Some(#s) }
        }
        None => quote! { None },
    };

    let stream_event_token = match &args.stream_event {
        Some(name) => quote! { Some(#name) },
        None => quote! { None },
    };

    let input_type_str = match &sig.input_type {
        Some(ty) => type_display(ty),
        None => "()".to_string(),
    };

    let registration = emit_handler_registration(fn_name, method, path, &sig.state_type);
    let schema_registrations = emit_schema_registrations(&func);

    Ok(quote! {
        #func

        ::orpc::inventory::submit! {
            ::orpc::HandlerMetadata {
                name: #fn_name_str,
                method: #method,
                path: #path,
                input_type_name: #input_type_str,
                output_type_name: #output_type_str,
                module_path: ::std::module_path!(),
                error_type_name: #error_type_token,
                stream_event_type_name: #stream_event_token,
            }
        }

        #registration
        #schema_registrations
    })
}

// ---------------------------------------------------------------------------
// Handler registration factory
// ---------------------------------------------------------------------------

fn emit_handler_registration(
    fn_name: &syn::Ident,
    method: &str,
    path: &str,
    state_type: &Option<syn::Type>,
) -> TokenStream {
    if let Some(state_ty) = state_type {
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
    }
}

// ---------------------------------------------------------------------------
// Schema registrations — z.unknown() fallback for types without #[derive(ZodTs)]
// ---------------------------------------------------------------------------

fn emit_schema_registrations(func: &ItemFn) -> TokenStream {
    let mut seen = std::collections::HashSet::new();
    let mut registrations = Vec::new();

    // Collect candidate types from Json<T> params and return type
    let mut candidates: Vec<&syn::Type> = Vec::new();

    for arg in &func.sig.inputs {
        if let syn::FnArg::Typed(pat_type) = arg
            && let Some(m) = try_extract_wrapper(&pat_type.ty, JSON)
            && let Some(inner) = m.first_type()
        {
            candidates.push(inner);
        }
    }

    if let syn::ReturnType::Type(_, ty) = &func.sig.output {
        // Handle both Json<T> and Result<Json<T>, E>
        if let Some(m) = try_extract_wrapper(ty, JSON) {
            if let Some(inner) = m.first_type() {
                candidates.push(inner);
            }
        } else if let Some(result_m) = try_extract_wrapper(ty, RESULT)
            && let Some(first) = result_m.first_type()
            && let Some(json_m) = try_extract_wrapper(first, JSON)
            && let Some(inner) = json_m.first_type()
        {
            candidates.push(inner);
        }
    }

    for ty in candidates {
        if let Some(custom_ty) = innermost_custom_type(ty) {
            if is_primitive(custom_ty) {
                continue;
            }
            let name = type_display(custom_ty);
            if !seen.insert(name.clone()) {
                continue;
            }
            let fallback = format!(
                "z.unknown() /* add #[derive(ZodTs)] to {} for a real schema */",
                name
            );
            registrations.push(quote! {
                ::orpc::inventory::submit! {
                    ::orpc::SchemaRegistration {
                        type_name: #name,
                        zod_ts: || #fallback.to_string(),
                        dependent_types: || vec![],
                    }
                }
            });
        }
    }

    quote! { #(#registrations)* }
}
