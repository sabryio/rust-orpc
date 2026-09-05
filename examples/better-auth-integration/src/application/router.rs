use crate::infrastructure::context::AppState;
use axum::Router;

// Import handler modules so #[orpc] submissions are linked into the binary
use super::handlers::{ping, planet, profile, stream};

/// Build the application router from all `#[orpc]`-annotated handlers.
pub fn build_router(state: AppState) -> Router {
    // Suppress unused import warnings — modules are needed to register
    // inventory::submit! entries even if we don't call functions directly.
    let _ = (
        ping::ping,
        planet::list_planets,
        profile::get_profile,
        stream::stream_events,
    );

    orpc::router!(state)
}
