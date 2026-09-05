//! Auto-router: builds an Axum `Router` from all `#[orpc]`-annotated handlers.

use axum::Router;
use std::any::Any;
use std::sync::Arc;

/// Auto-builds an Axum `Router` from all `#[orpc]`-annotated handlers.
///
/// Pass your Axum state — each handler's factory downcasts it to its
/// expected state type automatically.
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
pub fn router<S>(state: S) -> Router
where
    S: Clone + Send + Sync + 'static,
{
    let state: Arc<dyn Any + Send + Sync> = Arc::new(state);
    crate::registration::build_router(state)
}
