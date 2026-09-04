//! Fluent router builder — define routers without structs or trait impls.
//!
//! `RouterBuilder` is the Rust equivalent of TypeScript oRPC's plain-object
//! router pattern. Instead of defining a struct and implementing `Router<Ctx>`,
//! you compose procedures and nested routers inline.
//!
//! # Example
//!
//! ```rust
//! use orpc_core::{os, r, OrpcError};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Clone)]
//! struct Ctx { name: String }
//!
//! #[derive(Deserialize)]
//! struct FindInput { id: i32 }
//!
//! #[derive(Serialize)]
//! struct Planet { id: i32, name: String }
//!
//! let planet = r()
//!     .add("list", os()
//!         .context::<Ctx>()
//!         .output::<Vec<Planet>>()
//!         .handler(|_ctx, _: ()| async { Ok(vec![]) }))
//!     .add("find", os()
//!         .context::<Ctx>()
//!         .input::<FindInput>()
//!         .output::<Planet>()
//!         .handler(|_ctx, input: FindInput| async move {
//!             Ok(Planet { id: input.id, name: "Earth".to_string() })
//!         }));
//!
//! let router = r()
//!     .add("ping", os()
//!         .context::<Ctx>()
//!         .output::<String>()
//!         .handler(|ctx: Ctx, _: ()| async move { Ok(ctx.name) }))
//!     .nest("planet", planet);
//! ```

use crate::{Procedure, ProcedureRegistry, Router};
use std::marker::PhantomData;

// ---------------------------------------------------------------------------
// Internal entry trait — type-erased so procedures and nested routers can
// live in the same Vec.
// ---------------------------------------------------------------------------

trait RouterEntry<Ctx>: Send + Sync
where
    Ctx: Clone + Send + 'static,
{
    fn register(&self, full_path: &str, registry: &mut ProcedureRegistry<Ctx>);
}

// A single procedure entry
struct ProcEntry<Ctx, In, Out> {
    proc: Procedure<Ctx, In, Out>,
}

impl<Ctx, In, Out> RouterEntry<Ctx> for ProcEntry<Ctx, In, Out>
where
    Ctx: Clone + Send + Sync + 'static,
    In: serde::de::DeserializeOwned + Send + 'static,
    Out: serde::Serialize + Send + 'static,
{
    fn register(&self, full_path: &str, registry: &mut ProcedureRegistry<Ctx>) {
        registry.insert(full_path, &self.proc);
    }
}

// A nested router entry
struct NestEntry<Ctx> {
    inner: Box<dyn Router<Ctx> + Send + Sync>,
}

impl<Ctx> RouterEntry<Ctx> for NestEntry<Ctx>
where
    Ctx: Clone + Send + 'static,
{
    fn register(&self, full_path: &str, registry: &mut ProcedureRegistry<Ctx>) {
        self.inner.register_procedures(full_path, registry);
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Fluent builder for composing RPC routers without struct definitions.
///
/// # SRP: owns path→entry mapping only; dispatch is delegated to ProcedureRegistry
pub struct RouterBuilder<Ctx> {
    // Each entry is (key, type-erased entry)
    entries: Vec<(String, Box<dyn RouterEntry<Ctx>>)>,
    _phantom: PhantomData<Ctx>,
}

impl<Ctx> RouterBuilder<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    /// Creates an empty router builder.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            _phantom: PhantomData,
        }
    }

    /// Adds a procedure at the given key.
    ///
    /// The key becomes the final path segment. Slashes are supported for
    /// sub-paths within a single level, but prefer `.nest()` for grouping.
    ///
    /// # Example
    ///
    /// ```rust
    /// use orpc_core::{os, r};
    ///
    /// #[derive(Clone)]
    /// struct Ctx;
    ///
    /// let router = r()
    ///     .add("ping", os()
    ///         .context::<Ctx>()
    ///         .output::<String>()
    ///         .handler(|_ctx, _: ()| async { Ok("pong".to_string()) }));
    /// ```
    pub fn add<In, Out>(mut self, key: impl Into<String>, proc: Procedure<Ctx, In, Out>) -> Self
    where
        In: serde::de::DeserializeOwned + Send + 'static,
        Out: serde::Serialize + Send + 'static,
    {
        self.entries
            .push((key.into(), Box::new(ProcEntry { proc })));
        self
    }

    /// Nests another router under the given key prefix.
    ///
    /// All paths in `nested` are prefixed with `key/`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use orpc_core::{os, r};
    ///
    /// #[derive(Clone)]
    /// struct Ctx;
    ///
    /// let planet = r()
    ///     .add("list", os()
    ///         .context::<Ctx>()
    ///         .output::<String>()
    ///         .handler(|_ctx, _: ()| async { Ok("[]".to_string()) }));
    ///
    /// let router = r().nest("planet", planet);
    /// // Registers: "planet/list"
    /// ```
    pub fn nest(
        mut self,
        key: impl Into<String>,
        nested: impl Router<Ctx> + Send + Sync + 'static,
    ) -> Self {
        self.entries.push((
            key.into(),
            Box::new(NestEntry {
                inner: Box::new(nested),
            }),
        ));
        self
    }
}

impl<Ctx> Router<Ctx> for RouterBuilder<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    fn register_procedures(&self, prefix: &str, registry: &mut ProcedureRegistry<Ctx>) {
        for (key, entry) in &self.entries {
            let full_path = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{}/{}", prefix, key)
            };
            entry.register(&full_path, registry);
        }
    }
}

impl<Ctx> Default for RouterBuilder<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Shorthand entry point for `RouterBuilder::new()`, mirroring TypeScript's
/// plain-object router pattern.
///
/// # Example
///
/// ```rust
/// use orpc_core::{os, r};
///
/// #[derive(Clone)]
/// struct Ctx;
///
/// let router = r()
///     .add("ping", os()
///         .context::<Ctx>()
///         .output::<String>()
///         .handler(|_ctx, _: ()| async { Ok("pong".to_string()) }));
/// ```
pub fn r<Ctx>() -> RouterBuilder<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    RouterBuilder::new()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::os;

    #[derive(Clone)]
    struct TestCtx {
        value: i32,
    }

    #[derive(serde::Deserialize, serde::Serialize)]
    struct Input {
        x: i32,
    }

    #[test]
    fn test_add_single_procedure() {
        let router = r().add(
            "ping",
            os().context::<TestCtx>()
                .output::<String>()
                .handler(|_ctx: TestCtx, _: ()| async { Ok("pong".to_string()) }),
        );

        let mut registry = ProcedureRegistry::new();
        router.register_procedures("", &mut registry);

        assert!(registry.has("ping"));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_add_multiple_procedures() {
        let router = r()
            .add(
                "ping",
                os().context::<TestCtx>()
                    .output::<String>()
                    .handler(|_ctx: TestCtx, _: ()| async { Ok("pong".to_string()) }),
            )
            .add(
                "double",
                os().context::<TestCtx>()
                    .input::<Input>()
                    .output::<i32>()
                    .handler(|_ctx: TestCtx, input: Input| async move { Ok(input.x * 2) }),
            );

        let mut registry = ProcedureRegistry::new();
        router.register_procedures("", &mut registry);

        assert!(registry.has("ping"));
        assert!(registry.has("double"));
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_nest_sub_router() {
        let planet = r()
            .add(
                "list",
                os().context::<TestCtx>()
                    .output::<String>()
                    .handler(|_ctx: TestCtx, _: ()| async { Ok("[]".to_string()) }),
            )
            .add(
                "find",
                os().context::<TestCtx>()
                    .input::<Input>()
                    .output::<String>()
                    .handler(|_ctx: TestCtx, input: Input| async move {
                        Ok(format!("planet {}", input.x))
                    }),
            );

        let router = r()
            .add(
                "ping",
                os().context::<TestCtx>()
                    .output::<String>()
                    .handler(|_ctx: TestCtx, _: ()| async { Ok("pong".to_string()) }),
            )
            .nest("planet", planet);

        let mut registry = ProcedureRegistry::new();
        router.register_procedures("", &mut registry);

        assert!(registry.has("ping"));
        assert!(registry.has("planet/list"));
        assert!(registry.has("planet/find"));
        assert_eq!(registry.len(), 3);
    }

    #[test]
    fn test_deep_nesting() {
        let inner = r().add(
            "action",
            os().context::<TestCtx>()
                .output::<String>()
                .handler(|_ctx: TestCtx, _: ()| async { Ok("done".to_string()) }),
        );

        let middle = r().nest("inner", inner);
        let outer = r().nest("middle", middle);

        let mut registry = ProcedureRegistry::new();
        outer.register_procedures("", &mut registry);

        assert!(registry.has("middle/inner/action"));
    }

    #[tokio::test]
    async fn test_router_dispatch_end_to_end() {
        let router = r().add(
            "add",
            os().context::<TestCtx>()
                .input::<Input>()
                .output::<i32>()
                .handler(|ctx: TestCtx, input: Input| async move { Ok(ctx.value + input.x) }),
        );

        let mut registry = ProcedureRegistry::new();
        router.register_procedures("", &mut registry);

        let ctx = TestCtx { value: 10 };
        let result = registry
            .call("add", ctx, serde_json::json!({ "x": 32 }))
            .await;

        assert!(result.is_ok());
        match result.unwrap() {
            crate::OutputKind::Single(v) => assert_eq!(v, 42),
            crate::OutputKind::Stream(_) => panic!("Expected Single"),
        }
    }

    #[test]
    fn test_prefix_propagation() {
        let sub = r().add(
            "proc",
            os().context::<TestCtx>()
                .output::<String>()
                .handler(|_ctx: TestCtx, _: ()| async { Ok("ok".to_string()) }),
        );

        let mut registry = ProcedureRegistry::new();
        sub.register_procedures("api/v1", &mut registry);

        assert!(registry.has("api/v1/proc"));
    }
}
