//! # orpc-axum
//!
//! Axum integration for `orpc-core` — converts type-safe RPC procedure routers
//! into Axum routers using each procedure's declared route metadata.
//!
//! ## Features
//!
//! - `better-auth` - Better-Auth integration via `.with_better_auth()` extension method
//!
//! ## Basic usage
//!
//! ```rust,no_run
//! use orpc_core::{os, router, HttpMethod};
//! use orpc_axum::AxumRouter;
//!
//! #[derive(Clone)]
//! struct AppContext;
//!
//! #[tokio::main]
//! async fn main() {
//!     let app = router! {
//!         ping: os()
//!             .context::<AppContext>()
//!             .route(HttpMethod::Get, "/ping")
//!             .output::<String>()
//!             .handler(|_ctx, _: ()| async { Ok("pong".to_string()) })
//!     }
//!     .into_axum_router(AppContext);
//!
//!     let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
//!     axum::serve(listener, app).await.unwrap();
//! }
//! ```
//!
//! ## Per-request context enrichment (e.g. auth)
//!
//! Use `into_axum_router_with` when Axum middleware places per-request data
//! (such as an authenticated user) into request extensions and your context
//! needs to reflect it on every call:
//!
//! ```rust,no_run
//! use axum::http::Extensions;
//! use orpc_core::{os, router, HttpMethod};
//! use orpc_axum::AxumRouter;
//!
//! #[derive(Clone)]
//! struct User { id: String }
//!
//! #[derive(Clone)]
//! struct AppContext { current_user: Option<User> }
//!
//! fn extract_user(mut ctx: AppContext, ext: &Extensions) -> AppContext {
//!     ctx.current_user = ext.get::<User>().cloned();
//!     ctx
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let base_ctx = AppContext { current_user: None };
//!
//!     let app = router! {
//!         ping: os()
//!             .context::<AppContext>()
//!             .route(HttpMethod::Get, "/ping")
//!             .output::<String>()
//!             .handler(|ctx, _: ()| async move {
//!                 let who = ctx.current_user.map(|u| u.id).unwrap_or_else(|| "anon".into());
//!                 Ok(format!("pong from {who}"))
//!             })
//!     }
//!     .into_axum_router_with(base_ctx, extract_user);
//!
//!     let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
//!     axum::serve(listener, app).await.unwrap();
//! }
//! ```
//!
//! ## Better-Auth integration
//!
//! Enable the `better-auth` feature and use `.with_better_auth()`:
//!
//! ```rust,ignore
//! use orpc_axum::{AxumRouter, better_auth::BetterAuthExt};
//!
//! let app = orpc_router
//!     .into_axum_router_with(base_ctx, build_context)
//!     .with_better_auth(auth);
//! ```

#[cfg(feature = "better-auth")]
pub mod better_auth;

use axum::{
    extract::State,
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive},
        IntoResponse, Json, Response, Sse,
    },
    routing::{delete, get, patch, post, put},
    Router as AxumRouterType,
};
use futures::Stream;
use orpc_core::{HttpMethod, OrpcError, OutputKind, ProcedureRegistry, Router};
use std::{convert::Infallible, pin::Pin, sync::Arc, time::Duration};
use tokio_stream::StreamExt;
use tower_http::cors::{Any, CorsLayer};

/// Extension trait to convert orpc routers into Axum routers.
pub trait AxumRouter<Ctx>: Router<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    /// Converts this orpc router into an Axum router.
    ///
    /// Each procedure is registered at the HTTP method and absolute path
    /// declared via `.route()` on the procedure builder.
    fn into_axum_router(self, ctx: Ctx) -> AxumRouterType
    where
        Self: Sized,
    {
        build_axum_router(self, ctx, |ctx, _| ctx)
    }

    /// Converts this orpc router into an Axum router with a per-request
    /// context extractor.
    ///
    /// The `extractor` function is called on every request with a clone of
    /// the base context and the request's [`axum::http::Extensions`]. It
    /// returns a new context enriched with per-request data.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use axum::http::Extensions;
    /// use orpc_axum::AxumRouter;
    ///
    /// #[derive(Clone)]
    /// struct User { id: String }
    ///
    /// #[derive(Clone)]
    /// struct AppContext { current_user: Option<User> }
    ///
    /// fn extract_user(mut ctx: AppContext, ext: &Extensions) -> AppContext {
    ///     ctx.current_user = ext.get::<User>().cloned();
    ///     ctx
    /// }
    ///
    /// // router!{ ... }.into_axum_router_with(base_ctx, extract_user);
    /// ```
    fn into_axum_router_with<F>(self, ctx: Ctx, extractor: F) -> AxumRouterType
    where
        Self: Sized,
        F: Fn(Ctx, &axum::http::Extensions) -> Ctx + Clone + Send + Sync + 'static,
    {
        build_axum_router(self, ctx, extractor)
    }

    /// Converts this orpc router into an Axum router with automatic Better-Auth
    /// session injection.
    ///
    /// Requires the context to be `BetterAuthContext<Schema, InnerCtx>`.
    /// The schema type is inferred — no turbofish needed.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use orpc_axum::better_auth::{BetterAuthContext, BetterAuthExt};
    ///
    /// let base_ctx = BetterAuthContext::new(inner_ctx);
    ///
    /// let app = orpc_router
    ///     .into_axum_router_with_better_auth(base_ctx) // Schema inferred!
    ///     .with_better_auth(auth);
    /// ```
    #[cfg(feature = "better-auth")]
    fn into_axum_router_with_better_auth(self, ctx: Ctx) -> AxumRouterType
    where
        Self: Sized,
        Ctx: crate::better_auth::WithBetterAuth,
    {
        build_axum_router(self, ctx, |mut ctx, ext| {
            use ::better_auth::integrations::axum::OptionalSession;

            if let Some(session) = ext
                .get::<Arc<OptionalSession<<Ctx as crate::better_auth::WithBetterAuth>::Schema>>>()
            {
                ctx.inject_session(Arc::clone(session));
            }
            ctx
        })
    }
}

// Blanket impl for any type that implements Router
impl<T, Ctx> AxumRouter<Ctx> for T
where
    T: Router<Ctx>,
    Ctx: Clone + Send + Sync + 'static,
{
}

fn build_axum_router<R, Ctx, F>(router: R, ctx: Ctx, extractor: F) -> AxumRouterType
where
    R: Router<Ctx>,
    Ctx: Clone + Send + Sync + 'static,
    F: Fn(Ctx, &axum::http::Extensions) -> Ctx + Clone + Send + Sync + 'static,
{
    let mut registry = ProcedureRegistry::new();
    router.register_procedures("", &mut registry);

    let registry = Arc::new(registry);
    let context_arc = Arc::new(ctx);
    let mut axum_router = AxumRouterType::new();

    let routes: Vec<(String, HttpMethod, String)> = registry
        .routes()
        .map(|(key, meta)| (key.clone(), meta.method.clone(), meta.path.clone()))
        .collect();

    for (route_path, method, http_path) in routes {
        let registry_clone = Arc::clone(&registry);
        let route_path_clone = route_path.clone();
        let extractor_clone = extractor.clone();

        // POST/PUT/PATCH: body is optional — missing body treated as null input
        let handler =
            move |state: State<Arc<Ctx>>,
                  extensions: axum::http::Extensions,
                  body: Option<axum::extract::Json<serde_json::Value>>| {
                let registry = Arc::clone(&registry_clone);
                let key = route_path_clone.clone();
                let input = body.map(|b| b.0).unwrap_or(serde_json::Value::Null);
                let ctx = extractor_clone((*state.0).clone(), &extensions);
                async move { handle_procedure(registry, ctx, key, input).await }
            };

        let registry_clone2 = Arc::clone(&registry);
        let route_path_clone2 = route_path.clone();
        let extractor_clone2 = extractor.clone();

        // GET/DELETE: no body
        let handler_no_body = move |state: State<Arc<Ctx>>, extensions: axum::http::Extensions| {
            let registry = Arc::clone(&registry_clone2);
            let key = route_path_clone2.clone();
            let ctx = extractor_clone2((*state.0).clone(), &extensions);
            async move { handle_procedure(registry, ctx, key, serde_json::Value::Null).await }
        };

        axum_router = match method {
            HttpMethod::Get => axum_router.route(&http_path, get(handler_no_body)),
            HttpMethod::Post => axum_router.route(&http_path, post(handler)),
            HttpMethod::Put => axum_router.route(&http_path, put(handler)),
            HttpMethod::Patch => axum_router.route(&http_path, patch(handler)),
            HttpMethod::Delete => axum_router.route(&http_path, delete(handler_no_body)),
        };
    }

    axum_router
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(context_arc)
}

async fn handle_procedure<Ctx>(
    registry: Arc<ProcedureRegistry<Ctx>>,
    ctx: Ctx,
    key: String,
    input: serde_json::Value,
) -> Result<Response, AxumError>
where
    Ctx: Clone + Send + Sync + 'static,
{
    let result = registry.call(&key, ctx, input).await?;

    match result {
        OutputKind::Single(value) => Ok(Json(value).into_response()),
        OutputKind::Stream(stream) => {
            // Convert the stream into SSE format
            let sse_stream = stream_to_sse(stream);
            Ok(Sse::new(sse_stream)
                .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)).text(""))
                .into_response())
        }
    }
}

fn stream_to_sse(
    stream: Pin<Box<dyn Stream<Item = serde_json::Value> + Send>>,
) -> impl Stream<Item = Result<Event, Infallible>> {
    use async_stream::stream;

    stream! {
        // Send initial empty comment to flush headers immediately
        yield Ok(Event::default().comment(""));

        let mut stream = stream;

        while let Some(value) = stream.next().await {
            // Serialize the value to JSON string for SSE data field
            if let Ok(data) = serde_json::to_string(&value) {
                yield Ok(Event::default()
                    .event("message")
                    .data(data));
            }
        }

        // Send close event to signal end of stream
        yield Ok(Event::default()
            .event("close")
            .data(""));
    }
}

/// Axum-specific error wrapper that implements IntoResponse.
#[derive(Debug)]
enum AxumError {
    Orpc(OrpcError),
}

impl From<OrpcError> for AxumError {
    fn from(err: OrpcError) -> Self {
        AxumError::Orpc(err)
    }
}

impl IntoResponse for AxumError {
    fn into_response(self) -> Response {
        match self {
            AxumError::Orpc(err) => {
                let status = err
                    .status
                    .and_then(|s| StatusCode::from_u16(s).ok())
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

                let body = serde_json::json!({
                    "code": err.code,
                    "message": err.message,
                });

                (status, Json(body)).into_response()
            }
        }
    }
}
