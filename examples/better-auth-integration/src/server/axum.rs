use crate::{
    domain::models::auth::AuthenticatedUser,
    infrastructure::{
        auth::{
            middleware::{build_context, session_layer, BaseContext},
            schema::AppAuthSchema,
        },
        repositories::in_memory_planet_repo::{sample_planets, InMemoryPlanetRepository},
    },
};
use axum::{middleware, Router};
use better_auth::BetterAuth;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

/// Assembles the final Axum application, applying CORS, session layers, and routing.
/// SRP: Sole responsibility is HTTP server configuration and composition.
pub async fn run_server(
    orpc_router: impl orpc_axum::AxumRouter<BaseContext>,
    auth_router: Router<Arc<BetterAuth<AppAuthSchema>>>,
    auth: Arc<BetterAuth<AppAuthSchema>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let base_ctx = BaseContext {
        planet_repo: Arc::new(InMemoryPlanetRepository::new(sample_planets())),
        session: Arc::new(AuthenticatedUser::new(
            better_auth::integrations::axum::OptionalSession(None),
        )),
    };

    let orpc_axum_router = orpc_router.into_axum_router_with(base_ctx, build_context);

    // Wrap the orpc router with the session extraction layer so that
    // every request has a chance to have its user populated.
    let orpc_with_session =
        orpc_axum_router.layer(middleware::from_fn_with_state(auth.clone(), session_layer));

    // Nest the auth router under /api/auth, then merge with main router
    // auth_router already has BetterAuth state, so we nest it into itself
    let auth_nested = Router::new()
        .nest("/api/auth", auth_router)
        .with_state(auth.clone());

    let app = Router::new()
        .nest("/rpc", orpc_with_session)
        .merge(auth_nested)
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
