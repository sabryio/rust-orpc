//! Macro expansion tests for the router! macro.

use orpc_core::{os, router, HttpMethod, OutputKind, ProcedureRegistry, Router};

#[derive(Clone)]
struct TestContext {
    value: String,
}

#[test]
fn test_empty_router() {
    let _router = router! {};
}

#[test]
fn test_single_procedure() {
    let _router = router! {
        ping: os()
            .context::<TestContext>()
            .route(HttpMethod::Get, "/ping")
            .output::<String>()
            .handler(|_ctx, _: ()| async { Ok("pong".to_string()) })
    };
}

#[test]
fn test_multiple_procedures() {
    let _router = router! {
        ping: os()
            .context::<TestContext>()
            .route(HttpMethod::Get, "/ping")
            .output::<String>()
            .handler(|_ctx, _: ()| async { Ok("pong".to_string()) }),
        pong: os()
            .context::<TestContext>()
            .route(HttpMethod::Get, "/pong")
            .output::<String>()
            .handler(|_ctx, _: ()| async { Ok("ping".to_string()) })
    };
}

#[test]
fn test_nested_router() {
    #[derive(serde::Deserialize, serde::Serialize)]
    struct Planet {
        id: i32,
        name: String,
    }

    let _router = router! {
        planet: {
            list: os()
                .context::<TestContext>()
                .route(HttpMethod::Get, "/planet")
                .output::<Vec<Planet>>()
                .handler(|_ctx, _: ()| async { Ok(vec![]) })
        }
    };
}

#[test]
fn test_string_literal_keys() {
    let _router = router! {
        "list-paginated": os()
            .context::<TestContext>()
            .route(HttpMethod::Get, "/list-paginated")
            .output::<Vec<String>>()
            .handler(|_ctx, _: ()| async { Ok(vec![]) })
    };
}

#[test]
fn test_trailing_comma() {
    let _router = router! {
        ping: os()
            .context::<TestContext>()
            .route(HttpMethod::Get, "/ping")
            .output::<String>()
            .handler(|_ctx, _: ()| async { Ok("pong".to_string()) }),
    };
}

#[test]
fn test_no_trailing_comma() {
    let _router = router! {
        ping: os()
            .context::<TestContext>()
            .route(HttpMethod::Get, "/ping")
            .output::<String>()
            .handler(|_ctx, _: ()| async { Ok("pong".to_string()) })
    };
}

#[test]
fn test_deep_nesting() {
    let _router = router! {
        api: {
            v1: {
                users: {
                    list: os()
                        .context::<TestContext>()
                        .route(HttpMethod::Get, "/api/v1/users")
                        .output::<Vec<String>>()
                        .handler(|_ctx, _: ()| async { Ok(vec![]) })
                }
            }
        }
    };
}

#[test]
fn test_mixed_items() {
    #[derive(serde::Deserialize, serde::Serialize)]
    struct Planet {
        id: i32,
        name: String,
    }

    let _router = router! {
        ping: os()
            .context::<TestContext>()
            .route(HttpMethod::Get, "/ping")
            .output::<String>()
            .handler(|_ctx, _: ()| async { Ok("pong".to_string()) }),
        planet: {
            list: os()
                .context::<TestContext>()
                .route(HttpMethod::Get, "/planet")
                .output::<Vec<Planet>>()
                .handler(|_ctx, _: ()| async { Ok(vec![]) }),
            find: os()
                .context::<TestContext>()
                .route(HttpMethod::Get, "/planet/{id}")
                .input::<i32>()
                .output::<Planet>()
                .handler(|_ctx, id| async move {
                    Ok(Planet { id, name: "Earth".to_string() })
                })
        }
    };
}

#[tokio::test]
async fn test_router_registration() {
    let router_inst = router! {
        ping: os()
            .context::<TestContext>()
            .route(HttpMethod::Get, "/ping")
            .output::<String>()
            .handler(|_ctx, _: ()| async { Ok("pong".to_string()) })
    };

    let mut registry = ProcedureRegistry::new();
    router_inst.register_procedures("", &mut registry);

    assert!(registry.has("ping"));
}

#[tokio::test]
async fn test_nested_router_registration() {
    #[derive(serde::Deserialize, serde::Serialize)]
    struct Planet {
        id: i32,
        name: String,
    }

    let router_inst = router! {
        planet: {
            list: os()
                .context::<TestContext>()
                .route(HttpMethod::Get, "/planet")
                .output::<Vec<Planet>>()
                .handler(|_ctx, _: ()| async { Ok(vec![]) })
        }
    };

    let mut registry = ProcedureRegistry::new();
    router_inst.register_procedures("", &mut registry);

    assert!(registry.has("planet/list"));
}

#[tokio::test]
async fn test_end_to_end_dispatch() {
    let router_inst = router! {
        ping: os()
            .context::<TestContext>()
            .route(HttpMethod::Get, "/ping")
            .output::<String>()
            .handler(|ctx, _: ()| async move { Ok(ctx.value.clone()) })
    };

    let mut registry = ProcedureRegistry::new();
    router_inst.register_procedures("", &mut registry);

    let ctx = TestContext {
        value: "test-value".to_string(),
    };
    let result = registry.call("ping", ctx, serde_json::json!(null)).await;
    assert!(result.is_ok());

    match result.unwrap() {
        OutputKind::Single(val) => assert_eq!(val.as_str().unwrap(), "test-value"),
        _ => panic!("Expected Single output"),
    }
}

#[tokio::test]
async fn test_macro_equivalence() {
    let router_a = router! {
        ping: os()
            .context::<TestContext>()
            .route(HttpMethod::Get, "/ping")
            .output::<String>()
            .handler(|ctx, _: ()| async move { Ok(ctx.value.clone()) })
    };

    let router_b = router! {
        ping: os()
            .context::<TestContext>()
            .route(HttpMethod::Get, "/ping")
            .output::<String>()
            .handler(|ctx, _: ()| async move { Ok(ctx.value.clone()) })
    };

    let mut manual_registry = ProcedureRegistry::new();
    router_a.register_procedures("", &mut manual_registry);

    let mut macro_registry = ProcedureRegistry::new();
    router_b.register_procedures("", &mut macro_registry);

    assert_eq!(manual_registry.len(), macro_registry.len());
    assert!(manual_registry.has("ping"));
    assert!(macro_registry.has("ping"));
}
