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
pub mod schema_registry;

pub use metadata::HandlerMetadata;
pub use registration::HandlerRegistration;
pub use router::router;
pub use schema_registry::SchemaRegistration;

// Re-export inventory so users don't need to depend on it directly
pub use inventory;

pub use orpc_macros::ZodTs;

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

    // Build a lookup map of all registered schemas: type_name → registration
    let registry: std::collections::HashMap<&'static str, &SchemaRegistration> =
        inventory::iter::<SchemaRegistration>
            .into_iter()
            .map(|s| (s.type_name, s))
            .collect();

    // Topological sort: emit dependency schemas before the types that use them
    let mut ordered: Vec<orpc_codegen::SchemaEntry> = Vec::new();
    let mut visited: std::collections::HashSet<&'static str> = std::collections::HashSet::new();

    fn visit(
        type_name: &'static str,
        registry: &std::collections::HashMap<&'static str, &SchemaRegistration>,
        visited: &mut std::collections::HashSet<&'static str>,
        ordered: &mut Vec<orpc_codegen::SchemaEntry>,
    ) {
        if visited.contains(type_name) {
            return;
        }
        visited.insert(type_name);

        if let Some(reg) = registry.get(type_name) {
            // Recurse into dependencies first
            for dep in (reg.dependent_types)() {
                visit(dep, registry, visited, ordered);
            }
            ordered.push(orpc_codegen::SchemaEntry {
                type_name: reg.type_name,
                zod_ts: (reg.zod_ts)(),
            });
        }
    }

    // Start with directly registered types (from handler signatures)
    for type_name in registry.keys().copied() {
        visit(type_name, &registry, &mut visited, &mut ordered);
    }

    ContractBuilder::new(handlers, ordered)
}
