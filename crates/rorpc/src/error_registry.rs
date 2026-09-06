//! Error type registration for TypeScript contract generation.
//!
//! Error enums annotated with `#[derive(OrpcError)]` submit `ErrorRegistration`
//! entries via `inventory`. At contract generation time, these entries are matched
//! with handlers that return `Result<T, E>` to produce `.errors({...})` in the
//! TypeScript contract.

/// Error variant definition for contract generation.
///
/// Each variant becomes an entry in the `.errors({...})` object.
/// - Unit variants: `{ VARIANT_NAME: {} }`
/// - Data variants: `{ VARIANT_NAME: { data: <zod_schema> } }`
#[derive(Debug, Clone)]
pub struct ErrorVariant {
    /// Variant name in SCREAMING_SNAKE_CASE (e.g. "NOT_FOUND")
    pub name: &'static str,
    /// Optional Zod schema for variant data (None for unit variants)
    pub data_schema: Option<&'static str>,
}

/// Registration entry for an error enum type.
///
/// Submitted via `inventory::submit!` by the `#[derive(OrpcError)]` macro.
/// One registration per error type.
pub struct ErrorRegistration {
    /// Fully qualified type name (e.g. "AppError", "crate::errors::ApiError")
    pub type_name: &'static str,
    /// All variants of this error enum
    pub variants: &'static [ErrorVariant],
}

inventory::collect!(ErrorRegistration);
