//! Procedural macro for OpenAPI metadata with TypeScript-like object literal syntax.
//!
//! Provides the `openapi!` macro that transforms object-like syntax into builder calls.
//!
//! # IDE Support
//!
//! The macro uses `quote_spanned!` to preserve span information, enabling:
//! - Hover documentation on field names (shows builder method docs)
//! - Jump-to-definition navigation
//! - Accurate error locations
//!
//! When you hover over `method`, `path`, or `prefix` in the macro invocation,
//! your IDE will show the documentation from the corresponding builder method.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Expr, Ident, Result, Token,
};

/// Parsed field in the openapi! macro: `field_name: value`
struct OpenApiField {
    name: Ident,
    value: Expr,
}

impl Parse for OpenApiField {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let value: Expr = input.parse()?;
        Ok(OpenApiField { name, value })
    }
}

/// Parsed input for openapi! macro: `{ field1: value1, field2: value2, ... }`
pub struct OpenApiMacroInput {
    fields: Punctuated<OpenApiField, Token![,]>,
}

impl Parse for OpenApiMacroInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let fields = Punctuated::parse_terminated(input)?;
        Ok(OpenApiMacroInput { fields })
    }
}

/// Converts a string literal HTTP method to HttpMethod enum variant.
///
/// Returns compile error for invalid methods.
fn parse_http_method(method_str: &str) -> Result<TokenStream> {
    let method = match method_str {
        "GET" => quote! { ::orpc_core::HttpMethod::Get },
        "POST" => quote! { ::orpc_core::HttpMethod::Post },
        "PUT" => quote! { ::orpc_core::HttpMethod::Put },
        "PATCH" => quote! { ::orpc_core::HttpMethod::Patch },
        "DELETE" => quote! { ::orpc_core::HttpMethod::Delete },
        _ => {
            return Err(syn::Error::new_spanned(
                method_str,
                format!(
                    "Invalid HTTP method '{}'. Expected one of: GET, POST, PUT, PATCH, DELETE",
                    method_str
                ),
            ))
        }
    };
    Ok(method)
}

/// Generates code for the openapi! macro.
///
/// Transforms:
/// ```ignore
/// openapi!{ method: "GET", path: "/planets", prefix: "/api" }
/// ```
///
/// Into:
/// ```ignore
/// {
///     let mut builder = ::orpc_core::openapi_builder();
///     builder = builder.method(::orpc_core::HttpMethod::Get);
///     builder = builder.path("/planets");
///     builder = builder.prefix("/api");
///     builder.build()
/// }
/// ```
///
/// The macro preserves span information for better IDE support and error messages.
pub fn generate(input: OpenApiMacroInput) -> Result<TokenStream> {
    let mut method_calls = Vec::new();

    for field in input.fields.iter() {
        let field_name = field.name.to_string();
        let field_span = field.name.span();

        match field_name.as_str() {
            "method" => {
                // Extract string literal from the expression
                if let Expr::Lit(expr_lit) = &field.value {
                    if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                        let method_str = lit_str.value();
                        let method_enum = parse_http_method(&method_str)?;

                        // Preserve span for IDE hover support
                        method_calls.push(quote::quote_spanned! {field_span=>
                            builder = builder.method(#method_enum);
                        });
                    } else {
                        return Err(syn::Error::new_spanned(
                            &field.value,
                            "method value must be a string literal (e.g., \"GET\", \"POST\")",
                        ));
                    }
                } else {
                    return Err(syn::Error::new_spanned(
                        &field.value,
                        "method value must be a string literal (e.g., \"GET\", \"POST\")",
                    ));
                }
            }

            "path" => {
                let value = &field.value;
                // Preserve span for IDE hover support
                method_calls.push(quote::quote_spanned! {field_span=>
                    builder = builder.path(#value);
                });
            }

            "prefix" => {
                let value = &field.value;
                // Preserve span for IDE hover support
                method_calls.push(quote::quote_spanned! {field_span=>
                    builder = builder.prefix(#value);
                });
            }

            _ => {
                return Err(syn::Error::new_spanned(
                    &field.name,
                    format!(
                        "Unknown field '{}'. Valid fields are: method, path, prefix",
                        field_name
                    ),
                ));
            }
        }
    }

    Ok(quote! {
        {
            let mut builder = ::orpc_core::openapi_builder();
            #(#method_calls)*
            builder.build()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_parse_http_method_get() {
        let result = parse_http_method("GET");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_http_method_post() {
        let result = parse_http_method("POST");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_http_method_invalid() {
        let result = parse_http_method("INVALID");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_input() {
        let input: OpenApiMacroInput = parse_quote! {};
        assert_eq!(input.fields.len(), 0);
    }

    #[test]
    fn test_parse_single_field() {
        let input: OpenApiMacroInput = parse_quote! {
            method: "GET"
        };
        assert_eq!(input.fields.len(), 1);
    }

    #[test]
    fn test_parse_multiple_fields() {
        let input: OpenApiMacroInput = parse_quote! {
            method: "POST",
            path: "/users",
            prefix: "/api"
        };
        assert_eq!(input.fields.len(), 3);
    }

    #[test]
    fn test_generate_method_only() {
        let input: OpenApiMacroInput = parse_quote! {
            method: "GET"
        };
        let result = generate(input);
        assert!(result.is_ok());
        let tokens = result.unwrap();
        let code = tokens.to_string();
        assert!(code.contains("HttpMethod::Get"));
    }

    #[test]
    fn test_generate_path_only() {
        let input: OpenApiMacroInput = parse_quote! {
            path: "/planets"
        };
        let result = generate(input);
        assert!(result.is_ok());
        let tokens = result.unwrap();
        let code = tokens.to_string();
        assert!(code.contains("path"));
        assert!(code.contains("/planets"));
    }

    #[test]
    fn test_generate_all_fields() {
        let input: OpenApiMacroInput = parse_quote! {
            method: "POST",
            path: "/users",
            prefix: "/api/v2"
        };
        let result = generate(input);
        assert!(result.is_ok());
        let tokens = result.unwrap();
        let code = tokens.to_string();
        assert!(code.contains("HttpMethod::Post"));
        assert!(code.contains("/users"));
        assert!(code.contains("/api/v2"));
    }

    #[test]
    fn test_generate_invalid_field_name() {
        let input: OpenApiMacroInput = parse_quote! {
            invalid_field: "value"
        };
        let result = generate(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_invalid_method_value() {
        let input: OpenApiMacroInput = parse_quote! {
            method: "INVALID"
        };
        let result = generate(input);
        assert!(result.is_err());
    }
}
