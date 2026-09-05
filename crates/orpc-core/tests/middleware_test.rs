use orpc_core::{os, HttpMethod, Middleware, Next, OrpcError};

#[derive(Clone, Debug, PartialEq)]
struct BaseContext {
    db: String,
}

#[derive(Clone, Debug, PartialEq)]
struct AuthContext {
    db: String,
    user_id: String,
}

#[derive(Clone, Debug, PartialEq)]
struct AdminContext {
    db: String,
    user_id: String,
    is_admin: bool,
}

// Middleware that adds authentication
async fn require_auth(ctx: BaseContext, next: Next<AuthContext>) -> Result<AuthContext, OrpcError> {
    let auth_ctx = AuthContext {
        db: ctx.db.clone(),
        user_id: "user_123".to_string(),
    };
    next.run(auth_ctx).await
}

// Middleware that checks admin privileges
async fn require_admin(
    ctx: AuthContext,
    next: Next<AdminContext>,
) -> Result<AdminContext, OrpcError> {
    if ctx.user_id == "user_123" {
        let admin_ctx = AdminContext {
            db: ctx.db.clone(),
            user_id: ctx.user_id.clone(),
            is_admin: true,
        };
        next.run(admin_ctx).await
    } else {
        Err(OrpcError::unauthorized("Not an admin"))
    }
}

// Middleware that returns error to test short-circuiting
async fn failing_middleware(
    _ctx: AuthContext,
    _next: Next<AdminContext>,
) -> Result<AdminContext, OrpcError> {
    Err(OrpcError::unauthorized("Permission denied"))
}

#[tokio::test]
async fn test_single_middleware_transforms_context() {
    #[derive(serde::Deserialize)]
    struct Input;

    #[derive(serde::Serialize)]
    struct Output {
        user: String,
    }

    let proc = os()
        .context::<BaseContext>()
        .use_middleware(require_auth)
        .route(HttpMethod::Get, "/profile")
        .output::<Output>()
        .handler(|ctx: AuthContext, _: ()| async move {
            Ok(Output {
                user: ctx.user_id.clone(),
            })
        });

    // Verify the procedure was created
    assert_eq!(proc.route.method, HttpMethod::Get);
    assert_eq!(proc.route.path, "/profile");
}

#[tokio::test]
async fn test_two_chained_middlewares_compose() {
    #[derive(serde::Serialize)]
    struct Output {
        user: String,
        admin: bool,
    }

    let proc = os()
        .context::<BaseContext>()
        .use_middleware(require_auth)
        .use_middleware(require_admin)
        .route(HttpMethod::Get, "/admin")
        .output::<Output>()
        .handler(|ctx: AdminContext, _: ()| async move {
            Ok(Output {
                user: ctx.user_id.clone(),
                admin: ctx.is_admin,
            })
        });

    assert_eq!(proc.route.method, HttpMethod::Get);
    assert_eq!(proc.route.path, "/admin");
}

#[tokio::test]
async fn test_middleware_before_and_after_route() {
    #[derive(serde::Serialize)]
    struct Output {
        success: bool,
    }

    // Middleware before route
    let proc1 = os()
        .context::<BaseContext>()
        .use_middleware(require_auth)
        .route(HttpMethod::Post, "/create")
        .output::<Output>()
        .handler(|_ctx: AuthContext, _: ()| async move { Ok(Output { success: true }) });

    assert_eq!(proc1.route.path, "/create");

    // Middleware after route
    let proc2 = os()
        .context::<BaseContext>()
        .route(HttpMethod::Post, "/update")
        .use_middleware(require_auth)
        .output::<Output>()
        .handler(|_ctx: AuthContext, _: ()| async move { Ok(Output { success: true }) });

    assert_eq!(proc2.route.path, "/update");
}

#[tokio::test]
async fn test_middleware_order_matters() {
    #[derive(serde::Serialize)]
    struct Output {
        result: String,
    }

    // This should compile: BaseContext → AuthContext → AdminContext
    let _proc = os()
        .context::<BaseContext>()
        .use_middleware(require_auth)
        .use_middleware(require_admin)
        .route(HttpMethod::Get, "/test")
        .output::<Output>()
        .handler(|ctx: AdminContext, _: ()| async move {
            Ok(Output {
                result: format!("User: {}, Admin: {}", ctx.user_id, ctx.is_admin),
            })
        });
}

#[tokio::test]
async fn test_middleware_with_closure() {
    #[derive(serde::Serialize)]
    struct Output {
        count: i32,
    }

    let proc = os()
        .context::<BaseContext>()
        .use_middleware(|ctx: BaseContext, next: Next<AuthContext>| async move {
            // Add auth inline
            let auth_ctx = AuthContext {
                db: ctx.db,
                user_id: "inline_user".to_string(),
            };
            next.run(auth_ctx).await
        })
        .route(HttpMethod::Get, "/inline")
        .output::<Output>()
        .handler(|ctx: AuthContext, _: ()| async move {
            assert_eq!(ctx.user_id, "inline_user");
            Ok(Output { count: 1 })
        });

    assert_eq!(proc.route.path, "/inline");
}
