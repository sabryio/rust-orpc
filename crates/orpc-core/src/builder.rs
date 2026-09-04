//! Type-safe procedure builder with compile-time guarantees.
//!
//! Uses phantom types to enforce correct state transitions:
//!
//! ```text
//! os()                                    → ProcedureBuilder<(), (), (), Unrouted>
//!   .context::<Ctx>()                     → ProcedureBuilder<Ctx, (), (), Unrouted>
//!   .route(HttpMethod::Get, "/ping")      → ProcedureBuilder<Ctx, (), (), Routed>
//!   .output::<String>()                   → ProcedureBuilder<Ctx, (), String, Routed>
//!   .handler(|ctx, _: ()| async { ... })  → Procedure<Ctx, (), String>
//! ```
//!
//! `.handler()` is only available when the builder is in `Routed` state.
//! This enforces that every procedure declares a route at compile time.

use crate::route::{HttpMethod, RouteMetadata};
use crate::{OrpcError, Procedure};
use std::future::Future;
use std::marker::PhantomData;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Type-state markers
// ---------------------------------------------------------------------------

/// Marker — procedure has no route declared yet. `.handler()` is unavailable.
pub struct Unrouted;

/// Marker — procedure has a route declared. `.handler()` is available.
pub struct Routed;

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Type-safe builder for constructing RPC procedures.
///
/// The `R` phantom type parameter enforces the route declaration requirement:
/// - `ProcedureBuilder<Ctx, In, Out, Unrouted>` — `.handler()` not available
/// - `ProcedureBuilder<Ctx, In, Out, Routed>`   — `.handler()` available
///
/// # DIP: Depends on OrpcError abstraction, not concrete error types
pub struct ProcedureBuilder<Ctx, In, Out, R> {
    route: Option<RouteMetadata>,
    _phantom: PhantomData<(Ctx, In, Out, R)>,
}

impl ProcedureBuilder<(), (), (), Unrouted> {
    pub(crate) fn new() -> Self {
        Self {
            route: None,
            _phantom: PhantomData,
        }
    }
}

// context/input/output transitions — available in both Unrouted and Routed states
impl<Ctx, In, Out, R> ProcedureBuilder<Ctx, In, Out, R> {
    /// Sets the context type for this procedure.
    pub fn context<C>(self) -> ProcedureBuilder<C, In, Out, R> {
        ProcedureBuilder {
            route: self.route,
            _phantom: PhantomData,
        }
    }

    /// Sets the input type for this procedure.
    pub fn input<I>(self) -> ProcedureBuilder<Ctx, I, Out, R> {
        ProcedureBuilder {
            route: self.route,
            _phantom: PhantomData,
        }
    }

    /// Sets the output type for this procedure.
    pub fn output<O>(self) -> ProcedureBuilder<Ctx, In, O, R> {
        ProcedureBuilder {
            route: self.route,
            _phantom: PhantomData,
        }
    }

    /// Declares the HTTP method and absolute path for this procedure.
    ///
    /// This transitions the builder from `Unrouted` to `Routed`, unlocking
    /// the `.handler()` method. Every procedure must call `.route()`.
    ///
    /// # Arguments
    ///
    /// * `method` — HTTP method (e.g. `HttpMethod::Get`)
    /// * `path`   — Absolute path (e.g. `"/ping"`, `"/planet/{id}"`)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// os()
    ///     .context::<AppContext>()
    ///     .route(HttpMethod::Get, "/ping")
    ///     .output::<String>()
    ///     .handler(|_ctx, _: ()| async { Ok("pong".to_string()) })
    /// ```
    pub fn route(
        self,
        method: HttpMethod,
        path: impl Into<String>,
    ) -> ProcedureBuilder<Ctx, In, Out, Routed> {
        ProcedureBuilder {
            route: Some(RouteMetadata::new(method, path)),
            _phantom: PhantomData,
        }
    }
}

// handler — only available in Routed state
impl<Ctx, In, Out> ProcedureBuilder<Ctx, In, Out, Routed>
where
    Ctx: Clone + Send + 'static,
    In: serde::de::DeserializeOwned + Send + 'static,
    Out: serde::Serialize + Send + 'static,
{
    /// Defines the handler for a procedure.
    ///
    /// Only available after `.route()` has been called.
    ///
    /// # Examples
    ///
    /// With input:
    /// ```rust,ignore
    /// os()
    ///     .context::<Ctx>()
    ///     .route(HttpMethod::Post, "/items/{id}")
    ///     .input::<MyInput>()
    ///     .output::<MyOutput>()
    ///     .handler(|ctx, input| async move { Ok(output) })
    /// ```
    ///
    /// Without input:
    /// ```rust,ignore
    /// os()
    ///     .context::<Ctx>()
    ///     .route(HttpMethod::Get, "/ping")
    ///     .output::<String>()
    ///     .handler(|_ctx, _: ()| async { Ok("pong".to_string()) })
    /// ```
    pub fn handler<F, Fut>(self, handler: F) -> Procedure<Ctx, In, Out>
    where
        F: Fn(Ctx, In) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Out, OrpcError>> + Send + 'static,
    {
        // SAFETY: route is always Some when in Routed state — the type system
        // guarantees .route() was called before .handler().
        let route = self.route.expect("route is always set in Routed state");

        Procedure::new(
            Arc::new(move |ctx, input| {
                let fut = handler(ctx, input);
                Box::pin(fut)
            }),
            route,
        )
    }
}

/// Entry point for building procedures, mirroring TypeScript oRPC's `os` pattern.
///
/// # Example
///
/// ```rust,ignore
/// use orpc_core::{os, HttpMethod};
///
/// # #[derive(Clone)]
/// # struct Ctx;
/// let proc = os()
///     .context::<Ctx>()
///     .route(HttpMethod::Get, "/ping")
///     .output::<String>()
///     .handler(|_ctx: Ctx, _: ()| async { Ok("pong".to_string()) });
/// ```
pub fn os() -> ProcedureBuilder<(), (), (), Unrouted> {
    ProcedureBuilder::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::route::HttpMethod;

    #[derive(Clone)]
    struct TestContext {
        value: i32,
    }

    #[test]
    fn test_builder_no_input() {
        let proc = os()
            .context::<TestContext>()
            .route(HttpMethod::Get, "/test")
            .output::<String>()
            .handler(|_ctx: TestContext, _: ()| async { Ok("test".to_string()) });

        let _: crate::Procedure<TestContext, (), String> = proc;
    }

    #[test]
    fn test_builder_with_input() {
        #[derive(serde::Deserialize)]
        struct Input {
            value: i32,
        }

        let proc = os()
            .context::<TestContext>()
            .route(HttpMethod::Post, "/test")
            .input::<Input>()
            .output::<String>()
            .handler(|_ctx: TestContext, input: Input| async move {
                Ok(format!("value: {}", input.value))
            });

        let _: crate::Procedure<TestContext, Input, String> = proc;
    }

    #[test]
    fn test_builder_route_metadata_stored() {
        let proc = os()
            .context::<TestContext>()
            .route(HttpMethod::Delete, "/items/{id}")
            .output::<String>()
            .handler(|_ctx: TestContext, _: ()| async { Ok("deleted".to_string()) });

        assert_eq!(proc.route.method, HttpMethod::Delete);
        assert_eq!(proc.route.path, "/items/{id}");
    }

    #[test]
    fn test_builder_order_independent() {
        #[derive(serde::Deserialize)]
        struct Input {
            x: i32,
        }

        // input before output
        let _proc1 = os()
            .context::<TestContext>()
            .route(HttpMethod::Post, "/add")
            .input::<Input>()
            .output::<i32>()
            .handler(|_ctx: TestContext, input: Input| async move { Ok(input.x * 2) });

        // output before input
        let _proc2 = os()
            .context::<TestContext>()
            .route(HttpMethod::Post, "/add")
            .output::<i32>()
            .input::<Input>()
            .handler(|_ctx: TestContext, input: Input| async move { Ok(input.x * 2) });
    }

    #[test]
    fn test_builder_route_before_output() {
        // route can be called before output
        let proc = os()
            .context::<TestContext>()
            .route(HttpMethod::Get, "/ping")
            .output::<String>()
            .handler(|_ctx: TestContext, _: ()| async { Ok("pong".to_string()) });

        assert_eq!(proc.route.method, HttpMethod::Get);
        assert_eq!(proc.route.path, "/ping");
    }

    #[test]
    fn test_builder_route_after_output() {
        // route can also be called after output
        let proc = os()
            .context::<TestContext>()
            .output::<String>()
            .route(HttpMethod::Get, "/ping")
            .handler(|_ctx: TestContext, _: ()| async { Ok("pong".to_string()) });

        assert_eq!(proc.route.method, HttpMethod::Get);
        assert_eq!(proc.route.path, "/ping");
    }
}
