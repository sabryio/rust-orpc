use crate::infrastructure::auth::guard::Authenticated;
use better_auth::prelude::AuthUser;
use orpc_core::OrpcError;
use serde_json::json;

pub async fn get_profile(
    authenticated: Authenticated,
    _: (),
) -> Result<serde_json::Value, OrpcError> {
    let session = authenticated
        .session
        .0
        .as_ref()
        .ok_or_else(|| OrpcError::internal("Session unexpectedly missing after auth guard"))?;

    Ok(json!({
        "id": session.user.id(),
        "email": session.user.email(),
        "name": session.user.name(),
    }))
}
