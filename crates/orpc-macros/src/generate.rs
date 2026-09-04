//! Code generation for the r! macro.
//!
//! Transforms domain AST types into TokenStreams representing RouterBuilder chains
//! (Clean Architecture: Adapters layer — adapts domain to external code generation).

use crate::ast::{RouterItem, RouterKey, RouterMacroInput};
use proc_macro2::TokenStream;
use quote::quote;

/// Generates the complete router builder expression from the macro input.
///
/// Produces: `::orpc_core::r().add(...).nest(...)`
///
/// For empty routers, generates `::orpc_core::r::<()>()` so the type can be inferred.
///
/// # Example
///
/// Input AST:
/// ```ignore
/// RouterMacroInput {
///     items: [
///         Procedure { key: "ping", expr: os()... },
///         Nested { key: "planet", items: [...] }
///     ]
/// }
/// ```
///
/// Output:
/// ```ignore
/// ::orpc_core::r()
///     .add("ping", os()...)
///     .nest("planet", ::orpc_core::r().add(...))
/// ```
pub fn generate(input: &RouterMacroInput) -> TokenStream {
    let items = generate_items(&input.items);

    // For empty routers, explicitly specify () as context type for type inference
    if input.items.is_empty() {
        quote! {
            ::orpc_core::r::<()>()
        }
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
            RouterItem::Procedure { key, expr } => generate_procedure(key, expr),
            RouterItem::Nested { key, items } => generate_nested(key, items),
        };

        tokens.extend(item_tokens);
    }

    tokens
}

/// Generates a single procedure call: `.add("key", expr)`
///
/// # Example
///
/// ```ignore
/// generate_procedure("ping", os().output::<String>()...)
/// ```
///
/// Produces:
/// ```ignore
/// .add("ping", os().output::<String>()...)
/// ```
fn generate_procedure(key: &RouterKey, expr: &syn::Expr) -> TokenStream {
    let key_str = key.to_string();

    quote! {
        .add(#key_str, #expr)
    }
}

/// Generates a nested router call: `.nest("key", r().add(...).nest(...))`
///
/// Recursively generates the nested router's items.
///
/// # Example
///
/// ```ignore
/// generate_nested("planet", [
///     Procedure { key: "list", expr: os()... },
///     Procedure { key: "find", expr: os()... },
/// ])
/// ```
///
/// Produces:
/// ```ignore
/// .nest("planet", ::orpc_core::r()
///     .add("list", os()...)
///     .add("find", os()...))
/// ```
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

        // Should generate r::<()>() for empty routers to enable type inference
        let expected = quote! {
            ::orpc_core::r::<()>()
        };

        assert_eq!(output.to_string(), expected.to_string());
    }

    #[test]
    fn test_generate_single_procedure() {
        let expr: syn::Expr = parse_quote! {
            os().output::<String>().handler(|_ctx, _: ()| async { Ok("pong".to_string()) })
        };

        let input = RouterMacroInput::new(vec![RouterItem::Procedure {
            key: RouterKey::Ident(parse_quote!(ping)),
            expr,
        }]);

        let output = generate(&input);

        // Should generate r().add("ping", expr)
        let output_str = output.to_string();
        assert!(output_str.contains(":: orpc_core :: r ()"));
        assert!(output_str.contains(". add (\"ping\""));
    }

    #[test]
    fn test_generate_multiple_procedures() {
        let expr1: syn::Expr = parse_quote! {
            os().output::<String>().handler(|_ctx, _: ()| async { Ok("pong".to_string()) })
        };
        let expr2: syn::Expr = parse_quote! {
            os().output::<String>().handler(|_ctx, _: ()| async { Ok("ping".to_string()) })
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

        assert!(output_str.contains(". add (\"ping\""));
        assert!(output_str.contains(". add (\"pong\""));
    }

    #[test]
    fn test_generate_nested_router() {
        let expr: syn::Expr = parse_quote! {
            os().output::<Vec<String>>().handler(|_ctx, _: ()| async { Ok(vec![]) })
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

        assert!(output_str.contains(". nest (\"planet\""));
        assert!(output_str.contains(". add (\"list\""));
    }

    #[test]
    fn test_generate_string_literal_key() {
        let expr: syn::Expr = parse_quote! {
            os().output::<Vec<String>>().handler(|_ctx, _: ()| async { Ok(vec![]) })
        };

        let input = RouterMacroInput::new(vec![RouterItem::Procedure {
            key: RouterKey::Literal(parse_quote!("list-paginated")),
            expr,
        }]);

        let output = generate(&input);
        let output_str = output.to_string();

        assert!(output_str.contains(". add (\"list-paginated\""));
    }

    #[test]
    fn test_generate_deep_nesting() {
        let expr: syn::Expr = parse_quote! {
            os().output::<Vec<String>>().handler(|_ctx, _: ()| async { Ok(vec![]) })
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
        assert!(output_str.contains(". add (\"users\""));
    }

    #[test]
    fn test_generate_mixed_items() {
        let expr1: syn::Expr = parse_quote! {
            os().output::<String>().handler(|_ctx, _: ()| async { Ok("pong".to_string()) })
        };
        let expr2: syn::Expr = parse_quote! {
            os().output::<Vec<String>>().handler(|_ctx, _: ()| async { Ok(vec![]) })
        };

        let input = RouterMacroInput::new(vec![
            RouterItem::Procedure {
                key: RouterKey::Ident(parse_quote!(ping)),
                expr: expr1,
            },
            RouterItem::Nested {
                key: RouterKey::Ident(parse_quote!(planet)),
                items: vec![RouterItem::Procedure {
                    key: RouterKey::Ident(parse_quote!(list)),
                    expr: expr2,
                }],
            },
        ]);

        let output = generate(&input);
        let output_str = output.to_string();

        assert!(output_str.contains(". add (\"ping\""));
        assert!(output_str.contains(". nest (\"planet\""));
        assert!(output_str.contains(". add (\"list\""));
    }
}
