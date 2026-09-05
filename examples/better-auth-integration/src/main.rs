//! Better Auth + orpc Integration Example
//!
//! Refactored to demonstrate Clean Architecture and SOLID principles.
//! Dependencies flow inward toward the domain layer.

mod application;
mod domain;
mod infrastructure;
mod server;

use better_auth::{
    integrations::axum::AxumIntegration,
    plugins::{EmailPasswordPlugin, SessionManagementPlugin},
    seaorm::SeaOrmStore,
    AuthConfig, BetterAuth,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Better Auth + orpc integration example...");

    // 1. Initialize Infrastructure (DB & Auth)
    let database_url = "sqlite::memory:";
    let database = infrastructure::db::seaorm::connect(database_url).await?;
    infrastructure::auth::schema::run_app_migrations(&database).await?;

    let secret = "your-very-secure-secret-key-at-least-32-chars-long-for-production-use-only";
    let config = AuthConfig::new(secret)
        .base_url("http://localhost:3000")
        .password_min_length(8);

    let store = SeaOrmStore::<infrastructure::auth::schema::AppAuthSchema>::new(
        config.clone(),
        database.clone(),
    );

    let auth = Arc::new(
        BetterAuth::<infrastructure::auth::schema::AppAuthSchema>::new(config)
            .store(store)
            .plugin(EmailPasswordPlugin::new().enable_signup(true))
            .plugin(SessionManagementPlugin::new())
            .build()
            .await?,
    );

    // 2. Build Routers
    let orpc_router = application::router::build_orpc_router();
    let auth_router = auth.clone().axum_router();

    // 3. Start Server (Composition of all layers)
    server::axum::run_server(orpc_router, auth_router, auth).await
}
