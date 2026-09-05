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
//! ## Axum Extension Extractors
//!
//! Extract data from Axum's request extensions (added by middleware):
//!
//! ```rust,no_run
//! use orpc_axum::AxumExtension;
//! use orpc_core::{OrpcContext, OrpcError};
//!
//! #[derive(Clone)]
//! struct User { id: String }
//!
//! async fn get_profile(
//!     OrpcContext(ctx): OrpcContext<AppContext>,
//!     AxumExtension(user): AxumExtension<User>,
//! ) -> Result<String, OrpcError> {
//!     Ok(format!("Profile for user {}", user.id))
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
//! Enable the `better-auth` feature and use extractors:
//!
//! ```rust,ignore
//! use orpc_axum::{AxumRouter, BetterAuthSession};
//!
//! async fn get_profile(
//!     OrpcContext(ctx): OrpcContext<BaseContext>,
//!     BetterAuthSession(session): BetterAuthSession<AppAuthSchema>,
//! ) -> Result<Output, OrpcError> {
//!     // session is guaranteed to exist (401 if not authenticated)
//!     Ok(Output { email: session.user.email().to_string() })
//! }
//!
//! let app = orpc_router
//!     .into_axum_router(base_ctx)
//!     .with_better_auth(auth);
//! ```

#[cfg(feature = "better-auth")]
pub mod better_auth;

#[cfg(feature = "better-auth")]
pub use better_auth::{BetterAuthExt, BetterAuthSession, OptionalBetterAuthSession};

use async_trait::async_trait;
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
use orpc_core::{FromOrpcRequest, HttpMethod, OrpcError, OutputKind, ProcedureRegistry, Router};
use std::{convert::Infallible, pin::Pin, sync::Arc, time::Duration};
use tokio_stream::StreamExt;
use tower_http::cors::{Any, CorsLayer};

/// Extract a value from Axum's request extensions.
///
/// This allows orpc handlers to access data injected by Axum middleware:
///
/// ```rust,ignore
/// use orpc_axum::AxumExtension;
///
/// #[derive(Clone)]
/// struct User { id: String }
///
/// async fn handler(
///     AxumExtension(user): AxumExtension<User>,
/// ) -> Result<String, OrpcError> {
///     Ok(format!("Hello, user {}", user.id))
/// }
/// ```
///
/// Returns `OrpcError::internal` if the extension is not present.
pub struct AxumExtension<T>(pub T);

#[async_trait]
impl<Ctx, T> FromOrpcRequest<Ctx> for AxumExtension<T>
where
    Ctx: Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
{
    async fn from_request(
        ctx: Ctx,
        input: serde_json::Value,
        extensions: Option<&Arc<dyn std::any::Any + Send + Sync>>,
    ) -> Result<(Self, Ctx, serde_json::Value), OrpcError> {
        let extensions = extensions.ok_or_else(|| {
            OrpcError::internal("Extensions not available (not using Axum transport?)")
        })?;

        // Downcast from Arc<dyn Any> to Arc<axum::http::Extensions>
        let axum_extensions = extensions
            .downcast_ref::<axum::http::Extensions>()
            .ok_or_else(|| {
                OrpcError::internal("Failed to downcast extensions to Axum Extensions")
            })?;

        // Extract T from Axum Extensions
        let value = axum_extensions.get::<T>().cloned().ok_or_else(|| {
            OrpcError::internal(format!(
                "Extension type '{}' not found in request extensions",
                std::any::type_name::<T>()
            ))
        })?;

        Ok((AxumExtension(value), ctx, input))
    }
}

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
                let extensions = Arc::new(extensions);
                async move { handle_procedure(registry, ctx, key, input, extensions).await }
            };

        let registry_clone2 = Arc::clone(&registry);
        let route_path_clone2 = route_path.clone();
        let extractor_clone2 = extractor.clone();

        // GET/DELETE: no body
        let handler_no_body = move |state: State<Arc<Ctx>>, extensions: axum::http::Extensions| {
            let registry = Arc::clone(&registry_clone2);
            let key = route_path_clone2.clone();
            let ctx = extractor_clone2((*state.0).clone(), &extensions);
            let extensions = Arc::new(extensions);
            async move {
                handle_procedure(registry, ctx, key, serde_json::Value::Null, extensions).await
            }
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
    extensions: Arc<axum::http::Extensions>,
) -> Result<Response, AxumError>
where
    Ctx: Clone + Send + Sync + 'static,
{
    // Convert Axum Extensions to type-erased format for orpc-core
    let extensions_any: Arc<dyn std::any::Any + Send + Sync> = extensions;
    let result = registry
        .call(&key, ctx, input, Some(&extensions_any))
        .await?;

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
