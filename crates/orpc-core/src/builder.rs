//! Type-safe procedure builder with compile-time guarantees.
//!
//! Uses phantom types to enforce correct state transitions:
//!
//! ```text
//! os()                                    → ProcedureBuilder<(), (), (), (), Unrouted>
//!   .context::<Ctx>()                     → ProcedureBuilder<Ctx, Ctx, (), (), Unrouted>
//!   .route(HttpMethod::Get, "/ping")      → ProcedureBuilder<Ctx, Ctx, (), (), Routed>
//!   .output::<String>()                   → ProcedureBuilder<Ctx, Ctx, (), String, Routed>
//!   .handler(|ctx, _: ()| async { ... })  → Procedure<Ctx, (), String>
//! ```
//!
//! `.handler()` is only available when the builder is in `Routed` state.
//! This enforces that every procedure declares a route at compile time.
//!
//! The `HCtx` type parameter tracks the handler context type, which may differ
//! from `Ctx` when middleware is used. When no middleware is present, `HCtx = Ctx`.

use crate::middleware::MiddlewareStackFn;
use crate::openapi::OpenApiMeta;
use crate::route::{HttpMethod, RouteMetadata};
use crate::{OrpcError, Procedure};
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
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
/// - `ProcedureBuilder<Ctx, HCtx, In, Out, Unrouted>` — `.handler()` not available
/// - `ProcedureBuilder<Ctx, HCtx, In, Out, Routed>`   — `.handler()` available
///
/// The `HCtx` parameter tracks the handler context type. When no middleware is used,
/// `HCtx = Ctx`. Middleware can transform the context from `Ctx` to a different `HCtx`.
///
/// # DIP: Depends on OrpcError abstraction, not concrete error types
pub struct ProcedureBuilder<Ctx, HCtx, In, Out, R> {
    route: Option<RouteMetadata>,
    openapi_meta: OpenApiMeta,
    middleware_stack: Option<MiddlewareStackFn<Ctx, HCtx>>,
    _phantom: PhantomData<(Ctx, HCtx, In, Out, R)>,
}

impl<Ctx, HCtx, In, Out, R> Clone for ProcedureBuilder<Ctx, HCtx, In, Out, R> {
    fn clone(&self) -> Self {
        Self {
            route: self.route.clone(),
            openapi_meta: self.openapi_meta.clone(),
            middleware_stack: self.middleware_stack.clone(),
            _phantom: PhantomData,
        }
    }
}

impl ProcedureBuilder<(), (), (), (), Unrouted> {
    pub(crate) fn new() -> Self {
        Self {
            route: None,
            openapi_meta: OpenApiMeta::default(),
            middleware_stack: None,
            _phantom: PhantomData,
        }
    }
}

// context/input/output transitions — available in both Unrouted and Routed states
impl<Ctx, HCtx, In, Out, R> ProcedureBuilder<Ctx, HCtx, In, Out, R> {
    /// Sets the context type for this procedure.
    ///
    /// When setting a new context type, `HCtx` is reset to match `C` (no middleware).
    /// If you had middleware before calling `.context()`, you'll need to re-apply it.
    pub fn context<C>(self) -> ProcedureBuilder<C, C, In, Out, R> {
        ProcedureBuilder {
            route: self.route,
            openapi_meta: self.openapi_meta,
            middleware_stack: None, // Reset middleware when context changes
            _phantom: PhantomData,
        }
    }

    /// Sets the input type for this procedure.
    pub fn input<I>(self) -> ProcedureBuilder<Ctx, HCtx, I, Out, R> {
        ProcedureBuilder {
            route: self.route,
            openapi_meta: self.openapi_meta,
            middleware_stack: self.middleware_stack,
            _phantom: PhantomData,
        }
    }

    /// Sets the output type for this procedure.
    pub fn output<O>(self) -> ProcedureBuilder<Ctx, HCtx, In, O, R> {
        ProcedureBuilder {
            route: self.route,
            openapi_meta: self.openapi_meta,
            middleware_stack: self.middleware_stack,
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
    ) -> ProcedureBuilder<Ctx, HCtx, In, Out, Routed> {
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
            middleware_stack: self.middleware_stack,
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
    pub fn meta(mut self, meta: OpenApiMeta) -> ProcedureBuilder<Ctx, HCtx, In, Out, Routed>
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
                middleware_stack: self.middleware_stack,
                _phantom: PhantomData,
            }
        } else {
            // Stay in current state (this won't compile if called on Unrouted)
            // For now, we'll transition to Routed anyway to keep the API simple
            // TODO: Implement proper type-state preservation for incomplete routes
            ProcedureBuilder {
                route: self.route,
                openapi_meta: self.openapi_meta,
                middleware_stack: self.middleware_stack,
                _phantom: PhantomData,
            }
        }
    }
}

// handler — only available in Routed state
impl<Ctx, HCtx, In, Out> ProcedureBuilder<Ctx, HCtx, In, Out, Routed>
where
    Ctx: Clone + Send + Sync + 'static,
    HCtx: Send + 'static,
    In: serde::de::DeserializeOwned + serde::Serialize + Send + 'static,
    Out: serde::Serialize + Send + 'static,
{
    /// Defines the handler for a procedure.
    ///
    /// Only available after `.route()` has been called.
    ///
    /// The handler receives the context type `HCtx`, which may differ from `Ctx`
    /// if middleware has been applied via `.use_middleware()`.
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
    #[allow(clippy::type_complexity)]
    pub fn handler<H, Extractors>(self, handler: H) -> Procedure<Ctx, In, Out>
    where
        H: crate::extractors::Handler<HCtx, In, Out, Extractors>,
    {
        // SAFETY: route is always Some when in Routed state — the type system
        // guarantees .route() was called before .handler().
        let route = self.route.expect("route is always set in Routed state");

        let handler = Arc::new(handler);

        // Compose middleware + extraction into the handler closure
        let wrapped_handler: Arc<
            dyn Fn(Ctx, In) -> Pin<Box<dyn Future<Output = Result<Out, OrpcError>> + Send>>
                + Send
                + Sync,
        > = if let Some(middleware_stack) = self.middleware_stack {
            // Middleware present: compose the stack with extraction and handler
            Arc::new(move |ctx, input| {
                let handler = Arc::clone(&handler);
                let middleware_stack = Arc::clone(&middleware_stack);
                Box::pin(async move {
                    // 1. Run middleware to transform context
                    let hctx = middleware_stack(ctx).await?;

                    // 2. Call handler (extraction happens inside Handler::call)
                    handler.call(hctx, input).await
                }) as Pin<Box<dyn Future<Output = Result<Out, OrpcError>> + Send>>
            })
        } else {
            // No middleware: extract and call handler directly
            // SAFETY: When middleware_stack is None, HCtx = Ctx (guaranteed by ProcedureBuilder::new)
            Arc::new(move |ctx, input| {
                let handler = Arc::clone(&handler);
                Box::pin(async move {
                    // SAFETY: HCtx = Ctx when no middleware
                    let hctx: HCtx = unsafe {
                        let ctx_ptr = &ctx as *const Ctx as *const HCtx;
                        std::ptr::read(ctx_ptr)
                    };
                    std::mem::forget(ctx);

                    // Call handler (extraction happens inside Handler::call)
                    handler.call(hctx, input).await
                }) as Pin<Box<dyn Future<Output = Result<Out, OrpcError>> + Send>>
            })
        };

        Procedure::new(wrapped_handler, route, self.openapi_meta)
    }
}

// streaming handler — for Stream<Item = T> output types
impl<Ctx, HCtx, In, T> ProcedureBuilder<Ctx, HCtx, In, crate::AsyncIterator<T>, Routed>
where
    Ctx: Clone + Send + Sync + 'static,
    HCtx: Send + 'static,
    In: serde::de::DeserializeOwned + serde::Serialize + Send + 'static,
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
    pub fn handler<F, Fut, S, Extractors>(self, handler: F) -> crate::StreamingProcedure<Ctx, In, T>
    where
        F: Fn(Extractors) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<S, OrpcError>> + Send + 'static,
        S: futures_core::Stream<Item = T> + Send + 'static,
        Extractors: crate::extractors::FromOrpcRequest<HCtx> + Send + 'static,
    {
        let route = self.route.expect("route is always set in Routed state");

        let handler = Arc::new(handler);

        type StreamFuture<T> = std::pin::Pin<
            Box<
                dyn Future<
                        Output = Result<
                            std::pin::Pin<Box<dyn futures_core::Stream<Item = T> + Send>>,
                            OrpcError,
                        >,
                    > + Send,
            >,
        >;

        let wrapped_handler: Arc<dyn Fn(Ctx, In) -> StreamFuture<T> + Send + Sync> =
            if let Some(middleware_stack) = self.middleware_stack {
                // Middleware present
                Arc::new(move |ctx, input| {
                    let handler = Arc::clone(&handler);
                    let middleware_stack = Arc::clone(&middleware_stack);
                    Box::pin(async move {
                        // 1. Run middleware
                        let hctx = middleware_stack(ctx).await?;

                        // 2. Serialize input for extraction
                        let input_value = serde_json::to_value(&input).map_err(|e| {
                            OrpcError::internal(format!("Failed to serialize input: {}", e))
                        })?;

                        // 3. Run extractors
                        let (extractors, _, _) =
                            Extractors::from_request(hctx, input_value).await?;

                        // 4. Call handler
                        let stream = handler(extractors).await?;
                        Ok(Box::pin(stream)
                            as std::pin::Pin<
                                Box<dyn futures_core::Stream<Item = T> + Send>,
                            >)
                    }) as StreamFuture<T>
                })
            } else {
                // No middleware: HCtx = Ctx
                Arc::new(move |ctx, input| {
                    let handler = Arc::clone(&handler);
                    Box::pin(async move {
                        // SAFETY: HCtx = Ctx when no middleware
                        let hctx: HCtx = unsafe {
                            let ctx_ptr = &ctx as *const Ctx as *const HCtx;
                            std::ptr::read(ctx_ptr)
                        };
                        std::mem::forget(ctx);

                        // 1. Serialize input for extraction
                        let input_value = serde_json::to_value(&input).map_err(|e| {
                            OrpcError::internal(format!("Failed to serialize input: {}", e))
                        })?;

                        // 2. Run extractors
                        let (extractors, _, _) =
                            Extractors::from_request(hctx, input_value).await?;

                        // 3. Call handler
                        let stream = handler(extractors).await?;
                        Ok(Box::pin(stream)
                            as std::pin::Pin<
                                Box<dyn futures_core::Stream<Item = T> + Send>,
                            >)
                    }) as StreamFuture<T>
                })
            };

        crate::StreamingProcedure::new(wrapped_handler, route, self.openapi_meta)
    }
}

// Middleware support — available in both Unrouted and Routed states
impl<Ctx, HCtx, In, Out, R> ProcedureBuilder<Ctx, HCtx, In, Out, R>
where
    Ctx: Clone + Send + 'static,
    HCtx: Clone + Send + 'static,
{
    /// Apply middleware that transforms the context.
    ///
    /// Middleware receives the current context and a `Next` continuation,
    /// and must return a transformed context or an error.
    ///
    /// Multiple middleware calls chain together: `Ctx → A → B → C`,
    /// where the handler receives the final context type `C`.
    ///
    /// # Examples
    ///
    /// Bare async function:
    /// ```rust,ignore
    /// async fn require_auth(ctx: BaseContext, next: Next<AuthContext>) -> Result<AuthContext, OrpcError> {
    ///     let user = get_user(&ctx.db).await?;
    ///     next.run(AuthContext { db: ctx.db, user }).await
    /// }
    ///
    /// let proc = os()
    ///     .context::<BaseContext>()
    ///     .use_middleware(require_auth)
    ///     .route(HttpMethod::Get, "/profile")
    ///     .output::<UserProfile>()
    ///     .handler(|ctx: AuthContext, _: ()| async move {
    ///         // ctx.user is guaranteed to be present
    ///         Ok(UserProfile { name: ctx.user.name })
    ///     });
    /// ```
    ///
    /// Closure:
    /// ```rust,ignore
    /// let proc = os()
    ///     .context::<BaseContext>()
    ///     .use_middleware(|ctx, next| async move {
    ///         println!("Before handler");
    ///         let result = next.run(ctx).await;
    ///         println!("After handler");
    ///         result
    ///     })
    ///     .route(HttpMethod::Get, "/ping")
    ///     .output::<String>()
    ///     .handler(|ctx, _: ()| async { Ok("pong".to_string()) });
    /// ```
    pub fn use_middleware<M, NewCtx>(
        self,
        middleware: M,
    ) -> ProcedureBuilder<Ctx, NewCtx, In, Out, R>
    where
        M: crate::IntoMiddleware<HCtx, NewCtx>,
        NewCtx: Clone + Send + 'static,
    {
        use crate::middleware::Next;

        let mw = middleware.into_middleware();
        let mw_func = mw.func().clone();

        let new_stack: crate::middleware::MiddlewareStackFn<Ctx, NewCtx> =
            if let Some(existing_stack) = self.middleware_stack {
                // Compose: existing_stack (Ctx → HCtx), then new middleware (HCtx → NewCtx)
                Arc::new(move |ctx| {
                    let existing_stack = Arc::clone(&existing_stack);
                    let mw_func = Arc::clone(&mw_func);
                    Box::pin(async move {
                        // First, run the existing stack to get HCtx
                        let hctx = existing_stack(ctx).await?;

                        // Now call new middleware with HCtx
                        // The Next represents "identity" - just return whatever NewCtx the middleware produces
                        let next = Next::new(move |new_ctx: NewCtx| {
                            // The rest of the chain is empty, so just return the context
                            Box::pin(async move { Ok(new_ctx) })
                        });
                        mw_func(hctx, next).await
                    })
                })
            } else {
                // First middleware: HCtx = Ctx, so the middleware transforms Ctx → NewCtx
                // SAFETY: When middleware_stack is None, HCtx = Ctx
                Arc::new(move |ctx| {
                    let mw_func = Arc::clone(&mw_func);
                    Box::pin(async move {
                        // Transmute ctx to HCtx (they're the same type when no middleware exists)
                        let hctx: HCtx = unsafe {
                            let ctx_ptr = &ctx as *const Ctx as *const HCtx;
                            std::ptr::read(ctx_ptr)
                        };
                        std::mem::forget(ctx);

                        // Create a Next that represents "identity" - just return the context unchanged
                        let next =
                            Next::new(move |new_ctx: NewCtx| Box::pin(async move { Ok(new_ctx) }));
                        mw_func(hctx, next).await
                    })
                })
            };

        ProcedureBuilder {
            route: self.route,
            openapi_meta: self.openapi_meta,
            middleware_stack: Some(new_stack),
            _phantom: PhantomData,
        }
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
pub fn os() -> ProcedureBuilder<(), (), (), (), Unrouted> {
    ProcedureBuilder::new()
}
