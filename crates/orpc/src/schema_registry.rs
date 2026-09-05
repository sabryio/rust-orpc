//! Zod schema registration — collects TypeScript schema strings at link time.
//!
//! The `#[orpc]` macro emits `inventory::submit! { SchemaRegistration { ... } }`
//! for each input/output type it sees. `generate_contract()` then collects all
//! registered schemas and embeds them in the generated TypeScript file.
//!
//! ## How it works without manual `#[derive(ZodTs)]`
//!
//! Users annotate their types with `#[derive(ZodTs)]` from `zod-rs-ts`.
//! The `#[orpc]` macro emits a `SchemaRegistration` that captures:
//! - The Rust type name (for deduplication)
//! - A factory `fn() -> String` that calls `T::zod_ts()` at runtime
//!
//! This means the schema string is generated lazily at `generate_contract()` time,
//! not at macro expansion time.

/// A registered Zod schema for a single Rust type.
///
/// Registered globally by the `#[orpc]` macro via `inventory::submit!`.
pub struct SchemaRegistration {
    /// Rust type name for deduplication (e.g. `"Planet"`)
    pub type_name: &'static str,
    /// Factory that returns the full TypeScript schema string for this type
    pub zod_ts: fn() -> String,
    /// Factory that returns names of nested custom types this type depends on
    pub dependent_types: fn() -> Vec<&'static str>,
}

inventory::collect!(SchemaRegistration);
