//! Parser adapters for the r! macro.
//!
//! Implements syn's Parse trait to convert token streams into our domain AST
//! (Clean Architecture: Ports layer — adapts external parsing to domain types).

use crate::ast::{RouterItem, RouterKey, RouterMacroInput};
use syn::{
    braced,
    parse::{Parse, ParseStream},
    token, Expr, Ident, LitStr, Result, Token,
};

impl Parse for RouterMacroInput {
    /// Parses the macro input: `key: value, key: { ... }, ...`
    ///
    /// Expects comma-separated items (no outer braces needed).
    /// Trailing commas are optional.
    fn parse(input: ParseStream) -> Result<Self> {
        let mut items = Vec::new();

        // Parse comma-separated items until we reach the end
        while !input.is_empty() {
            items.push(parse_item(input)?);

            // Optional trailing comma
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(RouterMacroInput::new(items))
    }
}

/// Parses a single router item: `key: value` or `key: { ... }`
///
/// Distinguishes between procedure expressions and nested router blocks.
fn parse_item(input: ParseStream) -> Result<RouterItem> {
    // Parse the key (identifier or string literal)
    let key = parse_key(input)?;

    // Expect colon
    input.parse::<Token![:]>()?;

    // Check if this is a nested router (brace group) or a procedure (expression)
    if input.peek(token::Brace) {
        // Nested router: key: { items... }
        let content;
        braced!(content in input);

        let mut items = Vec::new();

        while !content.is_empty() {
            items.push(parse_item(&content)?);

            // Optional trailing comma
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(RouterItem::Nested { key, items })
    } else {
        // Procedure: key: expr
        let expr = input.parse::<Expr>()?;
        Ok(RouterItem::Procedure { key, expr })
    }
}

/// Parses a router key — either an identifier or a string literal.
///
/// # Examples
///
/// - `ping` → `RouterKey::Ident`
/// - `"list-paginated"` → `RouterKey::Literal`
///
/// # Errors
///
/// Returns an error if neither an identifier nor a string literal is found.
fn parse_key(input: ParseStream) -> Result<RouterKey> {
    // Try parsing as identifier first
    if input.peek(Ident) {
        let ident = input.parse::<Ident>()?;
        return Ok(RouterKey::Ident(ident));
    }

    // Try parsing as string literal
    if input.peek(LitStr) {
        let lit = input.parse::<LitStr>()?;
        return Ok(RouterKey::Literal(lit));
    }

    // Neither identifier nor string literal found
    Err(input.error("expected identifier or string literal for router key"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn test_parse_empty_router() {
        let input = quote! {};
        let parsed: RouterMacroInput = syn::parse2(input).unwrap();
        assert!(parsed.is_empty());
    }

    #[test]
    fn test_parse_single_procedure() {
        let input = quote! {
            ping: os().output::<String>().handler(|_ctx, _: ()| async { Ok("pong".to_string()) })
        };
        let parsed: RouterMacroInput = syn::parse2(input).unwrap();
        assert_eq!(parsed.items.len(), 1);
        match &parsed.items[0] {
            RouterItem::Procedure { key, .. } => {
                assert_eq!(key.to_string(), "ping");
            }
            _ => panic!("Expected procedure"),
        }
    }

    #[test]
    fn test_parse_nested_router() {
        let input = quote! {
            planet: {
                list: os().output::<Vec<String>>().handler(|_ctx, _: ()| async { Ok(vec![]) })
            }
        };
        let parsed: RouterMacroInput = syn::parse2(input).unwrap();
        assert_eq!(parsed.items.len(), 1);
        match &parsed.items[0] {
            RouterItem::Nested { key, items } => {
                assert_eq!(key.to_string(), "planet");
                assert_eq!(items.len(), 1);
            }
            _ => panic!("Expected nested router"),
        }
    }

    #[test]
    fn test_parse_string_literal_key() {
        let input = quote! {
            "list-paginated": os().output::<Vec<String>>().handler(|_ctx, _: ()| async { Ok(vec![]) })
        };
        let parsed: RouterMacroInput = syn::parse2(input).unwrap();
        assert_eq!(parsed.items.len(), 1);
        match &parsed.items[0] {
            RouterItem::Procedure { key, .. } => {
                assert_eq!(key.to_string(), "list-paginated");
            }
            _ => panic!("Expected procedure"),
        }
    }

    #[test]
    fn test_parse_multiple_items() {
        let input = quote! {
            ping: os().output::<String>().handler(|_ctx, _: ()| async { Ok("pong".to_string()) }),
            pong: os().output::<String>().handler(|_ctx, _: ()| async { Ok("ping".to_string()) })
        };
        let parsed: RouterMacroInput = syn::parse2(input).unwrap();
        assert_eq!(parsed.items.len(), 2);
    }

    #[test]
    fn test_parse_optional_trailing_comma() {
        let input_with_comma = quote! {
            ping: os().output::<String>().handler(|_ctx, _: ()| async { Ok("pong".to_string()) }),
        };
        let input_without_comma = quote! {
            ping: os().output::<String>().handler(|_ctx, _: ()| async { Ok("pong".to_string()) })
        };

        let parsed_with: RouterMacroInput = syn::parse2(input_with_comma).unwrap();
        let parsed_without: RouterMacroInput = syn::parse2(input_without_comma).unwrap();

        assert_eq!(parsed_with.items.len(), 1);
        assert_eq!(parsed_without.items.len(), 1);
    }

    #[test]
    fn test_parse_deep_nesting() {
        let input = quote! {
            api: {
                v1: {
                    users: {
                        list: os().output::<Vec<String>>().handler(|_ctx, _: ()| async { Ok(vec![]) })
                    }
                }
            }
        };
        let parsed: RouterMacroInput = syn::parse2(input).unwrap();
        assert_eq!(parsed.items.len(), 1);

        // Verify deep nesting structure
        match &parsed.items[0] {
            RouterItem::Nested { key, items } => {
                assert_eq!(key.to_string(), "api");
                assert_eq!(items.len(), 1);

                match &items[0] {
                    RouterItem::Nested { key, items } => {
                        assert_eq!(key.to_string(), "v1");
                        assert_eq!(items.len(), 1);
                    }
                    _ => panic!("Expected nested v1"),
                }
            }
            _ => panic!("Expected nested api"),
        }
    }
}
