use axum::{
    extract::State,
    http::StatusCode,
    response::sse::Event,
    response::{IntoResponse, Json, Response, Sse},
    routing::post,
    Router,
};
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, sync::Arc, time::Duration};
use tokio_stream::StreamExt;
use tower_http::cors::{Any, CorsLayer};

// ===== Simple In-Memory State =====

#[derive(Clone)]
struct AppState {
    planets: Arc<Vec<Planet>>,
}

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

// ===== Simple RPC Error Type =====

#[derive(Debug, Serialize)]
struct RpcError {
    code: String,
    message: String,
}

impl RpcError {
    fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: "NOT_FOUND".into(),
            message: message.into(),
        }
    }

    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            code: "BAD_REQUEST".into(),
            message: message.into(),
        }
    }
}

impl IntoResponse for RpcError {
    fn into_response(self) -> Response {
        let status = match self.code.as_str() {
            "NOT_FOUND" => StatusCode::NOT_FOUND,
            "BAD_REQUEST" => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, Json(self)).into_response()
    }
}

// ===== RPC Handlers =====

async fn ping() -> Json<String> {
    Json("pong".to_string())
}

async fn list_planets(State(state): State<AppState>) -> Json<Vec<Planet>> {
    Json(state.planets.to_vec())
}

async fn list_planets_paginated(
    State(state): State<AppState>,
    Json(input): Json<ListPlanetsPaginatedInput>,
) -> Json<ListPlanetsPaginatedOutput> {
    let offset = input.offset.unwrap_or(0);

    // Note: Offset pagination is O(n) for skipping — acceptable for demo with 12 items.
    // For large datasets, consider cursor-based pagination.
    let items: Vec<Planet> = state
        .planets
        .iter()
        .skip(offset)
        .take(input.limit)
        .cloned()
        .collect();

    let next_page_param = if offset + input.limit < state.planets.len() {
        Some(offset + input.limit)
    } else {
        None
    };

    Json(ListPlanetsPaginatedOutput {
        items,
        next_page_param,
    })
}

async fn find_planet(
    State(state): State<AppState>,
    Json(input): Json<FindPlanetInput>,
) -> Result<Json<Planet>, RpcError> {
    state
        .planets
        .iter()
        .find(|p| p.id == input.id)
        .cloned()
        .map(Json)
        .ok_or_else(|| RpcError::not_found(format!("Planet with id {} not found", input.id)))
}

async fn create_planet(
    State(state): State<AppState>,
    Json(input): Json<CreatePlanetInput>,
) -> Result<Json<Planet>, RpcError> {
    if input.name.trim().is_empty() {
        return Err(RpcError::bad_request("Planet name cannot be empty"));
    }

    // In a real app, this would insert into a database
    let new_planet = Planet {
        id: state.planets.len() as i32 + 1,
        name: input.name,
        description: input.description,
    };

    Ok(Json(new_planet))
}

// ===== Streaming =====

#[derive(Debug, Serialize)]
struct StreamEvent {
    message: String,
    count: u32,
}

async fn stream_events() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // oRPC OpenAPIHandler sends an initial empty comment to flush headers
    // immediately so the client starts iterating without waiting for the first event.
    let initial = tokio_stream::iter([Ok(Event::default().comment(""))]);

    let events = tokio_stream::iter(0u32..)
        .throttle(Duration::from_secs(1))
        .take(10)
        .map(|count| {
            let payload = serde_json::to_string(&StreamEvent {
                message: format!("Event #{count}"),
                count,
            })
            .unwrap();

            // oRPC reads the SSE `event:` field via EventStreamDecoderStream:
            //   event: message  → yield (data event)
            //   event: error    → throw (error event)
            //   event: close    → end of stream (return)
            Ok(Event::default()
                .event("message")
                .id(count.to_string())
                .retry(Duration::from_secs(5))
                .data(payload))
        })
        .chain(tokio_stream::iter([Ok(Event::default()
            .event("close")
            .data(""))]));

    Sse::new(initial.chain(events)).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text(""),
    )
}

async fn stream_async() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    use async_stream::stream;

    let s = stream! {
        // Initial comment to flush headers immediately
        yield Ok(Event::default().comment(""));

        // Yield 15 events, one per second
        for i in 0u32..15 {
            tokio::time::sleep(Duration::from_secs(1)).await;

            let payload = serde_json::to_string(&StreamEvent {
                message: format!("Async Stream Event #{i}"),
                count: i,
            })
            .unwrap();

            yield Ok(Event::default()
                .event("message")
                .id(i.to_string())
                .retry(Duration::from_secs(5))
                .data(payload));
        }

        // Final close event
        yield Ok(Event::default().event("close").data(""));
    };

    Sse::new(s).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text(""),
    )
}

// ===== Main =====

#[tokio::main]
async fn main() {
    // Initialize state with sample data
    let state = AppState {
        planets: Arc::new(vec![
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
        ]),
    };

    // Build our application with routes
    let app = Router::new()
        .route("/rpc/ping", post(ping))
        .route("/rpc/planet/list", post(list_planets))
        .route("/rpc/planet/list-paginated", post(list_planets_paginated))
        .route("/rpc/planet/find", post(find_planet))
        .route("/rpc/planet/create", post(create_planet))
        .route("/rpc/stream", post(stream_events))
        .route("/rpc/stream-async", post(stream_async))
        // Add CORS middleware
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    // Run the server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001")
        .await
        .unwrap();

    println!("🚀 Server running on http://127.0.0.1:3001");
    println!("   - POST /rpc/ping");
    println!("   - POST /rpc/planet/list");
    println!("   - POST /rpc/planet/list-paginated");
    println!("   - POST /rpc/planet/find");
    println!("   - POST /rpc/planet/create");
    println!("   - POST /rpc/stream");
    println!("   - POST /rpc/stream-async");

    axum::serve(listener, app).await.unwrap();
}
