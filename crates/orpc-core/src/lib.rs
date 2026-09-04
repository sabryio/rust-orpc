//! # orpc-core
//!
//! Type-safe RPC procedure abstractions for Rust.
//!
//! This crate provides the foundational abstractions for defining RPC procedures with
//! compile-time type safety. It remains framework-agnostic — no HTTP, no Axum, just
//! pure abstractions for building type-safe RPC handlers.
//!
//! ## Example
//!
//! ```rust
//! use orpc_core::{os, Procedure};
//!
//! #[derive(Clone)]
//! struct AppContext {
//!     data: String,
//! }
//!
//! // Define a procedure with typed input and output
//! let ping = os()
//!     .context::<AppContext>()
//!     .output::<String>()
//!     .handler(|_ctx: AppContext, _: ()| async move {
//!         Ok("pong".to_string())
//!     });
//! ```

mod builder;
mod error;
mod procedure;
mod registry;
mod router;

// Re-export public API
pub use builder::{os, ProcedureBuilder};
pub use error::{IntoOrpcError, OrpcError};
pub use procedure::{OutputKind, Procedure, ProcedureHandler};
pub use registry::ProcedureRegistry;
pub use router::Router;
