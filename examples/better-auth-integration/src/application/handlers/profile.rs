use crate::infrastructure::auth::middleware::AuthContext;
use orpc_core::OrpcError;
use serde_json::json;

pub async fn get_profile(ctx: AuthContext, _: ()) -> Result<serde_json::Value, OrpcError> {
    Ok(json!({
        "id": ctx.user.id,
        "email": ctx.user.email,
        "name": ctx.user.name,
    }))
}
