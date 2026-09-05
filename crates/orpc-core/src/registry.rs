//! Procedure registry for runtime dispatch.

use crate::route::RouteMetadata;
use crate::{OrpcError, OutputKind,  ProcedureHandler};
use serde_json::Value;
use std::collections::HashMap;

/// A registered entry — handler + route metadata kept together.
struct RegistryEntry<Ctx> {
    handler: Box<dyn ProcedureHandler<Ctx>>,
    route: RouteMetadata,
}

/// Registry that holds type-erased procedures for O(1) runtime dispatch.
///
/// Flattens nested router structures into a HashMap at initialization time,
/// enabling efficient path-based lookup during request handling.
///
/// Transport adapters iterate `routes()` at startup to register HTTP/IPC
/// handlers at the correct method + path.
///
/// # SRP: Manages procedure storage and dispatch only
pub struct ProcedureRegistry<Ctx> {
    entries: HashMap<String, RegistryEntry<Ctx>>,
}

impl<Ctx> ProcedureRegistry<Ctx>
where
    Ctx: Clone + Send + 'static,
{
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Inserts a procedure at the given key path.
    ///
    /// The key is used for `call()` lookup. The route metadata carried by
    /// the procedure is stored alongside for transport adapters.
    pub fn insert<P>(&mut self, path: impl Into<String>, procedure: &P)
    where
        P: ProcedureHandler<Ctx> + Clone + 'static,
    {
        let path_str = path.into();
        let entry = RegistryEntry {
            handler: Box::new(procedure.clone()),
            route: procedure.route_metadata().clone(),
        };
        self.entries.insert(path_str, entry);
    }

    /// Calls a procedure by key path with JSON input.
    pub async fn call(&self, path: &str, ctx: Ctx, input: Value) -> Result<OutputKind, OrpcError> {
        let entry = self
            .entries
            .get(path)
            .ok_or_else(|| OrpcError::not_found(format!("No procedure at path: {}", path)))?;

        entry.handler.call(ctx, input).await
    }

    /// Checks if a procedure exists at the given key path.
    pub fn has(&self, path: &str) -> bool {
        self.entries.contains_key(path)
    }

    /// Returns the number of registered procedures.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns an iterator over all registered key paths.
    pub fn paths(&self) -> impl Iterator<Item = &String> {
        self.entries.keys()
    }

    /// Returns an iterator over `(key_path, &RouteMetadata)` for all procedures.
    ///
    /// Transport adapters use this at startup to register handlers at the
    /// correct HTTP method + absolute path.
    ///
    /// # Example
    ///
    /// ```rust
    /// use orpc_core::{os, router, HttpMethod, Router, ProcedureRegistry};
    ///
    /// #[derive(Clone)]
    /// struct Ctx;
    ///
    /// let app = router! {
    ///     ping: os()
    ///         .context::<Ctx>()
    ///         .route(HttpMethod::Get, "/ping")
    ///         .output::<String>()
    ///         .handler(|_ctx, _: ()| async { Ok("pong".to_string()) })
    /// };
    ///
    /// let mut registry = ProcedureRegistry::new();
    /// app.register_procedures("", &mut registry);
    ///
    /// for (key, meta) in registry.routes() {
    ///     println!("{} {} (key: {})", meta.method, meta.path, key);
    /// }
    /// ```
    pub fn routes(&self) -> impl Iterator<Item = (&String, &RouteMetadata)> {
        self.entries.iter().map(|(k, e)| (k, &e.route))
    }
}

impl<Ctx> Default for ProcedureRegistry<Ctx>
where
    Ctx: Clone + Send + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}
