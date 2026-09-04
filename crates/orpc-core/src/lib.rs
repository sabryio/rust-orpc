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
//! use orpc_core::{os, HttpMethod};
//!
//! #[derive(Clone)]
//! struct AppContext {
//!     data: String,
//! }
//!
//! let ping = os()
//!     .context::<AppContext>()
//!     .route(HttpMethod::Get, "/ping")
//!     .output::<String>()
//!     .handler(|_ctx: AppContext, _: ()| async move {
//!         Ok("pong".to_string())
//!     });
//! ```

mod builder;
mod error;
mod procedure;
mod registry;
mod route;
mod router;
mod router_builder;

// Re-export public API
pub use builder::{os, ProcedureBuilder, Routed, Unrouted};
pub use error::{IntoOrpcError, OrpcError};
pub use procedure::{OutputKind, Procedure, ProcedureHandler};
pub use registry::ProcedureRegistry;
pub use route::{HttpMethod, RouteMetadata};
pub use router::Router;

// router_builder is internal — used by router! macro only
#[doc(hidden)]
pub use router_builder::{r, RouterBuilder};

// Re-export procedural macro
pub use orpc_macros::router;
