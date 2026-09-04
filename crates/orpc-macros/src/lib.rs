//! Procedural macros for orpc — declarative router syntax.
//!
//! This crate provides the `r!` macro for defining routers with a clean,
//! TypeScript-inspired object literal syntax.

mod ast;
mod generate;
mod parse;

use proc_macro::TokenStream;
use syn::parse_macro_input;

/// Declarative router macro with nested object syntax.
///
/// Transforms TypeScript-like router definitions into `RouterBuilder` method chains.
/// This macro provides a clean, declarative syntax for defining RPC routers that
/// mirrors the structure of TypeScript oRPC's plain object pattern.
///
/// # Syntax
///
/// ```text
/// router! {
///     key: procedure_expression,
///     key: {
///         nested_key: procedure_expression,
///         ...
///     },
///     ...
/// }
/// ```
///
/// - **Keys** can be identifiers (`ping`) or string literals (`"list-paginated"`)
/// - **Values** are either procedure expressions or nested router blocks
/// - **Trailing commas** are optional
///
/// # Basic Example
///
/// ```rust,ignore
/// use orpc_core::{router, os};
///
/// #[derive(Clone)]
/// struct AppContext {
///     greeting: String,
/// }
///
/// let router = router! {
///     ping: os()
///         .context::<AppContext>()
///         .output::<String>()
///         .handler(|_ctx, _: ()| async { Ok("pong".to_string()) })
/// };
/// ```
///
/// # Nested Routers
///
/// ```rust,ignore
/// let router = router! {
///     ping: os()
///         .context::<AppContext>()
///         .output::<String>()
///         .handler(|_ctx, _: ()| async { Ok("pong".to_string()) }),
///     
///     planet: {
///         list: os()
///             .context::<AppContext>()
///             .output::<Vec<Planet>>()
///             .handler(|ctx, _: ()| async move { Ok(ctx.db.list().await) }),
///         
///         find: os()
///             .context::<AppContext>()
///             .input::<FindInput>()
///             .output::<Planet>()
///             .handler(|ctx, input| async move {
///                 ctx.db.find(input.id).await
///                     .ok_or_else(|| OrpcError::not_found("Not found"))
///             })
///     }
/// };
/// ```
///
/// The above expands to:
///
/// ```rust,ignore
/// let router = r()
///     .add("ping", os().context::<AppContext>()...)
///     .nest("planet", r()
///         .add("list", os().context::<AppContext>()...)
///         .add("find", os().context::<AppContext>()...));
/// ```
///
/// # String Literal Keys
///
/// Use string literals for keys with special characters:
///
/// ```rust,ignore
/// router! {
///     ping: os()...,
///     "list-paginated": os()...,  // kebab-case
///     "users:create": os()...,    // colons
/// }
/// ```
///
/// # Integration with Axum
///
/// ```rust,ignore
/// use orpc_axum::AxumRouter;
///
/// let router = router! {
///     ping: os()
///         .context::<AppContext>()
///         .output::<String>()
///         .handler(|_ctx, _: ()| async { Ok("pong".to_string()) })
/// };
///
/// let app = router.into_axum_router(AppContext { greeting: "Hello".to_string() });
/// axum::serve(listener, app).await.unwrap();
/// ```
///
/// # Comparison with Manual Builder
///
/// The macro is syntactic sugar over `RouterBuilder`. These are equivalent:
///
/// **Macro:**
/// ```rust,ignore
/// router! {
///     ping: os()...,
///     planet: { list: os()... }
/// }
/// ```
///
/// **Manual:**
/// ```rust,ignore
/// r()
///     .add("ping", os()...)
///     .nest("planet", r().add("list", os()...))
/// ```
///
/// Use the macro for cleaner syntax, or the manual builder when you need
/// programmatic router construction.
///
/// # Generated Code
///
/// The macro generates a `RouterBuilder` that implements the `Router` trait.
/// All procedures are automatically registered with their full paths during
/// router composition.
///
/// Nested routers create hierarchical paths:
/// - `ping` → registered as `"ping"`
/// - `planet: { list: ... }` → registered as `"planet/list"`
/// - `api: { v1: { users: ... } }` → registered as `"api/v1/users"`
#[proc_macro]
pub fn router(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ast::RouterMacroInput);
    generate::generate(&input).into()
}
