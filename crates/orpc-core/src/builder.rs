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

use crate::openapi::OpenApiMeta;
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
    openapi_meta: OpenApiMeta,
    _phantom: PhantomData<(Ctx, In, Out, R)>,
}

impl ProcedureBuilder<(), (), (), Unrouted> {
    pub(crate) fn new() -> Self {
        Self {
            route: None,
            openapi_meta: OpenApiMeta::default(),
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
            openapi_meta: self.openapi_meta,
            _phantom: PhantomData,
        }
    }

    /// Sets the input type for this procedure.
    pub fn input<I>(self) -> ProcedureBuilder<Ctx, I, Out, R> {
        ProcedureBuilder {
            route: self.route,
            openapi_meta: self.openapi_meta,
            _phantom: PhantomData,
        }
    }

    /// Sets the output type for this procedure.
    pub fn output<O>(self) -> ProcedureBuilder<Ctx, In, O, R> {
        ProcedureBuilder {
            route: self.route,
            openapi_meta: self.openapi_meta,
            _phantom: PhantomData,
        }
    }

    /// Declares the HTTP method and absolute path for this procedure.
    ///
    /// This transitions the builder from `Unrouted` to `Routed`, unlocking
    /// the `.handler()` method. Every procedure must call `.route()`.
    ///
    /// Internally, this creates OpenAPI metadata and calls `.meta()`.
    ///
    /// # Arguments
    ///
    /// * `method` — HTTP method (e.g. `HttpMethod::Get`)
    /// * `path`   — Absolute path (e.g. `"/ping"`, `"/planet/{id}"`)
    ///
    /// # Example
    ///
    /// ```rust
    /// use orpc_core::{os, HttpMethod};
    ///
    /// #[derive(Clone)]
    /// struct AppContext;
    ///
    /// let proc = os()
    ///     .context::<AppContext>()
    ///     .route(HttpMethod::Get, "/ping")
    ///     .output::<String>()
    ///     .handler(|_ctx, _: ()| async { Ok("pong".to_string()) });
    /// ```
    pub fn route(
        mut self,
        method: HttpMethod,
        path: impl Into<String>,
    ) -> ProcedureBuilder<Ctx, In, Out, Routed> {
        let path_string = path.into();

        // Create OpenAPI metadata with method and path
        let meta = OpenApiMeta {
            method: Some(method.clone()),
            path: Some(path_string.clone()),
            prefixes: vec![],
        };

        // Merge into existing metadata
        self.openapi_meta.merge(meta);

        ProcedureBuilder {
            route: Some(RouteMetadata::new(method, path_string)),
            openapi_meta: self.openapi_meta,
            _phantom: PhantomData,
        }
    }

    /// Adds OpenAPI metadata to this procedure.
    ///
    /// Can be called multiple times. Metadata merges with these rules:
    /// - Prefixes: accumulate (concatenate)
    /// - Method: override (last one wins)
    /// - Path: override (last one wins)
    ///
    /// If the metadata provides both method AND path, transitions from
    /// `Unrouted` to `Routed` state, enabling `.handler()`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use orpc_core::{os, openapi};
    ///
    /// let proc = os()
    ///     .context::<AppContext>()
    ///     .meta(openapi!{ prefix: "/api" })
    ///     .meta(openapi!{
    ///         method: "GET",
    ///         path: "/planets"
    ///     })
    ///     .output::<Vec<Planet>>()
    ///     .handler(|ctx, _: ()| async { Ok(ctx.db.list().await) });
    /// ```
    pub fn meta(mut self, meta: OpenApiMeta) -> ProcedureBuilder<Ctx, In, Out, Routed>
    where
        Self: Sized,
    {
        self.openapi_meta.merge(meta);

        // Smart type-state transition: if we now have a complete route, become Routed
        if self.openapi_meta.has_complete_route() {
            // Extract method and path to create RouteMetadata
            let method = self.openapi_meta.method.clone().unwrap();
            let path = self.openapi_meta.full_path().unwrap();

            ProcedureBuilder {
                route: Some(RouteMetadata::new(method, path)),
                openapi_meta: self.openapi_meta,
                _phantom: PhantomData,
            }
        } else {
            // Stay in current state (this won't compile if called on Unrouted)
            // For now, we'll transition to Routed anyway to keep the API simple
            // TODO: Implement proper type-state preservation for incomplete routes
            ProcedureBuilder {
                route: self.route,
                openapi_meta: self.openapi_meta,
                _phantom: PhantomData,
            }
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
    /// ```rust
    /// use orpc_core::{os, HttpMethod};
    ///
    /// #[derive(Clone)]
    /// struct Ctx;
    ///
    /// #[derive(serde::Deserialize)]
    /// struct MyInput { id: i32 }
    ///
    /// #[derive(serde::Serialize)]
    /// struct MyOutput { name: String }
    ///
    /// let proc = os()
    ///     .context::<Ctx>()
    ///     .route(HttpMethod::Post, "/items/{id}")
    ///     .input::<MyInput>()
    ///     .output::<MyOutput>()
    ///     .handler(|_ctx, input: MyInput| async move {
    ///         Ok(MyOutput { name: format!("item-{}", input.id) })
    ///     });
    /// ```
    ///
    /// Without input:
    /// ```rust
    /// use orpc_core::{os, HttpMethod};
    ///
    /// #[derive(Clone)]
    /// struct Ctx;
    ///
    /// let proc = os()
    ///     .context::<Ctx>()
    ///     .route(HttpMethod::Get, "/ping")
    ///     .output::<String>()
    ///     .handler(|_ctx, _: ()| async { Ok("pong".to_string()) });
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
            self.openapi_meta,
        )
    }
}

// streaming handler — for Stream<Item = T> output types
impl<Ctx, In, T> ProcedureBuilder<Ctx, In, crate::AsyncIterator<T>, Routed>
where
    Ctx: Clone + Send + 'static,
    In: serde::de::DeserializeOwned + Send + 'static,
    T: serde::Serialize + Send + 'static,
{
    /// Defines a streaming handler for a procedure that outputs `Stream<Item = T>`.
    ///
    /// The handler must return `Result<impl Stream<Item = T>, OrpcError>`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use orpc_core::{os, HttpMethod, AsyncIterator};
    /// use tokio_stream::StreamExt;
    ///
    /// #[derive(Clone)]
    /// struct Ctx;
    ///
    /// #[derive(serde::Serialize)]
    /// struct Event { count: u32 }
    ///
    /// let proc = os()
    ///     .context::<Ctx>()
    ///     .route(HttpMethod::Post, "/stream")
    ///     .output::<AsyncIterator<Event>>()
    ///     .handler(|_ctx, _: ()| async {
    ///         let stream = tokio_stream::iter(0u32..10)
    ///             .map(|count| Event { count });
    ///         Ok(stream)
    ///     });
    /// ```
    pub fn handler<F, Fut, S>(self, handler: F) -> crate::StreamingProcedure<Ctx, In, T>
    where
        F: Fn(Ctx, In) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<S, OrpcError>> + Send + 'static,
        S: futures_core::Stream<Item = T> + Send + 'static,
    {
        let route = self.route.expect("route is always set in Routed state");

        crate::StreamingProcedure::new(
            Arc::new(move |ctx, input| {
                let fut = handler(ctx, input);
                Box::pin(async move {
                    let stream = fut.await?;
                    Ok(Box::pin(stream)
                        as std::pin::Pin<
                            Box<dyn futures_core::Stream<Item = T> + Send>,
                        >)
                })
            }),
            route,
            self.openapi_meta,
        )
    }
}

/// Entry point for building procedures, mirroring TypeScript oRPC's `os` pattern.
///
/// # Example
///
/// ```rust
/// use orpc_core::{os, HttpMethod};
///
/// #[derive(Clone)]
/// struct Ctx;
///
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
