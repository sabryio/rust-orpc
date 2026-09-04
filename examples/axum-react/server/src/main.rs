use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::StatusCode,
    response::sse::Event,
    response::{IntoResponse, Json, Response, Sse},
    routing::{get, post},
    Router,
};
use futures::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, convert::Infallible, sync::Arc, time::Duration};
use tokio::sync::Mutex;
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

    fn internal_error(message: impl Into<String>) -> Self {
        Self {
            code: "INTERNAL_ERROR".into(),
            message: message.into(),
        }
    }
}

impl IntoResponse for RpcError {
    fn into_response(self) -> Response {
        let status = match self.code.as_str() {
            "NOT_FOUND" => StatusCode::NOT_FOUND,
            "BAD_REQUEST" => StatusCode::BAD_REQUEST,
            "INTERNAL_ERROR" => StatusCode::INTERNAL_SERVER_ERROR,
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

    if input.name.len() > 100 {
        return Err(RpcError::internal_error(
            "Planet name is too long (max 100 characters)",
        ));
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

// ===== WebSocket — oRPC Peer Protocol =====

/// The RPC body envelope: { "json": <value>, "meta": [] }
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RpcBody {
    json: Value,
    #[serde(default)]
    meta: Vec<Value>,
}

/// Incoming request payload inside a peer message
#[derive(Debug, Deserialize)]
struct PeerRequestJson {
    url: String,
    #[serde(default = "default_method")]
    method: String,
    body: Option<RpcBody>,
}

fn default_method() -> String {
    "POST".to_string()
}

/// A peer message sent by the client
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum ClientMessage {
    Request { id: String, json: PeerRequestJson },
    Cancel { id: String },
}

/// A peer message sent by the server
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum ServerMessage {
    Response {
        id: String,
        json: ServerResponseJson,
    },
    EventStream {
        id: String,
        json: EventStreamJson,
    },
    Cancel {
        id: String,
    },
}

#[derive(Debug, Serialize)]
struct ServerResponseJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<RpcBody>,
}

#[derive(Debug, Serialize)]
struct EventStreamJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    event: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

type WsSender = Arc<Mutex<futures::stream::SplitSink<WebSocket, Message>>>;

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    use futures::StreamExt as FuturesStreamExt;

    let (sink, mut stream) = socket.split();
    let sender: WsSender = Arc::new(Mutex::new(sink));

    // Track spawned tasks so we can abort on cancel
    let tasks: Arc<Mutex<HashMap<String, tokio::task::JoinHandle<()>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    while let Some(Ok(msg)) = futures::StreamExt::next(&mut stream).await {
        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) => break,
            _ => continue,
        };

        let client_msg: ClientMessage = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(_) => continue,
        };

        match client_msg {
            ClientMessage::Cancel { id } => {
                if let Some(handle) = tasks.lock().await.remove(&id) {
                    handle.abort();
                }
            }
            ClientMessage::Request { id, json: req_json } => {
                let sender_clone = Arc::clone(&sender);
                let state_clone = state.clone();
                let id_clone = id.clone();
                let tasks_clone = Arc::clone(&tasks);

                let handle = tokio::spawn(async move {
                    dispatch_ws(id_clone.clone(), req_json, sender_clone, state_clone).await;
                    tasks_clone.lock().await.remove(&id_clone);
                });

                tasks.lock().await.insert(id, handle);
            }
        }
    }

    // Abort all in-flight tasks when connection closes
    let mut locked = tasks.lock().await;
    for (_, handle) in locked.drain() {
        handle.abort();
    }
}

async fn send_ws(sender: &WsSender, msg: ServerMessage) {
    use futures::SinkExt;
    if let Ok(text) = serde_json::to_string(&msg) {
        let _ = sender.lock().await.send(Message::Text(text.into())).await;
    }
}

async fn dispatch_ws(id: String, req: PeerRequestJson, sender: WsSender, state: AppState) {
    // Strip /rpc prefix and split into path segments
    let path = req.url.trim_start_matches("/rpc");
    let input = req.body.map(|b| b.json);

    match path {
        "/ping" => {
            send_ws(
                &sender,
                ServerMessage::Response {
                    id,
                    json: ServerResponseJson {
                        status: None,
                        headers: None,
                        body: Some(RpcBody {
                            json: Value::String("pong".to_string()),
                            meta: vec![],
                        }),
                    },
                },
            )
            .await;
        }

        "/planet/list" => {
            let planets: Vec<Value> = state
                .planets
                .iter()
                .map(|p| serde_json::to_value(p).unwrap())
                .collect();
            send_ws(
                &sender,
                ServerMessage::Response {
                    id,
                    json: ServerResponseJson {
                        status: None,
                        headers: None,
                        body: Some(RpcBody {
                            json: Value::Array(planets),
                            meta: vec![],
                        }),
                    },
                },
            )
            .await;
        }

        "/planet/find" => {
            let result: Result<Value, RpcError> = (|| {
                let input = input.ok_or_else(|| RpcError::bad_request("Missing input"))?;
                let id_val = input
                    .get("id")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| RpcError::bad_request("Missing id"))?
                    as i32;
                state
                    .planets
                    .iter()
                    .find(|p| p.id == id_val)
                    .map(|p| serde_json::to_value(p).unwrap())
                    .ok_or_else(|| {
                        RpcError::not_found(format!("Planet with id {id_val} not found"))
                    })
            })();

            match result {
                Ok(val) => {
                    send_ws(
                        &sender,
                        ServerMessage::Response {
                            id,
                            json: ServerResponseJson {
                                status: None,
                                headers: None,
                                body: Some(RpcBody {
                                    json: val,
                                    meta: vec![],
                                }),
                            },
                        },
                    )
                    .await
                }
                Err(e) => {
                    send_ws(
                        &sender,
                        ServerMessage::Response {
                            id,
                            json: ServerResponseJson {
                                status: Some(if e.code == "NOT_FOUND" { 404 } else { 400 }),
                                headers: None,
                                body: Some(RpcBody {
                                    json: serde_json::json!({
                                        "defined": false,
                                        "inferable": false,
                                        "code": e.code,
                                        "message": e.message,
                                        "data": null
                                    }),
                                    meta: vec![],
                                }),
                            },
                        },
                    )
                    .await
                }
            }
        }

        "/stream-async" | "/streamAsync" => {
            // Send initial response with event-stream header — no body
            let mut headers = HashMap::new();
            headers.insert("standard-server".to_string(), "event-stream".to_string());

            send_ws(
                &sender,
                ServerMessage::Response {
                    id: id.clone(),
                    json: ServerResponseJson {
                        status: None,
                        headers: Some(headers),
                        body: None,
                    },
                },
            )
            .await;

            // Stream events
            for i in 0u32..15 {
                tokio::time::sleep(Duration::from_secs(1)).await;

                let data = serde_json::json!({
                    "json": {
                        "message": format!("WS Async Stream Event #{i}"),
                        "count": i
                    },
                    "meta": []
                });

                send_ws(
                    &sender,
                    ServerMessage::EventStream {
                        id: id.clone(),
                        json: EventStreamJson {
                            event: None, // None = "message"
                            data: Some(data),
                        },
                    },
                )
                .await;
            }

            // Close event
            send_ws(
                &sender,
                ServerMessage::EventStream {
                    id,
                    json: EventStreamJson {
                        event: Some("close".to_string()),
                        data: None,
                    },
                },
            )
            .await;
        }

        _ => {
            send_ws(
                &sender,
                ServerMessage::Response {
                    id,
                    json: ServerResponseJson {
                        status: Some(404),
                        headers: None,
                        body: Some(RpcBody {
                            json: serde_json::json!({
                                "defined": false,
                                "inferable": false,
                                "code": "NOT_FOUND",
                                "message": format!("No procedure at {path}"),
                                "data": null
                            }),
                            meta: vec![],
                        }),
                    },
                },
            )
            .await;
        }
    }
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
        .route("/ws", get(ws_handler))
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
    println!("   - WS   /ws  (oRPC peer protocol)");

    axum::serve(listener, app).await.unwrap();
}
