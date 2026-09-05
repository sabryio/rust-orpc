use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use orpc::OrpcErrors;
use serde_json::json;

#[derive(Debug, OrpcErrors)]
pub enum AppError {
    NotFound,
    BadRequest { reason: String },
    Internal(String),
    Unauthorized,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::NotFound         => (StatusCode::NOT_FOUND, "NOT_FOUND", "Resource not found".into()),
            AppError::BadRequest { reason } => (StatusCode::BAD_REQUEST, "BAD_REQUEST", reason.clone()),
            AppError::Internal(msg)    => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", msg.clone()),
            AppError::Unauthorized     => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", "Authentication required".into()),
        };
        (status, Json(json!({ "code": code, "message": message }))).into_response()
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::NotFound             => write!(f, "Not found"),
            AppError::BadRequest { reason} => write!(f, "Bad request: {reason}"),
            AppError::Internal(msg)        => write!(f, "Internal error: {msg}"),
            AppError::Unauthorized         => write!(f, "Unauthorized"),
        }
    }
}

impl std::error::Error for AppError {}
