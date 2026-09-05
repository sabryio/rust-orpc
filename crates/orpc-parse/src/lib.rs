//! AST parsing utilities and code generation for orpc proc macros.
//!
//! This crate is a regular (non-proc-macro) library so all parsing logic is
//! independently testable with normal `#[test]` functions.
//!
//! # Layers
//!
//! - [`errors`] — structured `Error` type with span information and compile-time suggestions
//! - [`types`] — AST-based wrapper extraction (`Json<T>`, `Result<T,E>`, etc.)
//! - [`attributes`] — `#[serde(...)]` and `#[zod(...)]` attribute parsing
//! - [`functions`] — handler function signature analysis → [`functions::HandlerSignature`]
//! - [`codegen`] — code generation; each sub-module corresponds to one proc macro
//!
//! # Dependency direction
//!
//! ```text
//! orpc-macros  (proc-macro bridge, lib.rs only)
//!      └── orpc-parse  (this crate — all implementation logic)
//!               └── syn, quote, proc-macro2, inventory
//! ```

pub mod attributes;
pub mod codegen;
pub mod errors;
pub mod functions;
pub mod types;

pub use errors::{Error, Result};
