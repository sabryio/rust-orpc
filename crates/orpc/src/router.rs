//! Auto-router: builds an Axum `Router` from all `#[orpc]`-annotated handlers.

use axum::Router;
use std::any::Any;
use std::sync::Arc;

/// Auto-builds an Axum `Router` from all `#[orpc]`-annotated handlers.
///
/// **Deprecated:** Use the `router!()` macro instead for better ergonomics
/// and module filtering capabilities.
///
/// Pass your Axum state — each handler's factory downcasts it to its
/// expected state type automatically.
///
/// # Migration
///
/// ```rust,ignore
/// // Old:
/// let app = orpc::router(db);
///
/// // New:
/// let app = router!(db);
/// ```
///
/// The `router!()` macro also supports module filtering:
///
/// ```rust,ignore
/// // Filter by module pattern
/// let app = router!("handlers::planet", db);
///
/// // Brace expansion for multiple modules
/// let app = router!("handlers::{planet,user}", db);
/// ```
///
/// # Example
///
/// ```rust,ignore
/// #[tokio::main]
/// async fn main() {
///     let db = Db::new();
///
///     // ✨ No manual .route() calls — all discovered from #[orpc] annotations
///     let app = orpc::router(db);
///
///     let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
///     axum::serve(listener, app).await.unwrap();
/// }
/// ```
#[deprecated(
    since = "0.2.0",
    note = "Use the `router!()` macro instead. Example: `router!(db)` or `router!(\"pattern\", db)`"
)]
pub fn router<S>(state: S) -> Router
where
    S: Clone + Send + Sync + 'static,
{
    let state: Arc<dyn Any + Send + Sync> = Arc::new(state);
    crate::registration::build_router(state)
}
