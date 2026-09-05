use crate::domain::models::auth::AuthenticatedUser;
use crate::infrastructure::auth::middleware::BaseContext;
use orpc_core::OrpcError;
use std::ops::Deref;
use std::sync::Arc;

/// Wrapper that combines BaseContext with guaranteed authentication.
/// Allows handlers to access both repository dependencies and session.
///
/// DIP: Depends on Better-Auth's session abstraction via AuthenticatedUser newtype.
/// The presence of this type proves authentication succeeded at compile-time.
#[derive(Clone)]
pub struct Authenticated {
    pub ctx: BaseContext,
    /// Arc-wrapped AuthenticatedUser that's guaranteed to have Some(session)
    pub session: Arc<AuthenticatedUser>,
}

impl Deref for Authenticated {
    type Target = BaseContext;
    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

/// Authentication guard that transforms BaseContext → Authenticated.
///
/// Uses Better-Auth's native session types via our AuthenticatedUser newtype.
/// The Authenticated wrapper proves authentication occurred at compile-time.
///
/// SRP: Sole responsibility is auth enforcement via type transformation.
/// DIP: Depends on Better-Auth's session abstraction, not concrete session storage.
pub async fn require_auth(
    ctx: BaseContext,
    next: orpc_core::Next<Authenticated>,
) -> Result<Authenticated, OrpcError> {
    // Check if session exists using our helper method
    if !ctx.session.is_authenticated() {
        return Err(OrpcError::unauthorized("Authentication required"));
    }

    // Reuse the same Arc - the session is guaranteed to be Some at this point
    let authenticated = Authenticated {
        session: Arc::clone(&ctx.session),
        ctx,
    };

    next.run(authenticated).await
}
