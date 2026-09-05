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

pub mod error_registry;
pub mod metadata;
pub mod registration;
pub mod router;
pub mod schema_registry;

pub use error_registry::{ErrorRegistration, ErrorVariant};
pub use metadata::HandlerMetadata;
pub use registration::HandlerRegistration;
pub use schema_registry::SchemaRegistration;

// Re-export inventory so users don't need to depend on it directly
pub use inventory;

pub use orpc_macros::{OrpcErrors, ZodTs};

// Re-export orpc-core and orpc-axum for convenience
pub use orpc_axum::AxumRouter;
pub use orpc_core::*;

// Re-export the #[orpc] attribute macro and router! proc macro
pub use orpc_macros::orpc;
pub use orpc_macros::router;

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
            error_type_name: m.error_type_name,
        })
        .collect();

    // Build a lookup map of all registered schemas: type_name → registration.
    // When both a z.unknown() fallback (#[orpc]) and a real schema (#[derive(ZodTs)])
    // exist for the same type, prefer the real schema.
    let mut registry: std::collections::HashMap<&'static str, &SchemaRegistration> =
        std::collections::HashMap::new();

    for reg in inventory::iter::<SchemaRegistration>.into_iter() {
        let is_fallback = (reg.zod_ts)().contains("z.unknown()");
        match registry.get(reg.type_name) {
            // No entry yet — insert regardless
            None => {
                registry.insert(reg.type_name, reg);
            }
            // Existing entry is a fallback and this one is real — upgrade
            Some(existing) if (existing.zod_ts)().contains("z.unknown()") && !is_fallback => {
                registry.insert(reg.type_name, reg);
            }
            // Existing entry is real — keep it, skip this one
            _ => {}
        }
    }

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

    // Collect error registrations
    let errors: Vec<orpc_codegen::ErrorInfo> = inventory::iter::<ErrorRegistration>
        .into_iter()
        .map(|e| orpc_codegen::ErrorInfo {
            type_name: e.type_name,
            variants: e
                .variants
                .iter()
                .map(|v| orpc_codegen::ErrorVariantInfo {
                    name: v.name,
                    data_schema: v.data_schema,
                })
                .collect(),
        })
        .collect();

    ContractBuilder::new(handlers, ordered).with_errors(errors)
}
