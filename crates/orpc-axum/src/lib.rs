//! # orpc-axum
//!
//! Axum integration for `orpc-core` — converts type-safe RPC procedure routers
//! into Axum routers with automatic routing and JSON serialization.
//!
//! ## Example
//!
//! ```rust,no_run
//! use orpc_core::{os, Procedure, ProcedureRegistry, Router};
//! use orpc_axum::AxumRouter;
//!
//! #[derive(Clone)]
//! struct AppContext {
//!     data: String,
//! }
//!
//! struct ApiRouter {
//!     ping: Procedure<AppContext, (), String>,
//! }
//!
//! impl Router<AppContext> for ApiRouter {
//!     fn register_procedures(&self, prefix: &str, registry: &mut ProcedureRegistry<AppContext>) {
//!         registry.insert("ping", &self.ping);
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let router = ApiRouter {
//!         ping: os()
//!             .context::<AppContext>()
//!             .output::<String>()
//!             .handler(|_ctx, _: ()| async { Ok("pong".to_string()) }),
//!     };
//!
//!     let app = router.into_axum_router(AppContext { data: "test".to_string() });
//!
//!     let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
//!     axum::serve(listener, app).await.unwrap();
//! }
//! ```

use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::post,
    Router as AxumRouterType,
};
use orpc_core::{OrpcError, OutputKind, ProcedureRegistry, Router};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

/// Extension trait to convert orpc routers into Axum routers.
pub trait AxumRouter<Ctx>: Router<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    /// Converts this orpc router into an Axum router with automatic routing.
    ///
    /// All procedures are registered at POST endpoints matching their path,
    /// with CORS enabled for all origins.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let axum_app = my_router.into_axum_router(context);
    /// ```
    fn into_axum_router(self, ctx: Ctx) -> AxumRouterType
    where
        Self: Sized,
    {
        let mut registry = ProcedureRegistry::new();
        self.register_procedures("", &mut registry);

        let registry = Arc::new(registry);
        let context_arc = Arc::new(ctx);
        let mut axum_router = AxumRouterType::new();

        // Register each procedure as a POST route with closure capturing path
        let paths: Vec<String> = registry.paths().cloned().collect();

        for path in paths {
            let route_path = format!("/{}", path);
            let path_for_handler = path.clone();
            let registry_clone = Arc::clone(&registry);
            let context_clone = Arc::clone(&context_arc);

            axum_router = axum_router.route(
                &route_path,
                post(move |Json(input): Json<serde_json::Value>| {
                    handle_procedure_closure(
                        Arc::clone(&registry_clone),
                        Arc::clone(&context_clone),
                        path_for_handler.clone(),
                        input,
                    )
                }),
            );
        }

        axum_router.layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
    }
}

// Implement AxumRouter for any type that implements Router
impl<T, Ctx> AxumRouter<Ctx> for T
where
    T: Router<Ctx>,
    Ctx: Clone + Send + Sync + 'static,
{
}

async fn handle_procedure_closure<Ctx>(
    registry: Arc<ProcedureRegistry<Ctx>>,
    context: Arc<Ctx>,
    path: String,
    input: serde_json::Value,
) -> Result<Json<serde_json::Value>, AxumError>
where
    Ctx: Clone + Send + Sync + 'static,
{
    let ctx = (*context).clone();
    let result = registry.call(&path, ctx, input).await?;

    match result {
        OutputKind::Single(value) => Ok(Json(value)),
        OutputKind::Stream(_) => Err(AxumError::Internal(
            "Streaming not yet supported via Axum integration".to_string(),
        )),
    }
}

/// Axum-specific error wrapper that implements IntoResponse
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

    #[derive(Clone)]
    struct TestContext {
        value: String,
    }

    #[test]
    fn test_axum_router_trait_available() {
        // Type check: verify AxumRouter trait is implemented for Router types
        struct SimpleRouter;

        impl Router<TestContext> for SimpleRouter {
            fn register_procedures(
                &self,
                _prefix: &str,
                _registry: &mut ProcedureRegistry<TestContext>,
            ) {
                // No-op for type check
            }
        }

        // This should compile if AxumRouter is properly implemented
        fn _takes_axum_router<T: AxumRouter<TestContext>>(_router: T) {}
        _takes_axum_router(SimpleRouter);
    }
}
