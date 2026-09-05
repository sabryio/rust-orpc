//! Router trait for nested procedure registration.

use crate::ProcedureRegistry;

/// Trait for router types that can register their procedures into a registry.
///
/// Nested router structs implement this trait to flatten their hierarchy into
/// a flat HashMap at initialization time for O(1) runtime dispatch.
///
/// # OCP: New router types can be added by implementing this trait, without modifying the registry
/// # DIP: Registry depends on this abstraction, not concrete router implementations
pub trait Router<Ctx> {
    /// Registers all procedures in this router with the given prefix.
    fn register_procedures(&self, prefix: &str, registry: &mut ProcedureRegistry<Ctx>);
}
