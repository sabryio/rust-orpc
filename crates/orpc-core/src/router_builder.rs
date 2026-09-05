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
