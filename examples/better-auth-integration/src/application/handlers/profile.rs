use crate::infrastructure::{auth::schema::AppAuthSchema, context::BaseContext};
use better_auth::prelude::AuthUser;
use orpc_axum::BetterAuthSession;
use orpc_core::{OrpcContext, OrpcError};
use serde_json::json;

pub async fn get_profile(
    OrpcContext(ctx): OrpcContext<BaseContext>,
    BetterAuthSession(session): BetterAuthSession<AppAuthSchema>,
) -> Result<serde_json::Value, OrpcError> {
    // Session is guaranteed to exist (extractor returns 401 if not authenticated)
    let _ = ctx; // Context available but not needed in this handler

    // Since BetterAuthSession guarantees authentication, unwrap is safe
    let current_session = session
        .0
        .as_ref()
        .expect("BetterAuthSession guarantees session exists");

    Ok(json!({
        "id": current_session.user.id(),
        "email": current_session.user.email(),
        "name": current_session.user.name(),
    }))
}
