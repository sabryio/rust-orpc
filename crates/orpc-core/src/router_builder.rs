//! Internal router builder — used exclusively by the `router!` macro expansion.
//!
//! `RouterBuilder` and `r()` are not part of the public API. Users define
//! routers via the `router!` macro, which expands to calls on this type.
//!
//! # Example (via macro — the only public interface)
//!
//! ```rust
//! use orpc_core::{os, router, HttpMethod};
//!
//! #[derive(Clone)]
//! struct Ctx;
//!
//! let app = router! {
//!     ping: os()
//!         .context::<Ctx>()
//!         .route(HttpMethod::Get, "/ping")
//!         .output::<String>()
//!         .handler(|_ctx, _: ()| async { Ok("pong".to_string()) })
//! };
//! ```

use crate::{Procedure, ProcedureRegistry, Router, StreamingProcedure};
use std::marker::PhantomData;

// ---------------------------------------------------------------------------
// Trait for types that can be added to a router
// ---------------------------------------------------------------------------

/// Internal trait for types that can be registered in a router.
pub trait IntoRouterEntry<Ctx>: Send + Sync
where
    Ctx: Clone + Send + 'static,
{
    fn register_in(&self, registry: &mut ProcedureRegistry<Ctx>);
    fn route_path(&self) -> &str;
}

impl<Ctx, In, Out> IntoRouterEntry<Ctx> for Procedure<Ctx, In, Out>
where
    Ctx: Clone + Send + Sync + 'static,
    In: serde::de::DeserializeOwned + Send + 'static,
    Out: serde::Serialize + Send + 'static,
{
    fn register_in(&self, registry: &mut ProcedureRegistry<Ctx>) {
        registry.insert(self.route.path.clone(), self);
    }

    fn route_path(&self) -> &str {
        &self.route.path
    }
}

impl<Ctx, In, T> IntoRouterEntry<Ctx> for StreamingProcedure<Ctx, In, T>
where
    Ctx: Clone + Send + Sync + 'static,
    In: serde::de::DeserializeOwned + Send + 'static,
    T: serde::Serialize + Send + 'static,
{
    fn register_in(&self, registry: &mut ProcedureRegistry<Ctx>) {
        registry.insert(self.route.path.clone(), self);
    }

    fn route_path(&self) -> &str {
        &self.route.path
    }
}

// ---------------------------------------------------------------------------
// Internal entry trait
// ---------------------------------------------------------------------------

trait RouterEntry<Ctx>: Send + Sync
where
    Ctx: Clone + Send + 'static,
{
    fn register(&self, full_path: &str, registry: &mut ProcedureRegistry<Ctx>);
}

struct ProcEntry<Ctx, In, Out> {
    proc: Procedure<Ctx, In, Out>,
}

impl<Ctx, In, Out> RouterEntry<Ctx> for ProcEntry<Ctx, In, Out>
where
    Ctx: Clone + Send + Sync + 'static,
    In: serde::de::DeserializeOwned + Send + 'static,
    Out: serde::Serialize + Send + 'static,
{
    fn register(&self, _full_path: &str, registry: &mut ProcedureRegistry<Ctx>) {
        // Registry key is the route path declared on the procedure,
        // not the nest hierarchy — hierarchy is organizational only.
        self.proc.register_in(registry);
    }
}

struct StreamingProcEntry<Ctx, In, T> {
    proc: StreamingProcedure<Ctx, In, T>,
}

impl<Ctx, In, T> RouterEntry<Ctx> for StreamingProcEntry<Ctx, In, T>
where
    Ctx: Clone + Send + Sync + 'static,
    In: serde::de::DeserializeOwned + Send + 'static,
    T: serde::Serialize + Send + 'static,
{
    fn register(&self, _full_path: &str, registry: &mut ProcedureRegistry<Ctx>) {
        self.proc.register_in(registry);
    }
}

struct GenericProcEntry<Ctx> {
    proc: Box<dyn IntoRouterEntry<Ctx>>,
}

impl<Ctx> RouterEntry<Ctx> for GenericProcEntry<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    fn register(&self, _full_path: &str, registry: &mut ProcedureRegistry<Ctx>) {
        self.proc.register_in(registry);
    }
}

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
// RouterBuilder — internal, used by router! macro expansion
// ---------------------------------------------------------------------------

/// Internal fluent builder produced by the `router!` macro.
///
/// Not part of the public API — use `router! { ... }` instead.
pub struct RouterBuilder<Ctx> {
    entries: Vec<(String, Box<dyn RouterEntry<Ctx>>)>,
    _phantom: PhantomData<Ctx>,
}

impl<Ctx> RouterBuilder<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    #[doc(hidden)]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            _phantom: PhantomData,
        }
    }

    #[doc(hidden)]
    pub fn add_procedure<P>(mut self, proc: P) -> Self
    where
        P: IntoRouterEntry<Ctx> + 'static,
    {
        let route_path = proc.route_path().to_string();
        self.entries.push((
            route_path.clone(),
            Box::new(GenericProcEntry {
                proc: Box::new(proc),
            }),
        ));
        self
    }

    #[doc(hidden)]
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

/// Internal entry point used by `router!` macro expansion.
///
/// Not part of the public API — use `router! { ... }` instead.
#[doc(hidden)]
pub fn r<Ctx>() -> RouterBuilder<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    RouterBuilder::new()
}

// ---------------------------------------------------------------------------
// Tests — use router! macro, not r() directly
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::os;
    use crate::route::HttpMethod;

    #[derive(Clone)]
    struct TestCtx {
        value: i32,
    }

    #[derive(serde::Deserialize, serde::Serialize)]
    struct Input {
        x: i32,
    }

    // Internal tests still use r() directly since they test RouterBuilder itself.

    #[test]
    fn test_add_single_procedure() {
        let router = r().add_procedure(
            os().context::<TestCtx>()
                .route(HttpMethod::Get, "/ping")
                .output::<String>()
                .handler(|_ctx: TestCtx, _: ()| async { Ok("pong".to_string()) }),
        );

        let mut registry = ProcedureRegistry::new();
        router.register_procedures("", &mut registry);

        assert!(registry.has("/ping"));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_add_multiple_procedures() {
        let router = r()
            .add_procedure(
                os().context::<TestCtx>()
                    .route(HttpMethod::Get, "/ping")
                    .output::<String>()
                    .handler(|_ctx: TestCtx, _: ()| async { Ok("pong".to_string()) }),
            )
            .add_procedure(
                os().context::<TestCtx>()
                    .route(HttpMethod::Post, "/double")
                    .input::<Input>()
                    .output::<i32>()
                    .handler(|_ctx: TestCtx, input: Input| async move { Ok(input.x * 2) }),
            );

        let mut registry = ProcedureRegistry::new();
        router.register_procedures("", &mut registry);

        assert!(registry.has("/ping"));
        assert!(registry.has("/double"));
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_nest_sub_router() {
        let planet = r()
            .add_procedure(
                os().context::<TestCtx>()
                    .route(HttpMethod::Get, "/planet")
                    .output::<String>()
                    .handler(|_ctx: TestCtx, _: ()| async { Ok("[]".to_string()) }),
            )
            .add_procedure(
                os().context::<TestCtx>()
                    .route(HttpMethod::Get, "/planet/{id}")
                    .input::<Input>()
                    .output::<String>()
                    .handler(|_ctx: TestCtx, input: Input| async move {
                        Ok(format!("planet {}", input.x))
                    }),
            );

        let router = r()
            .add_procedure(
                os().context::<TestCtx>()
                    .route(HttpMethod::Get, "/ping")
                    .output::<String>()
                    .handler(|_ctx: TestCtx, _: ()| async { Ok("pong".to_string()) }),
            )
            .nest("planet", planet);

        let mut registry = ProcedureRegistry::new();
        router.register_procedures("", &mut registry);

        assert!(registry.has("/ping"));
        assert!(registry.has("/planet"));
        assert!(registry.has("/planet/{id}"));
        assert_eq!(registry.len(), 3);
    }

    #[test]
    fn test_deep_nesting() {
        let inner = r().add_procedure(
            os().context::<TestCtx>()
                .route(HttpMethod::Post, "/action")
                .output::<String>()
                .handler(|_ctx: TestCtx, _: ()| async { Ok("done".to_string()) }),
        );

        let middle = r().nest("inner", inner);
        let outer = r().nest("middle", middle);

        let mut registry = ProcedureRegistry::new();
        outer.register_procedures("", &mut registry);

        assert!(registry.has("/action"));
    }

    #[tokio::test]
    async fn test_router_dispatch_end_to_end() {
        let router = r().add_procedure(
            os().context::<TestCtx>()
                .route(HttpMethod::Post, "/add")
                .input::<Input>()
                .output::<i32>()
                .handler(|ctx: TestCtx, input: Input| async move { Ok(ctx.value + input.x) }),
        );

        let mut registry = ProcedureRegistry::new();
        router.register_procedures("", &mut registry);

        let ctx = TestCtx { value: 10 };
        let result = registry
            .call("/add", ctx, serde_json::json!({ "x": 32 }))
            .await;

        assert!(result.is_ok());
        match result.unwrap() {
            crate::OutputKind::Single(v) => assert_eq!(v, 42),
            crate::OutputKind::Stream(_) => panic!("Expected Single"),
        }
    }

    #[test]
    fn test_prefix_propagation() {
        // With route-path keying, nesting prefix doesn't affect the registry key
        // (registry key = route path, not the nest hierarchy)
        let sub = r().add_procedure(
            os().context::<TestCtx>()
                .route(HttpMethod::Get, "/proc")
                .output::<String>()
                .handler(|_ctx: TestCtx, _: ()| async { Ok("ok".to_string()) }),
        );

        let mut registry = ProcedureRegistry::new();
        sub.register_procedures("api/v1", &mut registry);

        assert!(registry.has("/proc"));
    }
}
