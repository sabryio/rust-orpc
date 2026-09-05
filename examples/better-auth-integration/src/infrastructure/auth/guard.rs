use crate::infrastructure::auth::middleware::BaseContext;
use crate::infrastructure::auth::schema::AppAuthSchema;
use better_auth::integrations::axum::OptionalSession;
use orpc_core::OrpcError;
use std::ops::Deref;
use std::sync::Arc;

/// Wrapper that combines BaseContext with guaranteed authentication.
/// Allows handlers to access both repository dependencies and session.
#[derive(Clone)]
pub struct Authenticated {
    pub ctx: BaseContext,
    pub session: Arc<OptionalSession<AppAuthSchema>>,
}

impl Deref for Authenticated {
    type Target = BaseContext;
    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

/// Authentication guard: transforms BaseContext → Authenticated.
/// Rejects unauthenticated requests with 401.
pub async fn require_auth(
    ctx: BaseContext,
    next: orpc_core::Next<Authenticated>,
) -> Result<Authenticated, OrpcError> {
    if ctx.session.0.is_none() {
        return Err(OrpcError::unauthorized("Authentication required"));
    }

    let authenticated = Authenticated {
        session: Arc::clone(&ctx.session),
        ctx,
    };

    next.run(authenticated).await
}
