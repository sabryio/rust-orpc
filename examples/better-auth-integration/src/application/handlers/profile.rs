use crate::infrastructure::auth::guard::AppContext;
use better_auth::prelude::AuthUser;
use orpc_core::OrpcError;
use serde_json::json;

pub async fn get_profile(ctx: AppContext, _: ()) -> Result<serde_json::Value, OrpcError> {
    // require_session() returns Err(401) if not authenticated
    let session = ctx.require_session()?;

    Ok(json!({
        "id": session.user.id(),
        "email": session.user.email(),
        "name": session.user.name(),
    }))
}
