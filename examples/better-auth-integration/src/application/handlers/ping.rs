use crate::infrastructure::auth::guard::AppContext;
use better_auth::prelude::AuthUser;
use orpc_core::OrpcError;

pub async fn ping(ctx: AppContext, _: ()) -> Result<String, OrpcError> {
    match ctx.session() {
        Some(session) => Ok(format!(
            "pong (authenticated as {})",
            session.user.email().unwrap_or("unknown")
        )),
        None => Ok("pong (anonymous)".to_string()),
    }
}
