//! Code generation layer — converts parsed AST into `proc_macro2::TokenStream`.
//!
//! Each sub-module corresponds to one proc macro in `orpc-macros`.
//! `orpc-macros/src/lib.rs` calls these functions directly; the only logic
//! that lives there is the thin `proc_macro::TokenStream` ↔ `proc_macro2::TokenStream`
//! conversion.

pub mod error_derive;
pub mod orpc;
pub mod router;
pub mod zod_ts;

// Re-export the types that orpc-macros needs at the call site
pub use error_derive::expand_orpc_errors;
pub use orpc::{expand_orpc, OrpcArgs};
pub use router::{expand_router, RouterArgs};
pub use zod_ts::derive_zod_ts;
