use axum::{extract::State, Json};
use better_auth::prelude::AuthUser;
use rorpc::orpc;

use crate::infrastructure::{auth::extractors::OptionalSession, context::AppState};

#[orpc(method = "POST", path = "/ping")]
pub async fn ping(State(_state): State<AppState>, session: OptionalSession) -> Json<String> {
    let msg = match session.0 {
        Some(s) => format!(
            "pong (authenticated as {})",
            s.user.email().unwrap_or("unknown")
        ),
        None => "pong (anonymous)".to_string(),
    };
    Json(msg)
}
