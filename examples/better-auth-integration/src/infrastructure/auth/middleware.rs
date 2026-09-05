use crate::domain::models::auth::AuthenticatedUser;
use crate::domain::ports::planet_repository::PlanetRepository;
use crate::infrastructure::auth::schema::AppAuthSchema;
use axum::middleware::Next;
use axum::response::Response;
use better_auth::integrations::axum::OptionalSession;
use std::sync::Arc;

/// Base context for all routes.
/// SRP: Contains shared dependencies and Better-Auth's session wrapped in our newtype + Arc.
#[derive(Clone)]
pub struct BaseContext {
    pub planet_repo: Arc<dyn PlanetRepository>,
    /// Better-Auth's session wrapped in AuthenticatedUser newtype and Arc for cloneability
    pub session: Arc<AuthenticatedUser>,
}

/// Axum middleware to extract Better-Auth session and store it in request extensions.
/// SRP: Only handles session extraction, not enforcement.
pub async fn session_layer(
    session: OptionalSession<AppAuthSchema>,
    mut req: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Response {
    req.extensions_mut()
        .insert(Arc::new(AuthenticatedUser::new(session)));
    next.run(req).await
}

/// Per-request context builder: extracts AuthenticatedUser from Axum extensions.
/// DIP: Depends on Better-Auth's OptionalSession abstraction.
pub fn build_context(mut ctx: BaseContext, ext: &axum::http::Extensions) -> BaseContext {
    ctx.session = ext
        .get::<Arc<AuthenticatedUser>>()
        .cloned()
        .unwrap_or_else(|| Arc::new(AuthenticatedUser::new(OptionalSession(None))));
    ctx
}
