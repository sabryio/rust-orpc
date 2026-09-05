//! OpenAPI metadata for procedures and routers.
//!
//! This module provides a TypeScript oRPC-compatible metadata system that allows
//! procedures to declare OpenAPI routing information through a fluent `.meta()` API.
//!
//! # Design Principles (SOLID)
//!
//! - **Single Responsibility**: OpenApiMeta handles metadata storage and merging only
//! - **Open/Closed**: New metadata fields can be added without modifying merge logic
//! - **Dependency Inversion**: Builder abstracts construction details from consumers

use crate::route::HttpMethod;

/// OpenAPI metadata for procedure routing.
///
/// Supports accumulation semantics: multiple `.meta()` calls merge with specific rules:
/// - **Prefixes**: concatenate in order (e.g., `/api` + `/v2` → `/api/v2`)
/// - **Method**: last value wins (override)
/// - **Path**: last value wins (override)
///
/// # Example
///
/// ```rust
/// use orpc_core::{openapi_builder, HttpMethod};
///
/// let meta = openapi_builder()
///     .prefix("/api")
///     .prefix("/v2")
///     .method(HttpMethod::Get)
///     .path("/planets")
///     .build();
///
/// assert_eq!(meta.prefixes, vec!["/api", "/v2"]);
/// assert_eq!(meta.method, Some(HttpMethod::Get));
/// assert_eq!(meta.path, Some("/planets".to_string()));
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OpenApiMeta {
    /// HTTP method (GET, POST, etc.). Last value wins when merging.
    pub method: Option<HttpMethod>,
    /// Path template (e.g., `/planets/{id}`). Last value wins when merging.
    pub path: Option<String>,
    /// Path prefixes accumulated from builder/router hierarchy. Concatenated when merging.
    pub prefixes: Vec<String>,
}

impl OpenApiMeta {
    /// Creates an empty OpenApiMeta.
    pub fn new() -> Self {
        Self::default()
    }

    /// Merges another OpenApiMeta into this one.
    ///
    /// **Merge rules:**
    /// - Prefixes: accumulate (concatenate)
    /// - Method: override (last one wins)
    /// - Path: override (last one wins)
    ///
    /// # Example
    ///
    /// ```rust
    /// use orpc_core::{openapi_builder, HttpMethod};
    ///
    /// let mut base = openapi_builder()
    ///     .prefix("/api")
    ///     .method(HttpMethod::Get)
    ///     .build();
    ///
    /// let override_meta = openapi_builder()
    ///     .prefix("/v2")
    ///     .method(HttpMethod::Post)
    ///     .path("/users")
    ///     .build();
    ///
    /// base.merge(override_meta);
    ///
    /// assert_eq!(base.prefixes, vec!["/api", "/v2"]);
    /// assert_eq!(base.method, Some(HttpMethod::Post)); // overridden
    /// assert_eq!(base.path, Some("/users".to_string()));
    /// ```
    pub fn merge(&mut self, other: OpenApiMeta) {
        // Prefixes: accumulate (concatenate in order)
        self.prefixes.extend(other.prefixes);

        // Method: last one wins (override)
        if other.method.is_some() {
            self.method = other.method;
        }

        // Path: last one wins (override)
        if other.path.is_some() {
            self.path = other.path;
        }
    }

    /// Returns true if this metadata provides a complete HTTP route.
    ///
    /// A complete route requires both method and path to be set.
    /// This is used for type-state transitions in `ProcedureBuilder`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use orpc_core::{openapi_builder, HttpMethod};
    ///
    /// let incomplete = openapi_builder()
    ///     .method(HttpMethod::Get)
    ///     .build();
    /// assert!(!incomplete.has_complete_route());
    ///
    /// let complete = openapi_builder()
    ///     .method(HttpMethod::Get)
    ///     .path("/planets")
    ///     .build();
    /// assert!(complete.has_complete_route());
    /// ```
    pub fn has_complete_route(&self) -> bool {
        self.method.is_some() && self.path.is_some()
    }

    /// Returns the full path with all prefixes concatenated.
    ///
    /// If no explicit path is set, returns `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use orpc_core::openapi_builder;
    ///
    /// let meta = openapi_builder()
    ///     .prefix("/api")
    ///     .prefix("/v2")
    ///     .path("/planets")
    ///     .build();
    ///
    /// assert_eq!(meta.full_path(), Some("/api/v2/planets".to_string()));
    /// ```
    pub fn full_path(&self) -> Option<String> {
        self.path.as_ref().map(|path| {
            if self.prefixes.is_empty() {
                path.clone()
            } else {
                format!("{}{}", self.prefixes.join(""), path)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_meta_default() {
        let meta = OpenApiMeta::default();
        assert_eq!(meta.method, None);
        assert_eq!(meta.path, None);
        assert!(meta.prefixes.is_empty());
    }

    #[test]
    fn test_merge_prefixes_accumulate() {
        let mut base = OpenApiMeta {
            method: None,
            path: None,
            prefixes: vec!["/api".to_string()],
        };

        let other = OpenApiMeta {
            method: None,
            path: None,
            prefixes: vec!["/v2".to_string(), "/users".to_string()],
        };

        base.merge(other);

        assert_eq!(
            base.prefixes,
            vec!["/api".to_string(), "/v2".to_string(), "/users".to_string()]
        );
    }

    #[test]
    fn test_merge_method_override() {
        let mut base = OpenApiMeta {
            method: Some(HttpMethod::Get),
            path: None,
            prefixes: vec![],
        };

        let other = OpenApiMeta {
            method: Some(HttpMethod::Post),
            path: None,
            prefixes: vec![],
        };

        base.merge(other);

        assert_eq!(base.method, Some(HttpMethod::Post));
    }

    #[test]
    fn test_merge_path_override() {
        let mut base = OpenApiMeta {
            method: None,
            path: Some("/old".to_string()),
            prefixes: vec![],
        };

        let other = OpenApiMeta {
            method: None,
            path: Some("/new".to_string()),
            prefixes: vec![],
        };

        base.merge(other);

        assert_eq!(base.path, Some("/new".to_string()));
    }

    #[test]
    fn test_merge_preserves_existing_when_other_is_none() {
        let mut base = OpenApiMeta {
            method: Some(HttpMethod::Get),
            path: Some("/path".to_string()),
            prefixes: vec!["/api".to_string()],
        };

        let other = OpenApiMeta {
            method: None,
            path: None,
            prefixes: vec![],
        };

        base.merge(other);

        assert_eq!(base.method, Some(HttpMethod::Get));
        assert_eq!(base.path, Some("/path".to_string()));
        assert_eq!(base.prefixes, vec!["/api".to_string()]);
    }

    #[test]
    fn test_has_complete_route_false_when_missing_method() {
        let meta = OpenApiMeta {
            method: None,
            path: Some("/path".to_string()),
            prefixes: vec![],
        };
        assert!(!meta.has_complete_route());
    }

    #[test]
    fn test_has_complete_route_false_when_missing_path() {
        let meta = OpenApiMeta {
            method: Some(HttpMethod::Get),
            path: None,
            prefixes: vec![],
        };
        assert!(!meta.has_complete_route());
    }

    #[test]
    fn test_has_complete_route_true_when_both_present() {
        let meta = OpenApiMeta {
            method: Some(HttpMethod::Post),
            path: Some("/users".to_string()),
            prefixes: vec![],
        };
        assert!(meta.has_complete_route());
    }

    #[test]
    fn test_full_path_without_prefixes() {
        let meta = OpenApiMeta {
            method: None,
            path: Some("/planets".to_string()),
            prefixes: vec![],
        };
        assert_eq!(meta.full_path(), Some("/planets".to_string()));
    }

    #[test]
    fn test_full_path_with_single_prefix() {
        let meta = OpenApiMeta {
            method: None,
            path: Some("/planets".to_string()),
            prefixes: vec!["/api".to_string()],
        };
        assert_eq!(meta.full_path(), Some("/api/planets".to_string()));
    }

    #[test]
    fn test_full_path_with_multiple_prefixes() {
        let meta = OpenApiMeta {
            method: None,
            path: Some("/planets".to_string()),
            prefixes: vec!["/api".to_string(), "/v2".to_string()],
        };
        assert_eq!(meta.full_path(), Some("/api/v2/planets".to_string()));
    }

    #[test]
    fn test_full_path_none_when_no_path() {
        let meta = OpenApiMeta {
            method: None,
            path: None,
            prefixes: vec!["/api".to_string()],
        };
        assert_eq!(meta.full_path(), None);
    }
}

/// Builder for constructing OpenApiMeta with a fluent API.
///
/// This builder provides a TypeScript-like ergonomic API for building metadata.
/// Used internally by the `openapi!` macro and available for manual construction.
///
/// # Example
///
/// ```rust
/// use orpc_core::{openapi_builder, HttpMethod};
///
/// let meta = openapi_builder()
///     .method(HttpMethod::Get)
///     .path("/planets/{id}")
///     .prefix("/api/v2")
///     .build();
///
/// assert!(meta.has_complete_route());
/// ```
#[derive(Debug, Clone, Default)]
pub struct OpenApiMetaBuilder {
    method: Option<HttpMethod>,
    path: Option<String>,
    prefixes: Vec<String>,
}

impl OpenApiMetaBuilder {
    /// Creates a new empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the HTTP method.
    ///
    /// # Example
    ///
    /// ```rust
    /// use orpc_core::{openapi_builder, HttpMethod};
    ///
    /// let meta = openapi_builder()
    ///     .method(HttpMethod::Post)
    ///     .build();
    ///
    /// assert_eq!(meta.method, Some(HttpMethod::Post));
    /// ```
    pub fn method(mut self, method: HttpMethod) -> Self {
        self.method = Some(method);
        self
    }

    /// Sets the path template.
    ///
    /// # Example
    ///
    /// ```rust
    /// use orpc_core::openapi_builder;
    ///
    /// let meta = openapi_builder()
    ///     .path("/planets/{id}")
    ///     .build();
    ///
    /// assert_eq!(meta.path, Some("/planets/{id}".to_string()));
    /// ```
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Adds a path prefix.
    ///
    /// Can be called multiple times to accumulate prefixes.
    ///
    /// # Example
    ///
    /// ```rust
    /// use orpc_core::openapi_builder;
    ///
    /// let meta = openapi_builder()
    ///     .prefix("/api")
    ///     .prefix("/v2")
    ///     .build();
    ///
    /// assert_eq!(meta.prefixes, vec!["/api", "/v2"]);
    /// ```
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefixes.push(prefix.into());
        self
    }

    /// Builds the final OpenApiMeta.
    ///
    /// # Example
    ///
    /// ```rust
    /// use orpc_core::{openapi_builder, HttpMethod};
    ///
    /// let meta = openapi_builder()
    ///     .method(HttpMethod::Get)
    ///     .path("/users")
    ///     .build();
    ///
    /// assert!(meta.has_complete_route());
    /// ```
    pub fn build(self) -> OpenApiMeta {
        OpenApiMeta {
            method: self.method,
            path: self.path,
            prefixes: self.prefixes,
        }
    }
}

/// Entry point for building OpenAPI metadata.
///
/// Returns a builder that can be used to construct OpenApiMeta with a fluent API.
///
/// # Example
///
/// ```rust
/// use orpc_core::{openapi_builder, HttpMethod};
///
/// let meta = openapi_builder()
///     .method(HttpMethod::Get)
///     .path("/planets")
///     .prefix("/api/v2")
///     .build();
/// ```
pub fn openapi_builder() -> OpenApiMetaBuilder {
    OpenApiMetaBuilder::new()
}

#[cfg(test)]
mod builder_tests {
    use super::*;

    #[test]
    fn test_builder_empty() {
        let meta = openapi_builder().build();
        assert_eq!(meta.method, None);
        assert_eq!(meta.path, None);
        assert!(meta.prefixes.is_empty());
    }

    #[test]
    fn test_builder_method_only() {
        let meta = openapi_builder().method(HttpMethod::Get).build();
        assert_eq!(meta.method, Some(HttpMethod::Get));
        assert_eq!(meta.path, None);
    }

    #[test]
    fn test_builder_path_only() {
        let meta = openapi_builder().path("/planets").build();
        assert_eq!(meta.path, Some("/planets".to_string()));
        assert_eq!(meta.method, None);
    }

    #[test]
    fn test_builder_prefix_only() {
        let meta = openapi_builder().prefix("/api").build();
        assert_eq!(meta.prefixes, vec!["/api".to_string()]);
        assert_eq!(meta.method, None);
        assert_eq!(meta.path, None);
    }

    #[test]
    fn test_builder_complete_route() {
        let meta = openapi_builder()
            .method(HttpMethod::Post)
            .path("/users")
            .build();
        assert!(meta.has_complete_route());
    }

    #[test]
    fn test_builder_multiple_prefixes() {
        let meta = openapi_builder()
            .prefix("/api")
            .prefix("/v2")
            .prefix("/admin")
            .build();
        assert_eq!(
            meta.prefixes,
            vec!["/api".to_string(), "/v2".to_string(), "/admin".to_string()]
        );
    }

    #[test]
    fn test_builder_all_fields() {
        let meta = openapi_builder()
            .method(HttpMethod::Put)
            .path("/planets/{id}")
            .prefix("/api")
            .prefix("/v2")
            .build();

        assert_eq!(meta.method, Some(HttpMethod::Put));
        assert_eq!(meta.path, Some("/planets/{id}".to_string()));
        assert_eq!(meta.prefixes, vec!["/api".to_string(), "/v2".to_string()]);
        assert!(meta.has_complete_route());
    }

    #[test]
    fn test_builder_into_string_path() {
        let path_string = String::from("/test");
        let meta = openapi_builder().path(path_string).build();
        assert_eq!(meta.path, Some("/test".to_string()));
    }

    #[test]
    fn test_builder_into_string_prefix() {
        let prefix_string = String::from("/api");
        let meta = openapi_builder().prefix(prefix_string).build();
        assert_eq!(meta.prefixes, vec!["/api".to_string()]);
    }

    #[test]
    fn test_builder_method_chaining_order() {
        // Test that method chaining works regardless of order
        let meta1 = openapi_builder()
            .path("/test")
            .method(HttpMethod::Get)
            .prefix("/api")
            .build();

        let meta2 = openapi_builder()
            .prefix("/api")
            .method(HttpMethod::Get)
            .path("/test")
            .build();

        assert_eq!(meta1.method, meta2.method);
        assert_eq!(meta1.path, meta2.path);
        assert_eq!(meta1.prefixes, meta2.prefixes);
    }
}
