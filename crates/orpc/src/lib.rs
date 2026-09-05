//! # orpc
//!
//! Unified facade for handler metadata collection, auto-router construction,
//! and TypeScript contract generation.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use axum::{extract::State, Json};
//! use orpc::orpc;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize)]
//! struct Planet { id: i32, name: String }
//!
//! // 1. Annotate plain Axum handlers — no orpc builder needed
//! #[orpc(method = "POST", path = "/planet/list")]
//! async fn list_planets(State(db): State<Db>) -> Json<Vec<Planet>> {
//!     Json(db.list().await)
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     // 2. Auto-build Axum router from annotated handlers
//!     let app = orpc::router().with_state(db);
//!
//!     // 3. Generate TypeScript contract
//!     orpc::generate_contract()
//!         .output("../client/src/rpc/index.ts")
//!         .unwrap();
//!
//!     axum::serve(listener, app).await.unwrap();
//! }
//! ```

pub mod metadata;
pub mod registration;
pub mod router;

pub use metadata::HandlerMetadata;
pub use registration::HandlerRegistration;
pub use router::router;

// Re-export inventory so users don't need to depend on it directly
pub use inventory;

// Re-export orpc-core and orpc-axum for convenience
pub use orpc_axum::AxumRouter;
pub use orpc_core::*;

// Re-export the #[orpc] macro
pub use orpc_macros::orpc;

use orpc_codegen::ContractBuilder;

/// Begin building a TypeScript contract from all discovered `#[orpc]` handlers.
///
/// # Example
///
/// ```rust,ignore
/// orpc::generate_contract()
///     .output("../client/src/rpc/index.ts")
///     .unwrap();
/// ```
pub fn generate_contract() -> ContractBuilder {
    let handlers: Vec<orpc_codegen::HandlerInfo> = inventory::iter::<HandlerMetadata>
        .into_iter()
        .map(|m| orpc_codegen::HandlerInfo {
            name: m.name,
            method: m.method,
            path: m.path,
            input_type_name: m.input_type_name,
            output_type_name: m.output_type_name,
            module_path: m.module_path,
        })
        .collect();
    ContractBuilder::new(handlers)
}
