use crate::infrastructure::{auth::schema::AppAuthSchema, context::AppState};
use axum::Router;
use better_auth::{integrations::axum::AxumIntegration, BetterAuth};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

pub async fn run_server(
    state: AppState,
    auth: Arc<BetterAuth<AppAuthSchema>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Resolve auth router's state before nesting — matches axum-react pattern
    let auth_router = auth.clone().axum_router().with_state(auth);

    // Build the orpc handler router and nest alongside auth
    let app = Router::new()
        .nest(
            "/rpc",
            crate::application::router::build_router(state.clone()),
        )
        .nest("/api/auth", auth_router)
        .with_state(state)
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
