//! Route metadata — HTTP method + path carried by every procedure.
//!
//! This module is transport-agnostic: Axum reads it to register HTTP routes;
//! Tauri (future) reads the same struct to register IPC handlers.

use std::fmt;

/// HTTP method for a procedure's route.
///
/// Stored on every `Procedure` and read by transport adapters at startup.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl HttpMethod {
    /// Returns the canonical uppercase string for this method.
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
        }
    }
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Route metadata attached to every procedure.
///
/// Carries the HTTP method and absolute path declared via `.route()`.
/// Both Axum and Tauri adapters read this at startup to register handlers.
///
/// # Example
///
/// ```rust,ignore
/// os()
///     .context::<AppContext>()
///     .route(HttpMethod::Get, "/ping")
///     .output::<String>()
///     .handler(|_ctx, _: ()| async { Ok("pong".to_string()) })
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteMetadata {
    /// HTTP method to bind this procedure to
    pub method: HttpMethod,
    /// Absolute path (e.g. `/ping`, `/planet/{id}`)
    pub path: String,
}

impl RouteMetadata {
    /// Creates new route metadata.
    pub fn new(method: HttpMethod, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_method_as_str() {
        assert_eq!(HttpMethod::Get.as_str(), "GET");
        assert_eq!(HttpMethod::Post.as_str(), "POST");
        assert_eq!(HttpMethod::Put.as_str(), "PUT");
        assert_eq!(HttpMethod::Patch.as_str(), "PATCH");
        assert_eq!(HttpMethod::Delete.as_str(), "DELETE");
    }

    #[test]
    fn test_http_method_display() {
        assert_eq!(HttpMethod::Get.to_string(), "GET");
        assert_eq!(HttpMethod::Post.to_string(), "POST");
    }

    #[test]
    fn test_route_metadata_new() {
        let meta = RouteMetadata::new(HttpMethod::Get, "/ping");
        assert_eq!(meta.method, HttpMethod::Get);
        assert_eq!(meta.path, "/ping");
    }

    #[test]
    fn test_route_metadata_absolute_path() {
        let meta = RouteMetadata::new(HttpMethod::Post, "/planet/{id}");
        assert_eq!(meta.path, "/planet/{id}");
    }
}
