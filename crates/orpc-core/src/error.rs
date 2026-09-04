//! Error types for oRPC procedures.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Standard oRPC error with code, message, and optional HTTP status hint.
///
/// Provides common error constructors (NOT_FOUND, BAD_REQUEST, INTERNAL_ERROR)
/// and allows custom error codes via `custom()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrpcError {
    /// Error code (e.g., "NOT_FOUND", "BAD_REQUEST")
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Optional HTTP status code hint for framework integrations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
}

impl OrpcError {
    /// Creates a NOT_FOUND error (404)
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: "NOT_FOUND".to_string(),
            message: message.into(),
            status: Some(404),
        }
    }

    /// Creates a BAD_REQUEST error (400)
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            code: "BAD_REQUEST".to_string(),
            message: message.into(),
            status: Some(400),
        }
    }

    /// Creates an UNAUTHORIZED error (401)
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            code: "UNAUTHORIZED".to_string(),
            message: message.into(),
            status: Some(401),
        }
    }

    /// Creates an INTERNAL_ERROR (500)
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            code: "INTERNAL_ERROR".to_string(),
            message: message.into(),
            status: Some(500),
        }
    }

    /// Creates a custom error with user-defined code
    pub fn custom(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            status: None,
        }
    }
}

impl fmt::Display for OrpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for OrpcError {}

/// Trait for converting custom error types into OrpcError.
///
/// Implement this trait to integrate your domain-specific error types
/// with oRPC's error handling.
pub trait IntoOrpcError {
    fn into_orpc_error(self) -> OrpcError;
}

impl IntoOrpcError for OrpcError {
    fn into_orpc_error(self) -> OrpcError {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_found() {
        let err = OrpcError::not_found("Resource not found");
        assert_eq!(err.code, "NOT_FOUND");
        assert_eq!(err.message, "Resource not found");
        assert_eq!(err.status, Some(404));
    }

    #[test]
    fn test_bad_request() {
        let err = OrpcError::bad_request("Invalid input");
        assert_eq!(err.code, "BAD_REQUEST");
        assert_eq!(err.message, "Invalid input");
        assert_eq!(err.status, Some(400));
    }

    #[test]
    fn test_internal() {
        let err = OrpcError::internal("Database connection failed");
        assert_eq!(err.code, "INTERNAL_ERROR");
        assert_eq!(err.message, "Database connection failed");
        assert_eq!(err.status, Some(500));
    }

    #[test]
    fn test_custom() {
        let err = OrpcError::custom("RATE_LIMITED", "Too many requests");
        assert_eq!(err.code, "RATE_LIMITED");
        assert_eq!(err.message, "Too many requests");
        assert_eq!(err.status, None);
    }

    #[test]
    fn test_display() {
        let err = OrpcError::not_found("User not found");
        assert_eq!(err.to_string(), "NOT_FOUND: User not found");
    }

    #[test]
    fn test_into_orpc_error_trait() {
        let err = OrpcError::not_found("Test");
        let converted = err.into_orpc_error();
        assert_eq!(converted.code, "NOT_FOUND");
    }
}
