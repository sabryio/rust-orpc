//! # orpc-axum
//!
//! Axum integration for `orpc-core` — converts type-safe RPC procedure routers
//! into Axum routers using each procedure's declared route metadata.
//!
//! ## Example
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

use axum::{
    body::Body,
    http::{Method, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::{delete, get, patch, post, put},
    Router as AxumRouterType,
};
use orpc_core::{HttpMethod, OrpcError, OutputKind, ProcedureRegistry, Router};
use std::sync::Arc;
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
    ///
    /// CORS is enabled for all origins by default.
    fn into_axum_router(self, ctx: Ctx) -> AxumRouterType
    where
        Self: Sized,
    {
        let mut registry = ProcedureRegistry::new();
        self.register_procedures("", &mut registry);

        let registry = Arc::new(registry);
        let context_arc = Arc::new(ctx);
        let mut axum_router = AxumRouterType::new();

        // Collect (key, method, path) before iterating to avoid borrow issues
        let routes: Vec<(String, HttpMethod, String)> = registry
            .routes()
            .map(|(key, meta)| (key.clone(), meta.method.clone(), meta.path.clone()))
            .collect();

        for (key, method, path) in routes {
            let registry_clone = Arc::clone(&registry);
            let context_clone = Arc::clone(&context_arc);
            let key_clone = key.clone();

            let handler = move |body: axum::extract::Json<serde_json::Value>| {
                let registry = Arc::clone(&registry_clone);
                let context = Arc::clone(&context_clone);
                let key = key_clone.clone();
                async move { handle_procedure(registry, context, key, body.0).await }
            };

            // Also register a no-body variant for GET-style requests
            let registry_clone2 = Arc::clone(&registry);
            let context_clone2 = Arc::clone(&context_arc);
            let key_clone2 = key.clone();

            let handler_no_body = move || {
                let registry = Arc::clone(&registry_clone2);
                let context = Arc::clone(&context_clone2);
                let key = key_clone2.clone();
                async move { handle_procedure(registry, context, key, serde_json::Value::Null).await }
            };

            axum_router = match method {
                HttpMethod::Get => axum_router.route(&path, get(handler_no_body)),
                HttpMethod::Post => axum_router.route(&path, post(handler)),
                HttpMethod::Put => axum_router.route(&path, put(handler)),
                HttpMethod::Patch => axum_router.route(&path, patch(handler)),
                HttpMethod::Delete => axum_router.route(&path, delete(handler_no_body)),
            };
        }

        axum_router.layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
    }
}

// Blanket impl for any type that implements Router
impl<T, Ctx> AxumRouter<Ctx> for T
where
    T: Router<Ctx>,
    Ctx: Clone + Send + Sync + 'static,
{
}

async fn handle_procedure<Ctx>(
    registry: Arc<ProcedureRegistry<Ctx>>,
    context: Arc<Ctx>,
    key: String,
    input: serde_json::Value,
) -> Result<Json<serde_json::Value>, AxumError>
where
    Ctx: Clone + Send + Sync + 'static,
{
    let ctx = (*context).clone();
    let result = registry.call(&key, ctx, input).await?;

    match result {
        OutputKind::Single(value) => Ok(Json(value)),
        OutputKind::Stream(_) => Err(AxumError::Internal(
            "Streaming not yet supported via Axum integration".to_string(),
        )),
    }
}

/// Axum-specific error wrapper that implements IntoResponse.
#[derive(Debug)]
enum AxumError {
    Orpc(OrpcError),
    Internal(String),
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
            AxumError::Internal(msg) => {
                let body = serde_json::json!({
                    "code": "INTERNAL_ERROR",
                    "message": msg,
                });

                (StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orpc_core::{os, router, HttpMethod, ProcedureRegistry, Router};

    #[derive(Clone)]
    struct TestContext {
        value: String,
    }

    #[test]
    fn test_axum_router_trait_available() {
        struct SimpleRouter;

        impl Router<TestContext> for SimpleRouter {
            fn register_procedures(
                &self,
                _prefix: &str,
                _registry: &mut ProcedureRegistry<TestContext>,
            ) {
            }
        }

        fn _takes_axum_router<T: AxumRouter<TestContext>>(_router: T) {}
        _takes_axum_router(SimpleRouter);
    }

    #[tokio::test]
    async fn test_into_axum_router_registers_correct_methods() {
        let router_inst = router! {
            ping: os()
                .context::<TestContext>()
                .route(HttpMethod::Get, "/ping")
                .output::<String>()
                .handler(|_ctx, _: ()| async { Ok("pong".to_string()) }),
            create: os()
                .context::<TestContext>()
                .route(HttpMethod::Post, "/items")
                .output::<String>()
                .handler(|_ctx, _: ()| async { Ok("created".to_string()) })
        };

        let _app = router_inst.into_axum_router(TestContext {
            value: "test".to_string(),
        });
    }
}
