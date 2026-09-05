use crate::infrastructure::auth::middleware::BaseContext;
use better_auth::prelude::AuthUser;
use orpc_core::OrpcError;

pub async fn ping(ctx: BaseContext, _: ()) -> Result<String, OrpcError> {
    match ctx.session.0.as_ref() {
        Some(session) => Ok(format!(
            "pong (authenticated as {})",
            session.user.email().unwrap_or("unknown")
        )),
        None => Ok("pong (anonymous)".to_string()),
    }
}
