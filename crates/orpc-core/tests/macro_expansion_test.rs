//! Macro expansion tests for the router! macro.
//!
//! These tests verify that the macro expands correctly for various input patterns.

use orpc_core::{os, router, ProcedureRegistry, Router};

#[derive(Clone)]
struct TestContext {
    value: String,
}

// Test 1: Empty router
#[test]
fn test_empty_router() {
    let _router = router! {};
    // Should compile without errors
}

// Test 2: Single procedure
#[test]
fn test_single_procedure() {
    let _router = router! {
        ping: os()
            .context::<TestContext>()
            .output::<String>()
            .handler(|_ctx, _: ()| async { Ok("pong".to_string()) })
    };
    // Should compile without errors
}

// Test 3: Multiple procedures
#[test]
fn test_multiple_procedures() {
    let _router = router! {
        ping: os()
            .context::<TestContext>()
            .output::<String>()
            .handler(|_ctx, _: ()| async { Ok("pong".to_string()) }),
        pong: os()
            .context::<TestContext>()
            .output::<String>()
            .handler(|_ctx, _: ()| async { Ok("ping".to_string()) })
    };
    // Should compile without errors
}

// Test 4: Nested router
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
                .output::<Vec<Planet>>()
                .handler(|_ctx, _: ()| async { Ok(vec![]) })
        }
    };
    // Should compile without errors
}

// Test 5: String literal keys
#[test]
fn test_string_literal_keys() {
    let _router = router! {
        "list-paginated": os()
            .context::<TestContext>()
            .output::<Vec<String>>()
            .handler(|_ctx, _: ()| async { Ok(vec![]) })
    };
    // Should compile without errors
}

// Test 6: Trailing comma
#[test]
fn test_trailing_comma() {
    let _router = router! {
        ping: os()
            .context::<TestContext>()
            .output::<String>()
            .handler(|_ctx, _: ()| async { Ok("pong".to_string()) }),
    };
    // Should compile without errors
}

// Test 7: No trailing comma
#[test]
fn test_no_trailing_comma() {
    let _router = router! {
        ping: os()
            .context::<TestContext>()
            .output::<String>()
            .handler(|_ctx, _: ()| async { Ok("pong".to_string()) })
    };
    // Should compile without errors
}

// Test 8: Deep nesting
#[test]
fn test_deep_nesting() {
    let _router = router! {
        api: {
            v1: {
                users: {
                    list: os()
                        .context::<TestContext>()
                        .output::<Vec<String>>()
                        .handler(|_ctx, _: ()| async { Ok(vec![]) })
                }
            }
        }
    };
    // Should compile without errors
}

// Test 9: Mixed items
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
            .output::<String>()
            .handler(|_ctx, _: ()| async { Ok("pong".to_string()) }),
        planet: {
            list: os()
                .context::<TestContext>()
                .output::<Vec<Planet>>()
                .handler(|_ctx, _: ()| async { Ok(vec![]) }),
            find: os()
                .context::<TestContext>()
                .input::<i32>()
                .output::<Planet>()
                .handler(|_ctx, id| async move {
                    Ok(Planet { id, name: "Earth".to_string() })
                })
        }
    };
    // Should compile without errors
}

// Test 10: Router can be registered
#[tokio::test]
async fn test_router_registration() {
    let router = router! {
        ping: os()
            .context::<TestContext>()
            .output::<String>()
            .handler(|_ctx, _: ()| async { Ok("pong".to_string()) })
    };

    let mut registry = ProcedureRegistry::new();
    router.register_procedures("", &mut registry);

    assert!(registry.has("ping"));
}

// Test 11: Nested router registration
#[tokio::test]
async fn test_nested_router_registration() {
    #[derive(serde::Deserialize, serde::Serialize)]
    struct Planet {
        id: i32,
        name: String,
    }

    let router = router! {
        planet: {
            list: os()
                .context::<TestContext>()
                .output::<Vec<Planet>>()
                .handler(|_ctx, _: ()| async { Ok(vec![]) })
        }
    };

    let mut registry = ProcedureRegistry::new();
    router.register_procedures("", &mut registry);

    assert!(registry.has("planet/list"));
}

// Test 12: End-to-end dispatch
#[tokio::test]
async fn test_end_to_end_dispatch() {
    let router = router! {
        ping: os()
            .context::<TestContext>()
            .output::<String>()
            .handler(|ctx, _: ()| async move { Ok(ctx.value.clone()) })
    };

    let mut registry = ProcedureRegistry::new();
    router.register_procedures("", &mut registry);

    let ctx = TestContext {
        value: "test-value".to_string(),
    };

    let result = registry.call("ping", ctx, serde_json::json!(null)).await;
    assert!(result.is_ok());

    match result.unwrap() {
        orpc_core::OutputKind::Single(val) => {
            assert_eq!(val.as_str().unwrap(), "test-value");
        }
        _ => panic!("Expected Single output"),
    }
}

// Test 13: Macro generates equivalent code to manual builder
#[tokio::test]
async fn test_macro_equivalence() {
    use orpc_core::r as r_fn;

    // Manual builder approach
    let manual_router = r_fn().add(
        "ping",
        os().context::<TestContext>()
            .output::<String>()
            .handler(|ctx, _: ()| async move { Ok(ctx.value.clone()) }),
    );

    // Macro approach
    let macro_router = router! {
        ping: os()
            .context::<TestContext>()
            .output::<String>()
            .handler(|ctx, _: ()| async move { Ok(ctx.value.clone()) })
    };

    // Both should register the same procedures
    let mut manual_registry = ProcedureRegistry::new();
    manual_router.register_procedures("", &mut manual_registry);

    let mut macro_registry = ProcedureRegistry::new();
    macro_router.register_procedures("", &mut macro_registry);

    assert_eq!(manual_registry.len(), macro_registry.len());
    assert!(manual_registry.has("ping"));
    assert!(macro_registry.has("ping"));
}

