use crate::infrastructure::auth::middleware::BaseContext;
use crate::infrastructure::auth::schema::AppAuthSchema;
use orpc_axum::better_auth::BetterAuthContext;
use orpc_core::OrpcError;

/// Type alias for convenience — the context every handler receives.
pub type AppContext = BetterAuthContext<AppAuthSchema, BaseContext>;

/// Authentication guard: rejects unauthenticated requests.
/// Transforms AppContext → AppContext (same type, but session presence verified).
pub async fn require_auth(
    ctx: AppContext,
    next: orpc_core::Next<AppContext>,
) -> Result<AppContext, OrpcError> {
    // Use the built-in require_session check
    ctx.require_session()?;
    next.run(ctx).await
}
