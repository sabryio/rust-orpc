//! Better Auth + orpc Integration Example
//!
//! Demonstrates how to integrate Better Auth RS authentication with orpc procedures.
//!
//! This example shows:
//! - Setting up Better Auth with email/password authentication
//! - Using Axum middleware to extract sessions
//! - Passing authenticated user through orpc context
//! - Protecting routes with authentication checks
//! - Accessing user information in orpc handlers

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
use orpc_core::{os, router, HttpMethod, OrpcError};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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

// ===== Application Context =====

/// Application context that includes optional authenticated user
#[derive(Clone)]
struct AppContext {
    #[allow(dead_code)]
    db: Arc<DatabaseConnection>,
    planets: Arc<tokio::sync::RwLock<Vec<Planet>>>,
    // Optional - only present for authenticated requests
    current_user: Option<AuthenticatedUser>,
}

/// Simplified user info extracted from Better Auth session
#[derive(Clone, Debug)]
struct AuthenticatedUser {
    id: String,
    email: Option<String>,
    name: Option<String>,
}

impl AppContext {
    fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db,
            planets: Arc::new(tokio::sync::RwLock::new(sample_planets())),
            current_user: None,
        }
    }

    #[allow(dead_code)]
    fn with_user(mut self, user: AuthenticatedUser) -> Self {
        self.current_user = Some(user);
        self
    }

    fn require_auth(&self) -> Result<&AuthenticatedUser, OrpcError> {
        self.current_user
            .as_ref()
            .ok_or_else(|| OrpcError::unauthorized("Authentication required"))
    }
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

// ===== Context Extractor =====

/// Extract authenticated user and create context with it
/// This is called by orpc-axum for each request
fn extract_context(base_ctx: AppContext, extensions: &axum::http::Extensions) -> AppContext {
    // Check if auth middleware added a user to extensions
    if let Some(user) = extensions.get::<AuthenticatedUser>() {
        base_ctx.with_user(user.clone())
    } else {
        base_ctx
    }
}

// ===== Middleware =====

/// Middleware that extracts Better Auth session and adds it to extensions
async fn auth_middleware(
    session: OptionalSession<AppAuthSchema>,
    mut req: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Response {
    // Store user info in extensions if session exists
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

// ===== Main =====

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Better Auth + orpc integration example...");

    // Setup database
    let database_url = "sqlite::memory:";
    let database = Database::connect(database_url).await?;
    auth_schema::run_app_migrations(&database).await?;

    // Setup Better Auth
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

    // Setup application context (without user - will be added per-request)
    let base_ctx = AppContext::new(Arc::new(database));

    // Build orpc router
    let orpc_router = router! {
        // Public route - no authentication required
        ping: os()
            .context::<AppContext>()
            .route(HttpMethod::Post, "/ping")
            .output::<String>()
            .handler(|ctx, _: ()| async move {
                if let Some(user) = &ctx.current_user {
                    Ok(format!("pong (authenticated as {})", user.email.as_deref().unwrap_or("unknown")))
                } else {
                    Ok("pong (anonymous)".to_string())
                }
            }),

        planet: {
            // List all planets (public - no auth required)
            list: os()
                .context::<AppContext>()
                .route(HttpMethod::Post, "/planet/list")
                .output::<Vec<Planet>>()
                .handler(|ctx, _: ()| async move {
                    let planets = ctx.planets.read().await;
                    Ok(planets.clone())
                }),

            // Paginated planet list (public)
            listPaginated: os()
                .context::<AppContext>()
                .route(HttpMethod::Post, "/planet/list-paginated")
                .input::<ListPlanetsPaginatedInput>()
                .output::<ListPlanetsPaginatedOutput>()
                .handler(|ctx, input: ListPlanetsPaginatedInput| async move {
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

            // Find planet by ID (public)
            find: os()
                .context::<AppContext>()
                .route(HttpMethod::Post, "/planet/find")
                .input::<FindPlanetInput>()
                .output::<Planet>()
                .handler(|ctx, input: FindPlanetInput| async move {
                    let planets = ctx.planets.read().await;
                    planets
                        .iter()
                        .find(|p| p.id == input.id)
                        .cloned()
                        .ok_or_else(|| OrpcError::not_found(format!("Planet with id {} not found", input.id)))
                }),

            // Create a new planet (requires auth)
            create: os()
                .context::<AppContext>()
                .route(HttpMethod::Post, "/planet/create")
                .input::<CreatePlanetInput>()
                .output::<Planet>()
                .handler(|ctx, input: CreatePlanetInput| async move {
                    // Require authentication for creating planets
                    let _user = ctx.require_auth()?;

                    if input.name.trim().is_empty() {
                        return Err(OrpcError::bad_request("Planet name cannot be empty"));
                    }

                    let mut planets = ctx.planets.write().await;
                    let new_id = planets.iter().map(|p| p.id).max().unwrap_or(0) + 1;

                    let planet = Planet {
                        id: new_id,
                        name: input.name,
                        description: input.description,
                    };

                    planets.push(planet.clone());
                    Ok(planet)
                })
        },

        // Profile route (requires auth)
        profile: os()
            .context::<AppContext>()
            .route(HttpMethod::Post, "/profile")
            .output::<serde_json::Value>()
            .handler(|ctx, _: ()| async move {
                let user = ctx.require_auth()?;

                Ok(serde_json::json!({
                    "id": user.id,
                    "email": user.email,
                    "name": user.name,
                }))
            })
    }
    .into_axum_router_with(base_ctx, extract_context);

    // Get Better Auth's router (it needs state)
    let auth_router = auth.clone().axum_router();

    // Build complete app with Better Auth routes and orpc routes
    // We need to handle state carefully since auth_router needs Arc<BetterAuth> state
    // but orpc_router has no state
    // Auth middleware must wrap the orpc router directly so its extensions
    // are visible to the context extractor on every /rpc request.
    let orpc_with_auth = orpc_router.layer(middleware::from_fn_with_state(
        auth.clone(),
        auth_middleware,
    ));

    let app = Router::new()
        .nest("/rpc", orpc_with_auth)
        .merge(
            Router::new()
                .nest("/api/auth", auth_router)
                .with_state(auth.clone()),
        )
        // CORS for development
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
    println!("   POST /api/auth/sign-up/email        - Create account");
    println!("   POST /api/auth/sign-in/email        - Sign in");
    println!("   POST /api/auth/sign-out             - Sign out");
    println!("   GET  /api/auth/get-session          - Get current session");
    println!("   GET  /api/auth/list-sessions        - List all sessions");
    println!("   POST /api/auth/revoke-session       - Revoke a session");
    println!();
    println!("🔧 orpc Endpoints (under /rpc):");
    println!("   POST /rpc/ping                      - Public ping (shows auth status)");
    println!("   POST /rpc/profile                   - Get user profile (requires auth)");
    println!("   POST /rpc/planet/list               - List all planets (public)");
    println!("   POST /rpc/planet/list-paginated     - List planets with pagination (public)");
    println!("   POST /rpc/planet/find               - Find planet by ID (public)");
    println!("   POST /rpc/planet/create             - Create planet (requires auth)");
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
        _ = ctrl_c => {
            println!("\n🛑 Received Ctrl+C, shutting down gracefully...");
        },
        _ = terminate => {
            println!("\n🛑 Received termination signal, shutting down gracefully...");
        },
    }
}
