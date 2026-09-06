//! Thin proc-macro bridge for [`rorpc_parse`].
//!
//! This crate contains only proc-macro entry points. All parsing, validation,
//! and code generation logic lives in `rorpc-parse` where it can be tested
//! with normal `#[test]` functions.

use proc_macro::TokenStream;
use syn::parse_macro_input;

/// Annotate an Axum handler with explicit HTTP method and path.
///
/// This is the canonical syntax for specifying both method and path explicitly.
/// The function remains a valid Axum handler with metadata registered for contract
/// generation and router discovery.
///
/// # Syntax
///
/// ```rust,ignore
/// #[rorpc::route(method = "POST", path = "/planet/list")]
/// async fn list_planets(State(db): State<Db>) -> Json<Vec<Planet>> {
///     Json(db.list().await)
/// }
/// ```
///
/// # Arguments
///
/// - `method` — HTTP method string (`"GET"`, `"POST"`, etc.), normalized to uppercase. Required.
/// - `path` — Route path string (e.g. `"/planet/list"`). Required.
/// - `data` — String literal naming the SSE data payload type for streaming handlers (e.g. `"StreamEvent"`). Optional.
///
/// # For Shorthand Syntax
///
/// Consider using method-specific macros for brevity:
/// - `#[rorpc::get("/path")]`
/// - `#[rorpc::post("/path")]`
/// - `#[rorpc::put("/path")]`
/// - `#[rorpc::patch("/path")]`
/// - `#[rorpc::delete("/path")]`
#[proc_macro_attribute]
pub fn route(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as rorpc_parse::codegen::OrpcArgs);
    let func = parse_macro_input!(item as syn::ItemFn);
    rorpc_parse::codegen::expand_orpc(args, func).into()
}

/// Shorthand for `#[orpc::route(method = "GET", path = "...")]`.
///
/// # Syntax
///
/// ```rust,ignore
/// #[orpc::get("/planet/list")]
/// async fn list_planets(State(db): State<Db>) -> Json<Vec<Planet>> {
///     Json(db.list().await)
/// }
/// ```
///
/// # With streaming data type
///
/// ```rust,ignore
/// #[orpc::get("/stream", data = "StreamEvent")]
/// async fn stream_events() -> Sse<impl Stream<Item = Event>> {
///     // data takes a string literal for IDE support
/// }
/// ```
#[proc_macro_attribute]
pub fn get(attr: TokenStream, item: TokenStream) -> TokenStream {
    let shorthand_args = parse_macro_input!(attr as rorpc_parse::codegen::MethodShorthandArgs);
    let func = parse_macro_input!(item as syn::ItemFn);
    let args = shorthand_args.into_orpc_args("GET");
    rorpc_parse::codegen::expand_orpc(args, func).into()
}

/// Shorthand for `#[orpc::route(method = "POST", path = "...")]`.
///
/// # Syntax
///
/// ```rust,ignore
/// #[orpc::post("/planet/create")]
/// async fn create_planet(
///     State(db): State<Db>,
///     Json(input): Json<CreateInput>,
/// ) -> Result<Json<Planet>, AppError> {
///     // ...
/// }
/// ```
#[proc_macro_attribute]
pub fn post(attr: TokenStream, item: TokenStream) -> TokenStream {
    let shorthand_args = parse_macro_input!(attr as rorpc_parse::codegen::MethodShorthandArgs);
    let func = parse_macro_input!(item as syn::ItemFn);
    let args = shorthand_args.into_orpc_args("POST");
    rorpc_parse::codegen::expand_orpc(args, func).into()
}

/// Shorthand for `#[orpc::route(method = "PUT", path = "...")]`.
///
/// # Syntax
///
/// ```rust,ignore
/// #[orpc::put("/planet/{id}")]
/// async fn update_planet(
///     State(db): State<Db>,
///     Json(input): Json<UpdateInput>,
/// ) -> Result<Json<Planet>, AppError> {
///     // ...
/// }
/// ```
#[proc_macro_attribute]
pub fn put(attr: TokenStream, item: TokenStream) -> TokenStream {
    let shorthand_args = parse_macro_input!(attr as rorpc_parse::codegen::MethodShorthandArgs);
    let func = parse_macro_input!(item as syn::ItemFn);
    let args = shorthand_args.into_orpc_args("PUT");
    rorpc_parse::codegen::expand_orpc(args, func).into()
}

/// Shorthand for `#[orpc::route(method = "PATCH", path = "...")]`.
///
/// # Syntax
///
/// ```rust,ignore
/// #[orpc::patch("/planet/{id}")]
/// async fn patch_planet(
///     State(db): State<Db>,
///     Json(input): Json<PatchInput>,
/// ) -> Result<Json<Planet>, AppError> {
///     // ...
/// }
/// ```
#[proc_macro_attribute]
pub fn patch(attr: TokenStream, item: TokenStream) -> TokenStream {
    let shorthand_args = parse_macro_input!(attr as rorpc_parse::codegen::MethodShorthandArgs);
    let func = parse_macro_input!(item as syn::ItemFn);
    let args = shorthand_args.into_orpc_args("PATCH");
    rorpc_parse::codegen::expand_orpc(args, func).into()
}

/// Shorthand for `#[orpc::route(method = "DELETE", path = "...")]`.
///
/// # Syntax
///
/// ```rust,ignore
/// #[orpc::delete("/planet/{id}")]
/// async fn delete_planet(
///     State(db): State<Db>,
///     Json(input): Json<DeleteInput>,
/// ) -> Result<Json<()>, AppError> {
///     // ...
/// }
/// ```
#[proc_macro_attribute]
pub fn delete(attr: TokenStream, item: TokenStream) -> TokenStream {
    let shorthand_args = parse_macro_input!(attr as rorpc_parse::codegen::MethodShorthandArgs);
    let func = parse_macro_input!(item as syn::ItemFn);
    let args = shorthand_args.into_orpc_args("DELETE");
    rorpc_parse::codegen::expand_orpc(args, func).into()
}

/// Auto-discovery router macro with optional module path filtering.
///
/// Discovers all `#[rorpc]`-annotated handlers via the `inventory` crate and
/// builds an Axum `Router`. Accepts an optional state expression and/or a
/// module path pattern to restrict which handlers are included.
///
/// # Syntax
///
/// ```text
/// router!()                            // all handlers, no state
/// router!(state)                       // all handlers, with state
/// router!("pattern")                   // filtered, no state
/// router!("pattern", state)            // filtered + state (any order)
/// router!(state, "pattern")            // filtered + state (any order)
/// router!(["pat1", "pat2"])            // multiple patterns
/// router!("prefix::{a,b}")             // brace expansion
/// router!("prefix::*")                 // wildcard
/// ```
///
/// # Pattern matching
///
/// Patterns match against the handler's `module_path!()` value:
/// - `"handlers::planet"` — exact module or any child
/// - `"handlers::*"` — all direct and nested children of `handlers::`
/// - `"handlers::{planet,user}"` — brace expansion
/// - `["handlers::planet", "api::v1"]` — explicit list
#[proc_macro]
pub fn router(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as rorpc_parse::codegen::RouterArgs);
    rorpc_parse::codegen::expand_router(args).into()
}

/// Derive macro that generates a `fn zod_ts() -> String` method on structs and enums.
///
/// The generated method returns a complete TypeScript block with a Zod schema
/// and a `z.infer` type alias. An `inventory::submit!` call registers the real
/// schema so contract generation prefers it over the `z.unknown()` fallback
/// emitted by `#[rorpc]`.
///
/// # Example
///
/// ```rust,ignore
/// #[derive(Serialize, Deserialize, ZodTs)]
/// pub struct Planet {
///     pub id: i32,
///     #[zod(min_length(1), max_length(100))]
///     pub name: String,
///     pub description: Option<String>,
/// }
/// ```
///
/// # Supported `#[zod(...)]` field attributes
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
    let input = parse_macro_input!(input as syn::DeriveInput);
    match rorpc_parse::codegen::derive_zod_ts(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derive macro for registering error enum variants with rorpc.
///
/// Annotate an error enum so `generate_contract()` can emit TypeScript
/// `.errors({...})` entries. Variant names are converted to `SCREAMING_SNAKE_CASE`.
///
/// # Example
///
/// ```rust,ignore
/// #[derive(OrpcError)]
/// pub enum AppError {
///     NotFound,
///     Conflict { reason: String },
///     DatabaseError(String),
/// }
/// ```
///
/// # Variant mapping
///
/// - Unit variants: `NotFound` → `NOT_FOUND: {}`
/// - Struct variants: `Conflict { reason: String }` → `CONFLICT: { data: z.object({...}) }`
/// - Tuple variants: `DatabaseError(String)` → `DATABASE_ERROR: { data: z.string() }`
#[proc_macro_derive(OrpcError)]
pub fn derive_orpc_errors(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    match rorpc_parse::codegen::expand_orpc_errors(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
