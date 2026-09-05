use crate::{
    domain::ports::planet_repository::PlanetRepository, infrastructure::auth::schema::AppAuthSchema,
};
use axum::extract::FromRef;
use better_auth::BetterAuth;
use std::sync::Arc;

/// Shared Axum state for the application.
///
/// Holds the planet repository and Better-Auth instance.
/// `FromRef` impls allow extractors to pull sub-state automatically —
/// `CurrentSession` / `OptionalSession` need `Arc<BetterAuth<Schema>>`.
#[derive(Clone)]
pub struct AppState {
    pub planet_repo: Arc<dyn PlanetRepository>,
    pub auth: Arc<BetterAuth<AppAuthSchema>>,
}

impl FromRef<AppState> for Arc<BetterAuth<AppAuthSchema>> {
    fn from_ref(state: &AppState) -> Self {
        state.auth.clone()
    }
}
