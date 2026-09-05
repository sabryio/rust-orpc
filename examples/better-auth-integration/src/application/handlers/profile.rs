use crate::infrastructure::auth::guard::Authenticated;
use better_auth::prelude::AuthUser;
use orpc_core::OrpcError;
use serde_json::json;

/// Handler that receives Authenticated wrapper - session is guaranteed to exist.
///
/// SRP: Returns user profile data only.
/// DIP: Depends on Better-Auth's session abstraction via AuthenticatedUser newtype.
pub async fn get_profile(
    authenticated: Authenticated,
    _: (),
) -> Result<serde_json::Value, OrpcError> {
    // Use the helper method to access the session cleanly
    let session = authenticated
        .session
        .session()
        .ok_or_else(|| OrpcError::internal("Session unexpectedly missing after auth guard"))?;

    Ok(json!({
        "id": session.user.id(),
        "email": session.user.email(),
        "name": session.user.name(),
    }))
}
