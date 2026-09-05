//! Better Auth + orpc Integration Example
//!
//! Demonstrates plain Axum handlers with `#[orpc]` annotations alongside
//! Better Auth session management. No orpc-axum wrapper needed.

mod application;
mod domain;
mod infrastructure;
mod server;

use better_auth::{
    plugins::{EmailPasswordPlugin, SessionManagementPlugin},
    seaorm::SeaOrmStore,
    AuthConfig, BetterAuth,
};
use infrastructure::{
    auth::schema::AppAuthSchema,
    context::AppState,
    repositories::in_memory_planet_repo::{sample_planets, InMemoryPlanetRepository},
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Better Auth + orpc integration example...");

    // Generate TypeScript contract before starting the server
    #[cfg(debug_assertions)]
    {
        println!("📝 Generating TypeScript contract...");
        orpc::generate_contract()
            .output(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../client/src/rpc/bindings.ts"
            ))
            .expect("contract generation failed");
        println!("✅ Generated examples/axum-react/client/src/rpc/bindings.ts");
    }

    // 1. Database & auth setup
    let database = infrastructure::db::seaorm::connect("sqlite::memory:").await?;
    infrastructure::auth::schema::run_app_migrations(&database).await?;

    let secret = "your-very-secure-secret-key-at-least-32-chars-long-for-production-use-only";
    let config = AuthConfig::new(secret)
        .base_url("http://localhost:3000")
        .password_min_length(8);

    let store = SeaOrmStore::<AppAuthSchema>::new(config.clone(), database);

    let auth = Arc::new(
        BetterAuth::<AppAuthSchema>::new(config)
            .store(store)
            .plugin(EmailPasswordPlugin::new().enable_signup(true))
            .plugin(SessionManagementPlugin::new())
            .build()
            .await?,
    );

    // 2. Build shared state — repo + auth in one place
    let state = AppState {
        planet_repo: Arc::new(InMemoryPlanetRepository::new(sample_planets())),
        auth: auth.clone(),
    };

    // 3. Start server
    server::axum::run_server(state, auth).await
}
