use crate::infrastructure::{
    auth::extractors::{Session, SessionExt},
    context::BaseContext,
};
use better_auth::prelude::AuthUser;
use orpc_core::{OrpcContext, OrpcError};
use serde_json::json;

pub async fn get_profile(
    OrpcContext(_ctx): OrpcContext<BaseContext>,
    session: Session,
) -> Result<serde_json::Value, OrpcError> {
    // Clean API: session.user_id(), session.user_email()
    Ok(json!({
        "id": session.user_id(),
        "email": session.user_email(),
        "name": session.current().user.name(),
    }))
}
