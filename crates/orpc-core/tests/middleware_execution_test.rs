/// End-to-end test that middleware actually executes when calling procedures
use orpc_core::{os, HttpMethod, Next, OrpcError, OutputKind, ProcedureRegistry};
use serde_json::json;

#[derive(Clone, Debug)]
struct BaseContext {
    request_id: String,
}

#[derive(Clone, Debug)]
struct AuthContext {
    request_id: String,
    user_id: String,
}

#[derive(Clone, Debug)]
struct AdminContext {
    request_id: String,
    user_id: String,
    is_admin: bool,
}

// Middleware that adds authentication
async fn add_auth(ctx: BaseContext, next: Next<AuthContext>) -> Result<AuthContext, OrpcError> {
    let auth_ctx = AuthContext {
        request_id: ctx.request_id.clone(),
        user_id: "user_123".to_string(),
    };
    next.run(auth_ctx).await
}

// Middleware that adds admin flag
async fn add_admin(ctx: AuthContext, next: Next<AdminContext>) -> Result<AdminContext, OrpcError> {
    let admin_ctx = AdminContext {
        request_id: ctx.request_id.clone(),
        user_id: ctx.user_id.clone(),
        is_admin: true,
    };
    next.run(admin_ctx).await
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ProfileOutput {
    user_id: String,
    request_id: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct AdminOutput {
    user_id: String,
    is_admin: bool,
    request_id: String,
}

#[tokio::test]
async fn test_single_middleware_executes() {
    let proc = os()
        .context::<BaseContext>()
        .use_middleware(add_auth)
        .route(HttpMethod::Get, "/profile")
        .output::<ProfileOutput>()
        .handler(|ctx: AuthContext, _: ()| async move {
            Ok(ProfileOutput {
                user_id: ctx.user_id.clone(),
                request_id: ctx.request_id.clone(),
            })
        });

    let mut registry = ProcedureRegistry::new();
    registry.insert("/profile", &proc);

    // Call the procedure through the registry
    let ctx = BaseContext {
        request_id: "req_001".to_string(),
    };

    let result = registry.call("/profile", ctx, json!(null)).await;

    if let Err(ref e) = result {
        eprintln!(
            "Error calling /profile: code={}, message={}",
            e.code, e.message
        );
    }
    assert!(
        result.is_ok(),
        "Expected Ok but got error: {:?}",
        result.as_ref().err()
    );
    let output_kind = result.unwrap();
    match output_kind {
        OutputKind::Single(value) => {
            let output: ProfileOutput = serde_json::from_value(value).unwrap();
            assert_eq!(output.user_id, "user_123");
            assert_eq!(output.request_id, "req_001");
        }
        _ => panic!("Expected Single output"),
    }
}

#[tokio::test]
async fn test_chained_middleware_executes_in_order() {
    let proc = os()
        .context::<BaseContext>()
        .use_middleware(add_auth)
        .use_middleware(add_admin)
        .route(HttpMethod::Get, "/admin")
        .output::<AdminOutput>()
        .handler(|ctx: AdminContext, _: ()| async move {
            Ok(AdminOutput {
                user_id: ctx.user_id.clone(),
                is_admin: ctx.is_admin,
                request_id: ctx.request_id.clone(),
            })
        });

    let mut registry = ProcedureRegistry::new();
    registry.insert("/admin", &proc);

    let ctx = BaseContext {
        request_id: "req_002".to_string(),
    };

    let result = registry.call("/admin", ctx, json!(null)).await;

    assert!(result.is_ok());
    let output_kind = result.unwrap();
    match output_kind {
        OutputKind::Single(value) => {
            let output: AdminOutput = serde_json::from_value(value).unwrap();
            assert_eq!(output.user_id, "user_123");
            assert!(output.is_admin);
            assert_eq!(output.request_id, "req_002");
        }
        _ => panic!("Expected Single output"),
    }
}

#[tokio::test]
async fn test_middleware_error_prevents_handler_execution() {
    async fn failing_auth(
        _ctx: BaseContext,
        _next: Next<AuthContext>,
    ) -> Result<AuthContext, OrpcError> {
        Err(OrpcError::unauthorized("Invalid credentials"))
    }

    let proc = os()
        .context::<BaseContext>()
        .use_middleware(failing_auth)
        .route(HttpMethod::Get, "/protected")
        .output::<ProfileOutput>()
        .handler(|ctx: AuthContext, _: ()| async move {
            // This should never execute
            Ok(ProfileOutput {
                user_id: ctx.user_id,
                request_id: ctx.request_id,
            })
        });

    let mut registry = ProcedureRegistry::new();
    registry.insert("/protected", &proc);

    let ctx = BaseContext {
        request_id: "req_003".to_string(),
    };

    let result = registry.call("/protected", ctx, json!(null)).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, "UNAUTHORIZED");
    assert!(err.message.contains("Invalid credentials"));
}

#[tokio::test]
async fn test_middleware_modifies_context() {
    async fn append_to_request_id(
        ctx: BaseContext,
        next: Next<BaseContext>,
    ) -> Result<BaseContext, OrpcError> {
        let new_ctx = BaseContext {
            request_id: format!("{}-modified", ctx.request_id),
        };
        next.run(new_ctx).await
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    struct Output {
        request_id: String,
    }

    let proc = os()
        .context::<BaseContext>()
        .use_middleware(append_to_request_id)
        .route(HttpMethod::Get, "/test")
        .output::<Output>()
        .handler(|ctx: BaseContext, _: ()| async move {
            Ok(Output {
                request_id: ctx.request_id,
            })
        });

    let mut registry = ProcedureRegistry::new();
    registry.insert("/test", &proc);

    let ctx = BaseContext {
        request_id: "original".to_string(),
    };

    let result = registry.call("/test", ctx, json!(null)).await;

    assert!(result.is_ok());
    let output_kind = result.unwrap();
    match output_kind {
        OutputKind::Single(value) => {
            let output: Output = serde_json::from_value(value).unwrap();
            assert_eq!(output.request_id, "original-modified");
        }
        _ => panic!("Expected Single output"),
    }
}

#[tokio::test]
async fn test_no_middleware_works() {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Output {
        message: String,
    }

    let proc = os()
        .context::<BaseContext>()
        .route(HttpMethod::Get, "/ping")
        .output::<Output>()
        .handler(|ctx: BaseContext, _: ()| async move {
            Ok(Output {
                message: format!("pong from {}", ctx.request_id),
            })
        });

    let mut registry = ProcedureRegistry::new();
    registry.insert("/ping", &proc);

    let ctx = BaseContext {
        request_id: "req_004".to_string(),
    };

    let result = registry.call("/ping", ctx, json!(null)).await;

    assert!(result.is_ok());
    let output_kind = result.unwrap();
    match output_kind {
        OutputKind::Single(value) => {
            let output: Output = serde_json::from_value(value).unwrap();
            assert_eq!(output.message, "pong from req_004");
        }
        _ => panic!("Expected Single output"),
    }
}
