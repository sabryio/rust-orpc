//! Procedural macros for orpc — handler registration and code generation.
//!
//! This crate provides macros for annotating handlers, deriving schemas,
//! and building routers with auto-discovery.

mod ast;
mod error_derive;
mod generate;
mod openapi_macro;
mod orpc_macro;
mod parse;
mod router_macro;
mod zod_ts;

use proc_macro::TokenStream;
use syn::parse_macro_input;

/// Auto-discovery router macro with optional module filtering.
///
/// Discovers all `#[orpc]`-annotated handlers via the `inventory` crate and
/// builds an Axum `Router`. Optionally filters handlers by module path pattern.
///
/// # Syntax
///
/// ```text
/// router!()                              // No state, all handlers
/// router!(state)                         // With state, all handlers
/// router!("pattern")                     // No state, filtered
/// router!("pattern", state)              // Filtered + state (any order)
/// router!(state, "pattern")              // Filtered + state (any order)
/// router!(["pat1", "pat2"])              // Array of patterns
/// router!("prefix::{a,b}")               // Brace expansion
/// router!("prefix::*")                   // Wildcard
/// ```
///
/// # Pattern Matching
///
/// Patterns match module paths using prefix matching:
/// - `"handlers::planet"` matches `handlers::planet` and `handlers::planet::*`
/// - `"handlers::*"` wildcard matches all under `handlers::`
/// - `"handlers::{planet,user}"` brace expansion to multiple patterns
/// - `["handlers::planet", "api::v1"]` explicit array of patterns
///
/// # Examples
///
/// ```rust,ignore
/// // All handlers with shared state
/// let app = router!(db);
///
/// // Only planet handlers
/// let app = router!("handlers::planet", db);
///
/// // Multiple modules with brace expansion
/// let app = router!("handlers::{planet,user}", db);
///
/// // Compose with Axum nesting
/// let app = Router::new()
///     .nest("/planet", router!("handlers::planet", db.clone()))
///     .nest("/user", router!("handlers::user", db));
/// ```
#[proc_macro]
pub fn router(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as router_macro::RouterMacroArgs);
    router_macro::expand_router(args).into()
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

/// Derive macro for registering error enum variants with orpc.
///
/// Annotate error enums to automatically generate TypeScript `.errors({...})`
/// entries in the contract. The macro registers variant names and data schemas
/// via `inventory::submit!`, allowing `generate_contract()` to discover them.
///
/// # Example
///
/// ```rust,ignore
/// use orpc::OrpcErrors;
///
/// #[derive(OrpcErrors)]
/// pub enum AppError {
///     NotFound,
///     Conflict { reason: String },
///     DatabaseError(String),
/// }
/// ```
///
/// When a handler returns `Result<T, AppError>`, the generated TypeScript
/// contract includes:
///
/// ```typescript
/// .errors({
///   NOT_FOUND: {},
///   CONFLICT: {
///     data: z.object({ reason: z.string() })
///   },
///   DATABASE_ERROR: {
///     data: z.string()
///   }
/// })
/// ```
///
/// # Variant Types
///
/// - **Unit variants**: `NotFound` → `NOT_FOUND: {}`
/// - **Struct variants**: `Conflict { reason: String }` → `CONFLICT: { data: z.object({...}) }`
/// - **Tuple variants**: `DatabaseError(String)` → `DATABASE_ERROR: { data: z.string() }`
///
/// # Type Mapping
///
/// Rust types are mapped to Zod schemas:
/// - `String` → `z.string()`
/// - `i32`, `u32`, etc. → `z.number()`
/// - `bool` → `z.boolean()`
/// - `Option<T>` → `<T_schema>.optional()`
/// - `Vec<T>` → `z.array(<T_schema>)`
/// - Custom types → `<TypeName>Schema` (must have `#[derive(ZodTs)]`)
#[proc_macro_derive(OrpcErrors)]
pub fn derive_orpc_errors(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    match error_derive::expand_orpc_errors(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
