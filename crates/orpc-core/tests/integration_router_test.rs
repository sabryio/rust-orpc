//! Integration test: Nested router struct flattening

use orpc_core::{os, Procedure, ProcedureRegistry, Router};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
struct AppContext {
    prefix: String,
}

#[derive(Deserialize, Serialize)]
struct FindInput {
    id: i32,
}

// Define nested router structure matching TypeScript contract shape
struct PlanetRouter {
    list: Procedure<AppContext, (), Vec<String>>,
    find: Procedure<AppContext, FindInput, String>,
}

impl Router<AppContext> for PlanetRouter {
    fn register_procedures(&self, prefix: &str, registry: &mut ProcedureRegistry<AppContext>) {
        let list_path = if prefix.is_empty() {
            "list".to_string()
        } else {
            format!("{}/list", prefix)
        };
        registry.insert(list_path, &self.list);

        let find_path = if prefix.is_empty() {
            "find".to_string()
        } else {
            format!("{}/find", prefix)
        };
        registry.insert(find_path, &self.find);
    }
}

struct ApiRouter {
    ping: Procedure<AppContext, (), String>,
    planet: PlanetRouter,
}

impl Router<AppContext> for ApiRouter {
    fn register_procedures(&self, prefix: &str, registry: &mut ProcedureRegistry<AppContext>) {
        let ping_path = if prefix.is_empty() {
            "ping".to_string()
        } else {
            format!("{}/ping", prefix)
        };
        registry.insert(ping_path, &self.ping);

        let planet_prefix = if prefix.is_empty() {
            "planet".to_string()
        } else {
            format!("{}/planet", prefix)
        };
        self.planet.register_procedures(&planet_prefix, registry);
    }
}

#[tokio::test]
async fn test_nested_router_flattening() {
    // Create nested router structure
    let router = ApiRouter {
        ping: os()
            .context::<AppContext>()
            .output::<String>()
            .handler(|ctx: AppContext, _: ()| async move {
                Ok(format!("{} pong", ctx.prefix))
            }),
        planet: PlanetRouter {
            list: os()
                .context::<AppContext>()
                .output::<Vec<String>>()
                .handler(|_ctx: AppContext, _: ()| async move {
                    Ok(vec!["Earth".to_string(), "Mars".to_string()])
                }),
            find: os()
                .context::<AppContext>()
                .input::<FindInput>()
                .output::<String>()
                .handler(|_ctx: AppContext, input: FindInput| async move {
                    Ok(format!("Planet {}", input.id))
                }),
        },
    };

    // Flatten into registry
    let mut registry = ProcedureRegistry::<AppContext>::new();
    router.register_procedures("", &mut registry);

    // Verify all procedures are registered
    assert!(registry.has("ping"));
    assert!(registry.has("planet/list"));
    assert!(registry.has("planet/find"));
    assert_eq!(registry.len(), 3);

    // Verify procedures can be called
    let ctx = AppContext {
        prefix: "test".to_string(),
    };

    let ping_result = registry
        .call("ping", ctx.clone(), serde_json::Value::Null)
        .await;
    assert!(ping_result.is_ok());

    let list_result = registry
        .call("planet/list", ctx.clone(), serde_json::Value::Null)
        .await;
    assert!(list_result.is_ok());

    let find_result = registry
        .call(
            "planet/find",
            ctx,
            serde_json::json!({ "id": 1 }),
        )
        .await;
    assert!(find_result.is_ok());
}

#[tokio::test]
async fn test_router_with_prefix() {
    let router = PlanetRouter {
        list: os()
            .context::<AppContext>()
            .output::<Vec<String>>()
            .handler(|_ctx: AppContext, _: ()| async move {
                Ok(vec!["Venus".to_string()])
            }),
        find: os()
            .context::<AppContext>()
            .input::<FindInput>()
            .output::<String>()
            .handler(|_ctx: AppContext, input: FindInput| async move {
                Ok(format!("Found {}", input.id))
            }),
    };

    let mut registry = ProcedureRegistry::<AppContext>::new();
    router.register_procedures("api/v1/planet", &mut registry);

    assert!(registry.has("api/v1/planet/list"));
    assert!(registry.has("api/v1/planet/find"));
    assert_eq!(registry.len(), 2);
}
