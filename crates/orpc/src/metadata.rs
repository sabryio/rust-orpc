//! Handler metadata collected at link time via the `inventory` crate.
//!
//! Every `#[orpc(method, path)]`-annotated handler emits an `inventory::submit!`
//! call that registers a `HandlerMetadata` entry. `inventory::iter::<HandlerMetadata>`
//! then provides access to all registered entries from any file in the binary.

/// Metadata registered by the `#[orpc(method, path)]` attribute macro.
///
/// One instance is created per annotated handler function and registered
/// globally at link time — no central list required.
pub struct HandlerMetadata {
    /// Handler function name (e.g. `"list_planets"`)
    pub name: &'static str,
    /// HTTP method in uppercase (e.g. `"GET"`, `"POST"`)
    pub method: &'static str,
    /// HTTP path (e.g. `"/planet/list"`)
    pub path: &'static str,
    /// Fully qualified Rust type name of the input type
    pub input_type_name: &'static str,
    /// Fully qualified Rust type name of the output type
    pub output_type_name: &'static str,
    /// Rust module path of the handler (e.g. `"axum_native::handlers::planet"`)
    pub module_path: &'static str,
}

inventory::collect!(HandlerMetadata);
