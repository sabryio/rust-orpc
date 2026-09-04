//! Procedure registry for runtime dispatch.

use crate::route::RouteMetadata;
use crate::{OrpcError, OutputKind, Procedure, ProcedureHandler};
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
    pub fn insert<In, Out>(&mut self, path: impl Into<String>, procedure: &Procedure<Ctx, In, Out>)
    where
        In: serde::de::DeserializeOwned + Send + 'static,
        Out: serde::Serialize + Send + 'static,
    {
        let path = path.into();
        let entry = RegistryEntry {
            handler: Box::new(procedure.clone()),
            route: procedure.route.clone(),
        };
        self.entries.insert(path, entry);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::os;
    use crate::route::HttpMethod;
    use serde::{Deserialize, Serialize};

    #[derive(Clone)]
    struct TestContext {
        value: i32,
    }

    #[derive(Deserialize, Serialize)]
    struct Input {
        x: i32,
    }

    #[tokio::test]
    async fn test_registry_insert_and_call() {
        let mut registry = ProcedureRegistry::<TestContext>::new();
        let proc = os()
            .context::<TestContext>()
            .route(HttpMethod::Post, "/add")
            .input::<Input>()
            .output::<i32>()
            .handler(|ctx: TestContext, input: Input| async move { Ok(ctx.value + input.x) });

        registry.insert("/add", &proc);

        assert!(registry.has("/add"));
        assert_eq!(registry.len(), 1);

        let ctx = TestContext { value: 10 };
        let result = registry
            .call("/add", ctx, serde_json::json!({ "x": 32 }))
            .await;

        assert!(result.is_ok());
        match result.unwrap() {
            OutputKind::Single(v) => assert_eq!(v, 42),
            OutputKind::Stream(_) => panic!("Expected Single"),
        }
    }

    #[tokio::test]
    async fn test_registry_routes_iterator() {
        let mut registry = ProcedureRegistry::<TestContext>::new();

        let ping = os()
            .context::<TestContext>()
            .route(HttpMethod::Get, "/ping")
            .output::<String>()
            .handler(|_ctx: TestContext, _: ()| async { Ok("pong".to_string()) });

        let create = os()
            .context::<TestContext>()
            .route(HttpMethod::Post, "/items")
            .output::<String>()
            .handler(|_ctx: TestContext, _: ()| async { Ok("created".to_string()) });

        registry.insert("/ping", &ping);
        registry.insert("/items", &create);

        let routes: HashMap<&String, &RouteMetadata> = registry.routes().collect();

        let ping_meta = routes.get(&"/ping".to_string()).unwrap();
        assert_eq!(ping_meta.method, HttpMethod::Get);
        assert_eq!(ping_meta.path, "/ping");

        let create_meta = routes.get(&"/items".to_string()).unwrap();
        assert_eq!(create_meta.method, HttpMethod::Post);
        assert_eq!(create_meta.path, "/items");
    }

    #[tokio::test]
    async fn test_registry_not_found() {
        let registry = ProcedureRegistry::<TestContext>::new();
        let ctx = TestContext { value: 0 };

        let result = registry.call("/nonexistent", ctx, Value::Null).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, "NOT_FOUND");
    }

    #[tokio::test]
    async fn test_registry_multiple_procedures() {
        let mut registry = ProcedureRegistry::<TestContext>::new();

        let ping = os()
            .context::<TestContext>()
            .route(HttpMethod::Get, "/ping")
            .output::<String>()
            .handler(|_ctx: TestContext, _: ()| async { Ok("pong".to_string()) });

        let double = os()
            .context::<TestContext>()
            .route(HttpMethod::Post, "/math/double")
            .input::<Input>()
            .output::<i32>()
            .handler(|_ctx: TestContext, input: Input| async move { Ok(input.x * 2) });

        registry.insert("/ping", &ping);
        registry.insert("/math/double", &double);

        assert_eq!(registry.len(), 2);
        assert!(registry.has("/ping"));
        assert!(registry.has("/math/double"));
    }

    #[tokio::test]
    async fn test_registry_is_empty() {
        let mut registry = ProcedureRegistry::<TestContext>::new();
        assert!(registry.is_empty());

        let proc = os()
            .context::<TestContext>()
            .route(HttpMethod::Get, "/test")
            .output::<String>()
            .handler(|_ctx: TestContext, _: ()| async { Ok("test".to_string()) });

        registry.insert("/test", &proc);
        assert!(!registry.is_empty());
    }
}
