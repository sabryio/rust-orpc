use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use rorpc::OrpcError;
use serde_json::json;

#[derive(Debug, OrpcError)]
pub enum AppError {
    NotFound,
    Internal { msg: String },
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::NotFound => (
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                "Resource not found".into(),
            ),
            AppError::Internal { msg } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                msg.clone(),
            ),
        };
        (status, Json(json!({ "code": code, "message": message }))).into_response()
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::NotFound => write!(f, "Not found"),
            AppError::Internal{ msg } => write!(f, "Internal error: {msg}"),
        }
    }
}

impl std::error::Error for AppError {}
