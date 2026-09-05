//! # axum-native example
//!
//! Demonstrates `#[orpc]` on plain Axum handlers — no `router!` macro,
//! no `os()` builder. Just annotate handlers and let orpc discover them.
//!
//! ## What this shows
//!
//! ```rust,ignore
//! // Before (manual)
//! let app = Router::new()
//!     .route("/ping",          get(ping))
//!     .route("/planet/list",   post(list_planets))
//!     .route("/planet/find",   post(find_planet))
//!     .route("/planet/create", post(create_planet))
//!     .with_state(db);
//!
//! // After (auto-discovered)
//! let app = orpc::router().with_state(db);
//! ```
//!
//! And TypeScript contract is generated automatically:
//!
//! ```rust,ignore
//! orpc::generate_contract()
//!     .output("client/src/rpc/index.ts")
//!     .unwrap();
//! ```

mod errors;
mod handlers;
mod models;

use axum::Router;
use models::Db;

#[tokio::main]
async fn main() {
    // Generate TypeScript contract before starting the server
    #[cfg(debug_assertions)]
    {
        println!("Generating TypeScript contract...");
        orpc::generate_contract()
            .output("client/src/rpc/index.ts")
            .expect("contract generation failed");
        println!("✅ Generated client/src/rpc/index.ts");
    }

    let db = Db::new();

    // ✨ Auto-built Axum router — no manual .route() calls needed
    let router = orpc::router(db);

    // Compose with Axum's native .nest()
    let app = Router::new().nest("/api/v1", router);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3002")
        .await
        .expect("failed to bind");

    println!("🚀 axum-native example running on http://127.0.0.1:3002");
    println!("📡 Handlers discovered via #[orpc]:");

    for meta in orpc::inventory::iter::<orpc::HandlerMetadata>.into_iter() {
        println!(
            "   {} {} (from {})",
            meta.method, meta.path, meta.module_path
        );
    }

    axum::serve(listener, app).await.expect("server error");
}
