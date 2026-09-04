//! orpc-server — same API as `server` (main.rs) but implemented using
//! `orpc-core` + `orpc-axum` + `router!` macro.
//!
//! Serves on the same port (3001) so the React client works unchanged.
//! Includes SSE streaming support via the orpc-axum integration.
//!
//! Run with:
//!   cargo run --bin orpc-server
//!   (from examples/axum-react/server/)

use orpc_axum::AxumRouter;
use orpc_core::{os, router, HttpMethod, OrpcError, Stream};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tokio_stream::StreamExt;

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

// ===== Context =====

#[derive(Clone)]
struct AppContext {
    planets: Arc<Vec<Planet>>,
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
        Planet {
            id: 10,
            name: "Ceres".to_string(),
            description: Some("Dwarf planet in asteroid belt".to_string()),
        },
        Planet {
            id: 11,
            name: "Eris".to_string(),
            description: Some("Distant dwarf planet".to_string()),
        },
        Planet {
            id: 12,
            name: "Haumea".to_string(),
            description: Some("Egg-shaped dwarf planet".to_string()),
        },
    ]
}

// ===== Main =====

#[tokio::main]
async fn main() {
    let ctx = AppContext {
        planets: Arc::new(sample_planets()),
    };

    // The client expects routes under /rpc — route paths include the prefix.
    let app = router! {

        ping: os()
            .context::<AppContext>()
            .route(HttpMethod::Post, "/rpc/ping")
            .output::<String>()
            .handler(|_ctx, _: ()| async { Ok("pong".to_string()) }),

        planet: {
            list: os()
                .context::<AppContext>()
                .route(HttpMethod::Post, "/rpc/planet/list")
                .output::<Vec<Planet>>()
                .handler(|ctx, _: ()| async move {
                    Ok(ctx.planets.to_vec())
                }),

            listPaginated: os()
                .context::<AppContext>()
                .route(HttpMethod::Post, "/rpc/planet/list-paginated")
                .input::<ListPlanetsPaginatedInput>()
                .output::<ListPlanetsPaginatedOutput>()
                .handler(|ctx, input: ListPlanetsPaginatedInput| async move {
                    let offset = input.offset.unwrap_or(0);
                    let items: Vec<Planet> = ctx.planets
                        .iter()
                        .skip(offset)
                        .take(input.limit)
                        .cloned()
                        .collect();

                    let next_page_param = if offset + input.limit < ctx.planets.len() {
                        Some(offset + input.limit)
                    } else {
                        None
                    };

                    Ok(ListPlanetsPaginatedOutput { items, next_page_param })
                }),

            find: os()
                .context::<AppContext>()
                .route(HttpMethod::Post, "/rpc/planet/find")
                .input::<FindPlanetInput>()
                .output::<Planet>()
                .handler(|ctx, input: FindPlanetInput| async move {
                    ctx.planets
                        .iter()
                        .find(|p| p.id == input.id)
                        .cloned()
                        .ok_or_else(|| OrpcError::not_found(format!("Planet with id {} not found", input.id)))
                }),

            create: os()
                .context::<AppContext>()
                .route(HttpMethod::Post, "/rpc/planet/create")
                .input::<CreatePlanetInput>()
                .output::<Planet>()
                .handler(|ctx, input: CreatePlanetInput| async move {
                    if input.name.trim().is_empty() {
                        return Err(OrpcError::bad_request("Planet name cannot be empty"));
                    }
                    if input.name.len() > 100 {
                        return Err(OrpcError::internal("Planet name is too long (max 100 characters)"));
                    }
                    Ok(Planet {
                        id: ctx.planets.len() as i32 + 1,
                        name: input.name,
                        description: input.description,
                    })
                })
        },

        stream: os()
            .context::<AppContext>()
            .route(HttpMethod::Post, "/rpc/stream")
            .output::<Stream<StreamEvent>>()
            .handler(|_ctx, _: ()| async {
                // Create a stream that emits 10 events, one per second
                let stream = tokio_stream::iter(0u32..)
                    .throttle(Duration::from_secs(1))
                    .take(10)
                    .map(|count| StreamEvent {
                        message: format!("Event #{count}"),
                        count,
                    });

                Ok(stream)
            }),

        streamAsync: os()
            .context::<AppContext>()
            .route(HttpMethod::Post, "/rpc/stream-async")
            .output::<Stream<StreamEvent>>()
            .handler(|_ctx, _: ()| async {
                use async_stream::stream;

                // Create an async stream using the async_stream crate
                let s = stream! {
                    for i in 0u32..15 {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        yield StreamEvent {
                            message: format!("Async Stream Event #{i}"),
                            count: i,
                        };
                    }
                };

                Ok(s)
            })
    }
    .into_axum_router(ctx);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001")
        .await
        .unwrap();

    println!("🚀 orpc-server running on http://127.0.0.1:3001");
    println!("   POST /rpc/ping");
    println!("   POST /rpc/planet/list");
    println!("   POST /rpc/planet/list-paginated");
    println!("   POST /rpc/planet/find");
    println!("   POST /rpc/planet/create");
    println!("   POST /rpc/stream             (SSE streaming)");
    println!("   POST /rpc/stream-async       (SSE streaming)");
    println!();
    println!("   ⚠️  WebSocket (/ws) not yet supported");

    axum::serve(listener, app).await.unwrap();
}
