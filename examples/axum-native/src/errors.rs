//! Application error types for the axum-native example.
//!
//! Demonstrates `#[derive(OrpcErrors)]` for automatic TypeScript `.errors({})`
//! generation in the contract.

use orpc::OrpcErrors;

/// Application-level errors returned by handlers.
///
/// Each variant is converted to a TypeScript error entry:
/// - Unit variants become `VARIANT_NAME: {}`
/// - Struct variants become `VARIANT_NAME: { data: z.object({...}) }`
/// - Tuple variants become `VARIANT_NAME: { data: <schema> }`
#[derive(Debug, OrpcErrors)]
pub enum AppError {
    /// Resource not found (404-equivalent).
    NotFound,

    /// Resource already exists or conflicts with existing state (409-equivalent).
    Conflict { reason: String },

    /// Database operation failed (500-equivalent).
    DatabaseError(String),
}

// Implement std::fmt::Display for better error messages
impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::NotFound => write!(f, "Resource not found"),
            AppError::Conflict { reason } => write!(f, "Conflict: {}", reason),
            AppError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

// Implement std::error::Error for compatibility
impl std::error::Error for AppError {}

// Conversion from AppError to axum::http::StatusCode for HTTP responses
impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        use axum::Json;
        use serde_json::json;

        let (status, message) = match &self {
            AppError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::Conflict { .. } => (StatusCode::CONFLICT, self.to_string()),
            AppError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}
