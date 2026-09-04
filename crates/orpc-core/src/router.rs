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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{os, route::HttpMethod, Procedure};

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
        fn register_procedures(&self, _prefix: &str, registry: &mut ProcedureRegistry<Ctx>) {
            // Use route path as registry key
            registry.insert(self.ping.route.path.clone(), &self.ping);
        }
    }

    #[test]
    fn test_router_trait_compiles() {
        let router = PingRouter {
            ping: os()
                .context::<TestContext>()
                .route(HttpMethod::Get, "/ping")
                .output::<String>()
                .handler(|_ctx: TestContext, _: ()| async { Ok("pong".to_string()) }),
        };

        let mut registry = crate::ProcedureRegistry::<TestContext>::new();
        router.register_procedures("", &mut registry);

        assert!(registry.has("/ping"));
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
            registry.insert(self.ping.route.path.clone(), &self.ping);
            self.nested.register_procedures(prefix, registry);
        }
    }

    #[test]
    fn test_nested_router() {
        let router = NestedRouter {
            ping: os()
                .context::<TestContext>()
                .route(HttpMethod::Get, "/ping")
                .output::<String>()
                .handler(|_ctx, _: ()| async { Ok("root pong".to_string()) }),
            nested: PingRouter {
                ping: os()
                    .context::<TestContext>()
                    .route(HttpMethod::Get, "/nested/ping")
                    .output::<String>()
                    .handler(|_ctx, _: ()| async { Ok("nested pong".to_string()) }),
            },
        };

        let mut registry = crate::ProcedureRegistry::<TestContext>::new();
        router.register_procedures("", &mut registry);

        assert!(registry.has("/ping"));
        assert!(registry.has("/nested/ping"));
    }
}
