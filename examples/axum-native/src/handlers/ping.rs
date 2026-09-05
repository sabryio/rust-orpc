use axum::Json;
use orpc::orpc;

/// Health check — no state, no input.
#[orpc(method = "GET", path = "/ping")]
pub async fn ping() -> Json<String> {
    Json("pong".to_string())
}
