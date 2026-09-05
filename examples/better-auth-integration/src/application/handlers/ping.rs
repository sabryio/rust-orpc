use crate::infrastructure::auth::middleware::BaseContext;
use orpc_core::OrpcError;

pub async fn ping(ctx: BaseContext, _: ()) -> Result<String, OrpcError> {
    match &ctx.current_user {
        Some(u) => Ok(format!(
            "pong (authenticated as {})",
            u.email.as_deref().unwrap_or("unknown")
        )),
        None => Ok("pong (anonymous)".to_string()),
    }
}
