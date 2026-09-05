use crate::infrastructure::{
    auth::{guard::AppContext, middleware::BaseContext, schema::AppAuthSchema},
    repositories::in_memory_planet_repo::{sample_planets, InMemoryPlanetRepository},
};
use axum::Router;
use better_auth::BetterAuth;
use orpc_axum::better_auth::{BetterAuthContext, BetterAuthExt};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

pub async fn run_server(
    orpc_router: impl orpc_axum::AxumRouter<AppContext>,
    auth_router: Router<Arc<BetterAuth<AppAuthSchema>>>,
    auth: Arc<BetterAuth<AppAuthSchema>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // User-defined context — only their own fields, no session boilerplate
    let inner_ctx = BaseContext {
        planet_repo: Arc::new(InMemoryPlanetRepository::new(sample_planets())),
    };

    // BetterAuthContext wraps it and manages the session automatically
    let base_ctx = BetterAuthContext::new(inner_ctx);

    // ✨ Dream API: no build_context, no session_layer, no session field in BaseContext
    let orpc_with_auth = orpc_router
        .into_axum_router_with_better_auth(base_ctx)
        .with_better_auth(auth.clone());

    let app = Router::new()
        .nest("/rpc", orpc_with_auth)
        .nest("/api/auth", auth_router.with_state(auth))
        .layer(
            CorsLayer::new()
                .allow_origin([
                    "http://localhost:5173"
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                    "http://localhost:3000"
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                ])
                .allow_methods([
                    axum::http::Method::GET,
                    axum::http::Method::POST,
                    axum::http::Method::PUT,
                    axum::http::Method::DELETE,
                    axum::http::Method::OPTIONS,
                ])
                .allow_headers([
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::AUTHORIZATION,
                    axum::http::header::ACCEPT,
                ])
                .allow_credentials(true),
        );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001").await?;

    println!("🚀 Server running on http://127.0.0.1:3001");
    println!("📚 Better Auth Endpoints (under /api/auth)");
    println!("🔧 orpc Endpoints (under /rpc)");

    axum::serve(listener, app)
        .with_graceful_shutdown(crate::server::shutdown::shutdown_signal())
        .await?;

    println!("\n✨ Server shutdown complete");
    Ok(())
}
