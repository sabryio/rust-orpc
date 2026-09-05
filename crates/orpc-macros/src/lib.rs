//! Procedural macros for orpc — declarative router syntax.
//!
//! This crate provides the `r!` macro for defining routers with a clean,
//! TypeScript-inspired object literal syntax.

mod ast;
mod generate;
mod openapi_macro;
mod orpc_macro;
mod parse;
mod zod_ts;

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

/// TypeScript-like OpenAPI metadata macro.
///
/// Provides object literal syntax for building OpenAPI metadata. Transforms
/// TypeScript-style `{ method: "GET", path: "/planets" }` into builder calls.
///
/// # Supported Fields
///
/// - `method`: HTTP method as a string literal (`"GET"`, `"POST"`, `"PUT"`, `"PATCH"`, `"DELETE"`)
/// - `path`: Path template as a string (e.g., `"/planets/{id}"`)
/// - `prefix`: Path prefix as a string (e.g., `"/api/v2"`)
///
/// All fields are optional. Multiple `prefix` fields are not supported in a single
/// macro invocation (use multiple `.meta()` calls instead).
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust,ignore
/// use orpc_core::{os, openapi, HttpMethod};
///
/// let proc = os()
///     .context::<AppContext>()
///     .meta(openapi!{
///         method: "GET",
///         path: "/planets"
///     })
///     .output::<Vec<Planet>>()
///     .handler(|ctx, _: ()| async { Ok(ctx.db.list().await) });
/// ```
///
/// ## With Prefix
///
/// ```rust,ignore
/// use orpc_core::{os, openapi};
///
/// let proc = os()
///     .context::<AppContext>()
///     .meta(openapi!{ prefix: "/api/v2" })
///     .meta(openapi!{
///         method: "POST",
///         path: "/users"
///     })
///     .output::<User>()
///     .handler(|ctx, input| async { Ok(ctx.create_user(input).await) });
/// ```
///
/// ## Path Parameters
///
/// ```rust,ignore
/// use orpc_core::{os, openapi};
///
/// let proc = os()
///     .context::<AppContext>()
///     .meta(openapi!{
///         method: "GET",
///         path: "/planets/{id}"
///     })
///     .input::<FindInput>()
///     .output::<Planet>()
///     .handler(|ctx, input| async { ctx.find_planet(input.id).await });
/// ```
///
/// # Compilation Errors
///
/// The macro validates inputs at compile time:
///
/// ```rust,compile_fail
/// # use orpc_core::openapi;
/// // ERROR: Invalid HTTP method
/// let meta = openapi!{ method: "INVALID" };
/// ```
///
/// ```rust,compile_fail
/// # use orpc_core::openapi;
/// // ERROR: Unknown field
/// let meta = openapi!{ unknown_field: "value" };
/// ```
///
/// ```rust,compile_fail
/// # use orpc_core::openapi;
/// // ERROR: method must be a string literal
/// let method_var = "GET";
/// let meta = openapi!{ method: method_var };
/// ```
///
/// # Generated Code
///
/// The macro generates builder chain code:
///
/// ```rust,ignore
/// openapi!{ method: "GET", path: "/planets" }
///
/// // Expands to:
/// {
///     let mut builder = ::orpc_core::openapi_builder();
///     builder = builder.method(::orpc_core::HttpMethod::Get);
///     builder = builder.path("/planets");
///     builder.build()
/// }
/// ```
#[proc_macro]
pub fn openapi(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as openapi_macro::OpenApiMacroInput);
    match openapi_macro::generate(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Annotate a plain Axum handler to register its metadata with orpc.
///
/// This macro does **not** change the handler function — it remains a fully
/// valid Axum handler. It adds an `inventory::submit!` call that registers
/// the handler's route metadata globally, enabling `orpc::router()` and
/// `orpc::generate_contract()` to discover it automatically.
///
/// # Syntax
///
/// ```rust,ignore
/// #[orpc(method = "POST", path = "/planet/list")]
/// async fn list_planets(State(db): State<Db>) -> Json<Vec<Planet>> {
///     Json(db.list().await)
/// }
/// ```
///
/// # Arguments
///
/// - `method` — HTTP method in any case (`"GET"`, `"post"`, etc.), normalized to uppercase
/// - `path` — HTTP path string (e.g. `"/planet/list"`)
///
/// Both arguments are required.
#[proc_macro_attribute]
pub fn orpc(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as orpc_macro::OrpcArgs);
    let func = parse_macro_input!(item as syn::ItemFn);
    orpc_macro::expand_orpc(args, func).into()
}

/// Derive macro that generates a `fn zod_ts() -> String` method on structs and enums.
///
/// The generated method returns a complete TypeScript block with a Zod schema
/// and a `z.infer` type alias — ready to write to a `.ts` file.
///
/// # Example
///
/// ```rust,ignore
/// use orpc::ZodTs;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Serialize, Deserialize, ZodTs)]
/// pub struct Planet {
///     pub id: i32,
///     #[zod(min_length(1), max_length(100))]
///     pub name: String,
///     pub description: Option<String>,
/// }
///
/// // Generated TypeScript:
/// // import * as z from "zod";
/// //
/// // export const PlanetSchema = z.object({
/// //   id: z.number().int(),
/// //   name: z.string().min(1).max(100),
/// //   description: z.string().optional(),
/// // });
/// //
/// // export type Planet = z.infer<typeof PlanetSchema>;
/// println!("{}", Planet::zod_ts());
/// ```
///
/// ## Supported `#[zod(...)]` field attributes
///
/// **Strings:** `min_length(n)`, `max_length(n)`, `length(n)`, `email`, `url`,
/// `regex("pattern")`, `starts_with("s")`, `ends_with("s")`, `includes("s")`
///
/// **Numbers:** `min(n)`, `max(n)`, `int`, `positive`, `negative`,
/// `nonnegative`, `nonpositive`, `finite`
///
/// **Arrays (`Vec<T>`):** `min_length(n)`, `max_length(n)`, `length(n)`
#[proc_macro_derive(ZodTs, attributes(zod))]
pub fn derive_zod_ts(input: TokenStream) -> TokenStream {
    zod_ts::derive_zod_ts(input)
}
