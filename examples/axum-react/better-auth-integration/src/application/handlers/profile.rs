use axum::{extract::State, Json};
use better_auth::prelude::AuthUser;
use serde_json::json;

use crate::infrastructure::{
    auth::extractors::{Session, SessionExt},
    context::AppState,
};

#[rorpc::get("/profile")]
pub async fn get_profile(
    State(_state): State<AppState>,
    session: Session,
) -> Json<serde_json::Value> {
    Json(json!({
        "id": session.user_id(),
        "email": session.user_email(),
        "name": session.user.name(),
    }))
}
