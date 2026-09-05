use better_auth::prelude::AuthUser;
use orpc_axum::OptionalBetterAuthSession;
use orpc_core::{OrpcContext, OrpcError};

use crate::infrastructure::{auth::schema::AppAuthSchema, context::BaseContext};

pub async fn ping(
    OrpcContext(_ctx): OrpcContext<BaseContext>,
    OptionalBetterAuthSession(session): OptionalBetterAuthSession<AppAuthSchema>,
) -> Result<String, OrpcError> {
    match session.0.as_ref() {
        Some(current_session) => Ok(format!(
            "pong (authenticated as {})",
            current_session.user.email().unwrap_or("unknown")
        )),
        None => Ok("pong (anonymous)".to_string()),
    }
}
