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
    ///
    /// For nested routers, this method recursively calls `register_procedures`
    /// on child routers, building up the full path hierarchy.
    ///
    /// # Arguments
    ///
    /// * `prefix` - Path prefix for this router's procedures (e.g., "planet" for planet.find)
    /// * `registry` - Registry to populate with this router's procedures
    fn register_procedures(&self, prefix: &str, registry: &mut ProcedureRegistry<Ctx>);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{os, Procedure};

    #[derive(Clone)]
    struct TestContext {
        value: i32,
    }

    struct PingRouter<Ctx> {
        ping: Procedure<Ctx, (), String>,
    }

    impl<Ctx> Router<Ctx> for PingRouter<Ctx>
    where
        Ctx: Clone + Send + 'static,
    {
        fn register_procedures(&self, prefix: &str, registry: &mut ProcedureRegistry<Ctx>) {
            let path = if prefix.is_empty() {
                "ping".to_string()
            } else {
                format!("{}/ping", prefix)
            };
            registry.insert(path, &self.ping);
        }
    }

    #[test]
    fn test_router_trait_compiles() {
        let router = PingRouter {
            ping: os()
                .context::<TestContext>()
                .output::<String>()
                .handler(|_ctx: TestContext, _: ()| async { Ok("pong".to_string()) }),
        };

        let mut registry = ProcedureRegistry::<TestContext>::new();
        router.register_procedures("", &mut registry);

        assert!(registry.has("ping"));
    }

    struct NestedRouter<Ctx> {
        ping: Procedure<Ctx, (), String>,
        nested: PingRouter<Ctx>,
    }

    impl<Ctx> Router<Ctx> for NestedRouter<Ctx>
    where
        Ctx: Clone + Send + 'static,
    {
        fn register_procedures(&self, prefix: &str, registry: &mut ProcedureRegistry<Ctx>) {
            let ping_path = if prefix.is_empty() {
                "ping".to_string()
            } else {
                format!("{}/ping", prefix)
            };
            registry.insert(ping_path, &self.ping);

            let nested_prefix = if prefix.is_empty() {
                "nested".to_string()
            } else {
                format!("{}/nested", prefix)
            };
            self.nested.register_procedures(&nested_prefix, registry);
        }
    }

    #[test]
    fn test_nested_router() {
        let router = NestedRouter {
            ping: os()
                .context::<TestContext>()
                .output::<String>()
                .handler(|_ctx: TestContext, _: ()| async { Ok("root pong".to_string()) }),
            nested: PingRouter {
                ping: os()
                    .context::<TestContext>()
                    .output::<String>()
                    .handler(|_ctx: TestContext, _: ()| async { Ok("nested pong".to_string()) }),
            },
        };

        let mut registry = ProcedureRegistry::<TestContext>::new();
        router.register_procedures("", &mut registry);

        assert!(registry.has("ping"));
        assert!(registry.has("nested/ping"));
    }
}
