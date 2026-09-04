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
mod streaming;

// Re-export public API
pub use builder::{os, ProcedureBuilder, Routed, Unrouted};
pub use error::{IntoOrpcError, OrpcError};
pub use procedure::{OutputKind, Procedure, ProcedureHandler};
pub use registry::ProcedureRegistry;
pub use route::{HttpMethod, RouteMetadata};
pub use router::Router;
pub use streaming::{AsyncIterator, StreamingProcedure};

/// Type alias for streaming output types.
///
/// Use this with `.output::<Stream<T>>()` to declare a streaming procedure.
///
/// # Example
///
/// ```rust
/// use orpc_core::{os, Stream, HttpMethod};
///
/// #[derive(Clone)]
/// struct Ctx;
///
/// #[derive(serde::Serialize)]
/// struct Event { count: u32 }
///
/// let proc = os()
///     .context::<Ctx>()
///     .route(HttpMethod::Post, "/stream")
///     .output::<Stream<Event>>()
///     .handler(|_ctx, _: ()| async {
///         let stream = tokio_stream::iter(0u32..10)
///             .map(|count| Event { count });
///         Ok(stream)
///     });
/// ```
pub type Stream<T> = AsyncIterator<T>;

// router_builder is internal — used by router! macro only
#[doc(hidden)]
pub use router_builder::{r, RouterBuilder};

// Re-export procedural macro
pub use orpc_macros::router;
