use crate::infrastructure::{auth::extractors::OptionalSession, context::BaseContext};
use better_auth::prelude::AuthUser;
use orpc_core::{OrpcContext, OrpcError};

pub async fn ping(
    OrpcContext(_ctx): OrpcContext<BaseContext>,
    session: OptionalSession,
) -> Result<String, OrpcError> {
    match session.0.as_ref().0.as_ref() {
        Some(current_session) => Ok(format!(
            "pong (authenticated as {})",
            current_session.user.email().unwrap_or("unknown")
        )),
        None => Ok("pong (anonymous)".to_string()),
    }
}
