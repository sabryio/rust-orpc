use crate::infrastructure::auth::middleware::BaseContext;
use better_auth::prelude::AuthUser;
use orpc_core::OrpcError;

/// Ping handler with optional authentication.
/// Shows session status if authenticated, otherwise returns anonymous response.
pub async fn ping(ctx: BaseContext, _: ()) -> Result<String, OrpcError> {
    // Use the helper method to access the session cleanly
    match ctx.session.session() {
        Some(session) => Ok(format!(
            "pong (authenticated as {})",
            session.user.email().unwrap_or("unknown")
        )),
        None => Ok("pong (anonymous)".to_string()),
    }
}
