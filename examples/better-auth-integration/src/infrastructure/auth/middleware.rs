use crate::domain::models::auth::AuthenticatedUser;
use crate::domain::ports::planet_repository::PlanetRepository;
use crate::infrastructure::auth::schema::AppAuthSchema;
use axum::middleware::Next;
use axum::response::Response;
use better_auth::{integrations::axum::OptionalSession, prelude::AuthUser};
use orpc_core::{Next as OrpcNext, OrpcError};
use std::sync::Arc;

/// Base context for public routes.
/// ISP: Contains only the dependencies required for public operations.
#[derive(Clone)]
pub struct BaseContext {
    pub planet_repo: Arc<dyn PlanetRepository>,
    pub current_user: Option<AuthenticatedUser>,
}

/// Segregated context for protected routes.
/// ISP & LSP: Guarantees `user` is present. Handlers receiving this context
/// do not need to handle `Option<AuthenticatedUser>`, preventing runtime unwrap panics.
#[derive(Clone)]
pub struct AuthContext {
    pub planet_repo: Arc<dyn PlanetRepository>,
    pub user: AuthenticatedUser,
}

/// Middleware that enforces authentication and upgrades BaseContext → AuthContext.
/// SRP: Sole responsibility is auth enforcement and context transformation.
pub async fn require_auth(
    ctx: BaseContext,
    next: OrpcNext<AuthContext>,
) -> Result<AuthContext, OrpcError> {
    let user = ctx
        .current_user
        .clone()
        .ok_or_else(|| OrpcError::unauthorized("Authentication required"))?;

    let auth_ctx = AuthContext {
        planet_repo: Arc::clone(&ctx.planet_repo),
        user,
    };

    next.run(auth_ctx).await
}

/// Axum middleware to extract the Better Auth session and store it in request extensions.
/// SRP: Only handles session extraction, not enforcement.
pub async fn session_layer(
    session: OptionalSession<AppAuthSchema>,
    mut req: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Response {
    if let Some(session_data) = session.0 {
        let user = AuthenticatedUser {
            id: session_data.user.id().to_string(),
            email: session_data.user.email().map(|e| e.to_string()),
            name: session_data.user.name().map(|n| n.to_string()),
        };
        req.extensions_mut().insert(user);
    }
    next.run(req).await
}

/// Per-request context builder: pulls the user from Axum extensions and sets it on BaseContext.
pub fn build_context(mut ctx: BaseContext, ext: &axum::http::Extensions) -> BaseContext {
    ctx.current_user = ext.get::<AuthenticatedUser>().cloned();
    ctx
}
