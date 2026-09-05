//! Auto-router: builds an Axum `Router` from all `#[orpc]`-annotated handlers.
//!
//! `orpc::router()` iterates over all `HandlerMetadata` entries registered via
//! `inventory::submit!` and registers the appropriate Axum route for each one.

// NOTE (T012): Full implementation requires storing a type-erased handler function
// pointer alongside the metadata. This is the structural placeholder — the macro
// (T011) will emit both the metadata and a registration callback.

use crate::metadata::HandlerMetadata;
use axum::Router;

/// Auto-builds an Axum `Router` from all `#[orpc]`-annotated handlers.
///
/// # Example
///
/// ```rust,ignore
/// #[tokio::main]
/// async fn main() {
///     let app = orpc::router()
///         .with_state(db);
///
///     let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
///     axum::serve(listener, app).await.unwrap();
/// }
/// ```
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let app = Router::new();

    for meta in inventory::iter::<HandlerMetadata> {
        // TODO (T012): retrieve and register actual handler function pointer.
        // The macro will emit a companion HandlerRegistration entry that pairs
        // each HandlerMetadata with its type-erased Axum handler.
        let _ = meta;
    }

    app
}
