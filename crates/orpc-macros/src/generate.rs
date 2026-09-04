//! Code generation for the router! macro.
//!
//! Transforms domain AST types into TokenStreams representing RouterBuilder chains
//! (Clean Architecture: Adapters layer — adapts domain to external code generation).
//!
//! Keys in RouterItem::Procedure are no longer passed to .add() — the registry
//! key is derived automatically from the procedure's route path.
//! Keys in RouterItem::Nested are still used for .nest() grouping.

use crate::ast::{RouterItem, RouterKey, RouterMacroInput};
use proc_macro2::TokenStream;
use quote::quote;

/// Generates the complete router builder expression from the macro input.
///
/// Produces: `::orpc_core::r().add(expr).nest("key", r().add(expr)...)`
///
/// For empty routers, generates `::orpc_core::r::<()>()` so the type can be inferred.
///
/// # Example
///
/// Input AST:
/// ```ignore
/// RouterMacroInput {
///     items: [
///         Procedure { key: "ping", expr: os().route(GET, "/ping")... },
///         Nested { key: "planet", items: [...] }
///     ]
/// }
/// ```
///
/// Output:
/// ```ignore
/// ::orpc_core::r()
///     .add(os().route(GET, "/ping")...)
///     .nest("planet", ::orpc_core::r().add(...))
/// ```
pub fn generate(input: &RouterMacroInput) -> TokenStream {
    let items = generate_items(&input.items);

    if input.items.is_empty() {
        quote! { ::orpc_core::r::<()>() }
    } else {
        quote! {
            ::orpc_core::r()
                #items
        }
    }
}

/// Generates the chain of `.add()` and `.nest()` calls for a list of items.
fn generate_items(items: &[RouterItem]) -> TokenStream {
    let mut tokens = TokenStream::new();

    for item in items {
        let item_tokens = match item {
            RouterItem::Procedure { expr, .. } => generate_procedure(expr),
            RouterItem::Nested { key, items } => generate_nested(key, items),
        };

        tokens.extend(item_tokens);
    }

    tokens
}

/// Generates a single procedure call: `.add(expr)`
///
/// No key needed — the registry key is derived from the procedure's route path.
///
/// # Example
///
/// ```ignore
/// generate_procedure(os().route(GET, "/ping").output::<String>()...)
/// ```
///
/// Produces:
/// ```ignore
/// .add(os().route(GET, "/ping").output::<String>()...)
/// ```
fn generate_procedure(expr: &syn::Expr) -> TokenStream {
    quote! {
        .add(#expr)
    }
}

/// Generates a nested router call: `.nest("key", r().add(...).nest(...))`
///
/// The nest key is still needed for organizational grouping in the router hierarchy.
fn generate_nested(key: &RouterKey, items: &[RouterItem]) -> TokenStream {
    let key_str = key.to_string();
    let nested_items = generate_items(items);

    quote! {
        .nest(#key_str, ::orpc_core::r()
            #nested_items
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{RouterItem, RouterKey, RouterMacroInput};
    use quote::quote;
    use syn::parse_quote;

    #[test]
    fn test_generate_empty_router() {
        let input = RouterMacroInput::new(vec![]);
        let output = generate(&input);

        let expected = quote! { ::orpc_core::r::<()>() };
        assert_eq!(output.to_string(), expected.to_string());
    }

    #[test]
    fn test_generate_single_procedure() {
        let expr: syn::Expr = parse_quote! {
            os().route(HttpMethod::Get, "/ping").output::<String>().handler(|_ctx, _: ()| async { Ok("pong".to_string()) })
        };

        let input = RouterMacroInput::new(vec![RouterItem::Procedure {
            key: RouterKey::Ident(parse_quote!(ping)),
            expr,
        }]);

        let output = generate(&input);
        let output_str = output.to_string();

        // No key in .add() — just the expression
        assert!(output_str.contains(":: orpc_core :: r ()"));
        assert!(output_str.contains(". add ("));
        assert!(!output_str.contains(". add (\"ping\""));
    }

    #[test]
    fn test_generate_multiple_procedures() {
        let expr1: syn::Expr = parse_quote! {
            os().route(HttpMethod::Get, "/ping").output::<String>().handler(|_ctx, _: ()| async { Ok("pong".to_string()) })
        };
        let expr2: syn::Expr = parse_quote! {
            os().route(HttpMethod::Get, "/pong").output::<String>().handler(|_ctx, _: ()| async { Ok("ping".to_string()) })
        };

        let input = RouterMacroInput::new(vec![
            RouterItem::Procedure {
                key: RouterKey::Ident(parse_quote!(ping)),
                expr: expr1,
            },
            RouterItem::Procedure {
                key: RouterKey::Ident(parse_quote!(pong)),
                expr: expr2,
            },
        ]);

        let output = generate(&input);
        let output_str = output.to_string();

        // Two .add() calls, no keys
        assert_eq!(output_str.matches(". add (").count(), 2);
    }

    #[test]
    fn test_generate_nested_router_still_uses_key() {
        let expr: syn::Expr = parse_quote! {
            os().route(HttpMethod::Get, "/planet").output::<Vec<String>>().handler(|_ctx, _: ()| async { Ok(vec![]) })
        };

        let input = RouterMacroInput::new(vec![RouterItem::Nested {
            key: RouterKey::Ident(parse_quote!(planet)),
            items: vec![RouterItem::Procedure {
                key: RouterKey::Ident(parse_quote!(list)),
                expr,
            }],
        }]);

        let output = generate(&input);
        let output_str = output.to_string();

        // Nest key is still used for .nest()
        assert!(output_str.contains(". nest (\"planet\""));
        // But .add() has no key
        assert!(!output_str.contains(". add (\"list\""));
        assert!(output_str.contains(". add ("));
    }

    #[test]
    fn test_generate_deep_nesting() {
        let expr: syn::Expr = parse_quote! {
            os().route(HttpMethod::Get, "/api/v1/users").output::<Vec<String>>().handler(|_ctx, _: ()| async { Ok(vec![]) })
        };

        let input = RouterMacroInput::new(vec![RouterItem::Nested {
            key: RouterKey::Ident(parse_quote!(api)),
            items: vec![RouterItem::Nested {
                key: RouterKey::Ident(parse_quote!(v1)),
                items: vec![RouterItem::Procedure {
                    key: RouterKey::Ident(parse_quote!(users)),
                    expr,
                }],
            }],
        }]);

        let output = generate(&input);
        let output_str = output.to_string();

        assert!(output_str.contains(". nest (\"api\""));
        assert!(output_str.contains(". nest (\"v1\""));
        assert!(output_str.contains(". add ("));
    }
}
