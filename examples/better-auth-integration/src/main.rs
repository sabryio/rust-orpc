//! Better Auth + orpc Integration Example
//!
//! Demonstrates how to integrate Better Auth RS authentication with orpc procedures
//! using orpc's native `.use_middleware()` for authentication — no Axum middleware
//! layer needed for auth logic.
//!
//! Context flow:
//!   BaseContext (every request)
//!     └─ .use_middleware(require_auth) ──► AuthContext (authenticated requests only)

mod auth_schema;

use auth_schema::AppAuthSchema;
use axum::{
    middleware::{self, Next},
    response::Response,
    Router,
};
use better_auth::seaorm::{sea_orm::Database, DatabaseConnection};
use better_auth::{
    integrations::axum::{AxumIntegration, OptionalSession},
    plugins::{EmailPasswordPlugin, SessionManagementPlugin},
    prelude::AuthUser,
    seaorm::SeaOrmStore,
    AuthConfig, BetterAuth,
};
use orpc_axum::AxumRouter;
use orpc_core::{openapi, os, router, HttpMethod, Next as OrpcNext, OrpcError, Stream};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tokio_stream::StreamExt;
use tower_http::cors::CorsLayer;

// ===== Types =====

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Planet {
    id: i32,
    name: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FindPlanetInput {
    id: i32,
}

#[derive(Debug, Deserialize)]
struct CreatePlanetInput {
    name: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListPlanetsPaginatedInput {
    limit: usize,
    offset: Option<usize>,
}

#[derive(Debug, Serialize)]
struct ListPlanetsPaginatedOutput {
    items: Vec<Planet>,
    #[serde(rename = "nextPageParam")]
    next_page_param: Option<usize>,
}

#[derive(Debug, Serialize)]
struct StreamEvent {
    message: String,
    count: u32,
}

// ===== Simplified user info =====

#[derive(Clone, Debug)]
struct AuthenticatedUser {
    id: String,
    email: Option<String>,
    name: Option<String>,
}

// ===== Contexts =====

/// Base context available on every request — no user required.
#[derive(Clone)]
struct BaseContext {
    #[allow(dead_code)]
    db: Arc<DatabaseConnection>,
    planets: Arc<tokio::sync::RwLock<Vec<Planet>>>,
    /// Populated by the Axum session-extraction layer.
    current_user: Option<AuthenticatedUser>,
}

impl BaseContext {
    fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db,
            planets: Arc::new(tokio::sync::RwLock::new(sample_planets())),
            current_user: None,
        }
    }
}

/// Authenticated context — only reachable after `require_auth` middleware succeeds.
#[derive(Clone)]
struct AuthContext {
    #[allow(dead_code)]
    db: Arc<DatabaseConnection>,
    planets: Arc<tokio::sync::RwLock<Vec<Planet>>>,
    user: AuthenticatedUser,
}

// ===== orpc Middleware =====

/// Middleware that enforces authentication and upgrades BaseContext → AuthContext.
///
/// Handlers that receive `AuthContext` are guaranteed to have a logged-in user.
async fn require_auth(
    ctx: BaseContext,
    next: OrpcNext<AuthContext>,
) -> Result<AuthContext, OrpcError> {
    let user = ctx
        .current_user
        .clone()
        .ok_or_else(|| OrpcError::unauthorized("Authentication required"))?;

    let auth_ctx = AuthContext {
        db: Arc::clone(&ctx.db),
        planets: Arc::clone(&ctx.planets),
        user,
    };

    next.run(auth_ctx).await
}

// ===== Sample data =====

fn sample_planets() -> Vec<Planet> {
    vec![
        Planet {
            id: 1,
            name: "Mercury".to_string(),
            description: Some("The smallest planet".to_string()),
        },
        Planet {
            id: 2,
            name: "Venus".to_string(),
            description: Some("The hottest planet".to_string()),
        },
        Planet {
            id: 3,
            name: "Earth".to_string(),
            description: Some("The blue planet".to_string()),
        },
        Planet {
            id: 4,
            name: "Mars".to_string(),
            description: Some("The red planet".to_string()),
        },
        Planet {
            id: 5,
            name: "Jupiter".to_string(),
            description: Some("The largest planet".to_string()),
        },
        Planet {
            id: 6,
            name: "Saturn".to_string(),
            description: Some("The ringed planet".to_string()),
        },
        Planet {
            id: 7,
            name: "Uranus".to_string(),
            description: Some("The ice giant".to_string()),
        },
        Planet {
            id: 8,
            name: "Neptune".to_string(),
            description: Some("The windiest planet".to_string()),
        },
        Planet {
            id: 9,
            name: "Pluto".to_string(),
            description: Some("The dwarf planet".to_string()),
        },
    ]
}

// ===== Axum session layer =====
//
// This thin Axum middleware only does one thing: extract the Better Auth session
// and store it in Axum request extensions.  All auth *enforcement* is done by
// the orpc `require_auth` middleware above.

async fn session_layer(
    session: OptionalSession<AppAuthSchema>,
    mut req: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Response {
    if let Some(session_data) = session.0 {
        let user = AuthenticatedUser {
            id: session_data.user.id().to_string(),
            email: session_data.user.email().map(|e| e.to_string()),
            name: session_data.user.name().map(|n| n.to_string()),
        };
        req.extensions_mut().insert(user);
    }
    next.run(req).await
}

/// Per-request context builder: pulls the user from extensions (if present)
/// and sets it on BaseContext so the orpc middleware can inspect it.
fn build_context(mut ctx: BaseContext, ext: &axum::http::Extensions) -> BaseContext {
    ctx.current_user = ext.get::<AuthenticatedUser>().cloned();
    ctx
}

// ===== Main =====

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Better Auth + orpc integration example...");

    let database_url = "sqlite::memory:";
    let database = Database::connect(database_url).await?;
    auth_schema::run_app_migrations(&database).await?;

    let secret = "your-very-secure-secret-key-at-least-32-chars-long-for-production-use-only";
    let config = AuthConfig::new(secret)
        .base_url("http://localhost:3000")
        .password_min_length(8);

    let store = SeaOrmStore::<AppAuthSchema>::new(config.clone(), database.clone());

    let auth = Arc::new(
        BetterAuth::<AppAuthSchema>::new(config)
            .store(store)
            .plugin(EmailPasswordPlugin::new().enable_signup(true))
            .plugin(SessionManagementPlugin::new())
            .build()
            .await?,
    );

    let base_ctx = BaseContext::new(Arc::new(database));

    // ── orpc router ──────────────────────────────────────────────────────────
    //
    // Public procedures use BaseContext directly.
    // Protected procedures call .use_middleware(require_auth) which upgrades
    // the context to AuthContext — the handler receives AuthContext and is
    // guaranteed to have `ctx.user`.

    // Define once — base procedure with auth middleware already applied
    let protected = os().context::<BaseContext>().use_middleware(require_auth);

    let orpc_router = router! {
        ping: os()
            .context::<BaseContext>()
            .meta(openapi!{ method: "POST", path: "/ping" })
            .output::<String>()
            .handler(|ctx: BaseContext, _: ()| async move {
                match &ctx.current_user {
                    Some(u) => Ok(format!(
                        "pong (authenticated as {})",
                        u.email.as_deref().unwrap_or("unknown")
                    )),
                    None => Ok("pong (anonymous)".to_string()),
                }
            }),

        planet: {
            list: os()
                .context::<BaseContext>()
                .meta(openapi!{ method: "POST", path: "/planet/list" })
                .output::<Vec<Planet>>()
                .handler(|ctx: BaseContext, _: ()| async move {
                    Ok(ctx.planets.read().await.clone())
                }),

            listPaginated: os()
                .context::<BaseContext>()
                .route(HttpMethod::Post, "/planet/list-paginated")
                .input::<ListPlanetsPaginatedInput>()
                .output::<ListPlanetsPaginatedOutput>()
                .handler(|ctx: BaseContext, input: ListPlanetsPaginatedInput| async move {
                    let planets = ctx.planets.read().await;
                    let offset = input.offset.unwrap_or(0);
                    let items: Vec<Planet> = planets
                        .iter()
                        .skip(offset)
                        .take(input.limit)
                        .cloned()
                        .collect();
                    let next_page_param = if offset + input.limit < planets.len() {
                        Some(offset + input.limit)
                    } else {
                        None
                    };
                    Ok(ListPlanetsPaginatedOutput { items, next_page_param })
                }),

            find: os()
                .context::<BaseContext>()
                .meta(openapi!{ method: "POST", path: "/planet/find" })
                .input::<FindPlanetInput>()
                .output::<Planet>()
                .handler(|ctx: BaseContext, input: FindPlanetInput| async move {
                    ctx.planets
                        .read()
                        .await
                        .iter()
                        .find(|p| p.id == input.id)
                        .cloned()
                        .ok_or_else(|| OrpcError::not_found(format!("Planet {} not found", input.id)))
                }),

            // Protected: requires authentication via orpc middleware
            create: protected.clone()
                .route(HttpMethod::Post, "/planet/create")
                .input::<CreatePlanetInput>()
                .output::<Planet>()
                .handler(|ctx: AuthContext, input: CreatePlanetInput| async move {
                    // ctx.user is guaranteed — no manual auth check needed
                    if input.name.trim().is_empty() {
                        return Err(OrpcError::bad_request("Planet name cannot be empty"));
                    }
                    let mut planets = ctx.planets.write().await;
                    let new_id = planets.iter().map(|p| p.id).max().unwrap_or(0) + 1;
                    let planet = Planet { id: new_id, name: input.name, description: input.description };
                    planets.push(planet.clone());
                    Ok(planet)
                })
        },

        // Protected: requires authentication via orpc middleware
        profile: protected.clone()
            .route(HttpMethod::Post, "/profile")
            .output::<serde_json::Value>()
            .handler(|ctx: AuthContext, _: ()| async move {
                // ctx.user is guaranteed — direct field access, no Option unwrap
                Ok(serde_json::json!({
                    "id":    ctx.user.id,
                    "email": ctx.user.email,
                    "name":  ctx.user.name,
                }))
            }),

        stream: os()
            .context::<BaseContext>()
            .route(HttpMethod::Post, "/stream")
            .output::<Stream<StreamEvent>>()
            .handler(|_ctx: BaseContext, _: ()| async {
                let stream = tokio_stream::iter(0u32..)
                    .throttle(Duration::from_secs(1))
                    .take(10)
                    .map(|count| StreamEvent { message: format!("Event #{count}"), count });
                Ok(stream)
            }),

        stream_async: os()
            .context::<BaseContext>()
            .route(HttpMethod::Post, "/stream-async")
            .output::<Stream<StreamEvent>>()
            .handler(|_ctx: BaseContext, _: ()| async {
                use async_stream::stream;
                let s = stream! {
                    for i in 0u32..15 {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        yield StreamEvent { message: format!("Async Stream Event #{i}"), count: i };
                    }
                };
                Ok(s)
            })
    }
    // build_context populates current_user from Axum extensions each request;
    // require_auth middleware then enforces auth on protected routes.
    .into_axum_router_with(base_ctx, build_context);

    let auth_router = auth.clone().axum_router();

    // Wrap the orpc router with the session extraction layer so that
    // every request has a chance to have its user populated.
    let orpc_with_session =
        orpc_router.layer(middleware::from_fn_with_state(auth.clone(), session_layer));

    let app = Router::new()
        .nest("/rpc", orpc_with_session)
        .merge(
            Router::new()
                .nest("/api/auth", auth_router)
                .with_state(auth.clone()),
        )
        .layer(
            CorsLayer::new()
                .allow_origin([
                    "http://localhost:5173"
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                    "http://localhost:3000"
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                ])
                .allow_methods([
                    axum::http::Method::GET,
                    axum::http::Method::POST,
                    axum::http::Method::PUT,
                    axum::http::Method::DELETE,
                    axum::http::Method::OPTIONS,
                ])
                .allow_headers([
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::AUTHORIZATION,
                    axum::http::header::ACCEPT,
                ])
                .allow_credentials(true),
        );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001").await?;

    println!("🚀 Server running on http://127.0.0.1:3001");
    println!();
    println!("📚 Better Auth Endpoints (under /api/auth):");
    println!("   POST /api/auth/sign-up/email    - Create account");
    println!("   POST /api/auth/sign-in/email    - Sign in");
    println!("   POST /api/auth/sign-out         - Sign out");
    println!("   GET  /api/auth/get-session      - Get current session");
    println!();
    println!("🔧 orpc Endpoints (under /rpc):");
    println!("   POST /rpc/ping                  - Public ping");
    println!("   POST /rpc/profile               - User profile  [auth required]");
    println!("   POST /rpc/planet/list           - List planets  [public]");
    println!("   POST /rpc/planet/list-paginated - Paginated     [public]");
    println!("   POST /rpc/planet/find           - Find by ID    [public]");
    println!("   POST /rpc/planet/create         - Create planet [auth required]");
    println!("   POST /rpc/stream                - SSE stream    [public]");
    println!("   POST /rpc/stream-async          - SSE async     [public]");
    println!();
    println!("   Press Ctrl+C to shutdown");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    println!("\n✨ Server shutdown complete");
    Ok(())
}

async fn shutdown_signal() {
    use tokio::signal;
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c     => println!("\n🛑 Received Ctrl+C, shutting down gracefully..."),
        _ = terminate  => println!("\n🛑 Received termination signal, shutting down gracefully..."),
    }
}
