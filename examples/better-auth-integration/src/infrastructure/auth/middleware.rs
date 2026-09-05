use crate::domain::ports::planet_repository::PlanetRepository;
use crate::infrastructure::auth::schema::AppAuthSchema;
use better_auth::integrations::axum::OptionalSession;
use orpc_axum::better_auth::WithBetterAuth;
use std::sync::Arc;

/// Base context for all routes.
#[derive(Clone)]
pub struct BaseContext {
    pub planet_repo: Arc<dyn PlanetRepository>,
    pub session: Arc<OptionalSession<AppAuthSchema>>,
}

/// Implement WithBetterAuth so `.with_better_auth()` injects session automatically.
/// Associated type means no turbofish needed at call sites.
impl WithBetterAuth for BaseContext {
    type Schema = AppAuthSchema;

    fn inject_session(&mut self, session: Arc<OptionalSession<AppAuthSchema>>) {
        self.session = session;
    }
}
