# Better Auth + orpc Integration Example

Production-ready example demonstrating Axum handlers with `#[orpc]` annotations alongside [Better Auth](https://www.better-auth.com/) session management for authentication and authorization.

## Overview

This example shows how to:
- Annotate plain Axum handlers with `#[orpc]` for automatic routing and contract generation
- Integrate Better Auth RS for authentication with custom extractors
- Implement protected endpoints using `Session` extractor (401 on missing/invalid session)
- Implement optional auth using `OptionalSession` extractor
- Follow Clean Architecture with domain/application/infrastructure layers
- Generate TypeScript contracts with error schemas for type-safe client consumption

## Architecture

```
src/
├── domain/              # Business logic (entities, ports)
│   ├── models/          # Domain entities with #[derive(ZodTs)]
│   │   └── planet.rs    # Planet, CreatePlanetInput, FindPlanetInput, etc.
│   └── ports/           # Repository traits
│       └── planet_repository.rs
├── application/         # Use cases (handlers, errors)
│   ├── handlers/        # #[orpc]-annotated Axum handlers
│   │   ├── ping.rs      # Health check with optional auth
│   │   ├── planet.rs    # CRUD operations (protected + public)
│   │   ├── profile.rs   # User profile (protected)
│   │   └── stream.rs    # SSE streaming examples
│   └── errors.rs        # AppError enum with #[derive(OrpcErrors)]
├── infrastructure/      # External adapters (auth, db, repos)
│   ├── auth/            # Better Auth integration
│   │   ├── extractors.rs    # Session, OptionalSession, SessionExt trait
│   │   └── schema.rs        # Better Auth schema + migrations
│   ├── db/              # Database setup (SeaORM)
│   ├── repositories/    # Repository implementations
│   └── context.rs       # AppState (shared state across handlers)
├── server/              # Server composition
│   └── axum.rs          # Router assembly, CORS, middleware
└── main.rs              # Entry point (auth setup, contract generation, server start)
```

## Features Demonstrated

### 1. Automatic Contract Generation

The `main.rs` generates TypeScript contracts before starting the server (dev mode only):

```rust
#[cfg(debug_assertions)]
{
    orpc::generate_contract()
        .output("../client/src/rpc/bindings.ts")
        .expect("contract generation failed");
}
```

See [`../client/src/rpc/bindings.ts`](../client/src/rpc/bindings.ts) for the generated output.

### 2. Handler Patterns

**Public endpoint (no auth):**
```rust
#[orpc(method = "POST", path = "/planet/list")]
pub async fn list_planets(
    State(state): State<AppState>
) -> Result<Json<Vec<Planet>>, AppError> {
    state.planet_repo.list().await.map(Json)
}
```

**Protected endpoint (requires auth):**
```rust
#[orpc(method = "POST", path = "/planet/create")]
pub async fn create_planet(
    State(state): State<AppState>,
    _session: Session,  // 401 if missing/invalid
    Json(input): Json<CreatePlanetInput>,
) -> Result<Json<Planet>, AppError> {
    state.planet_repo.create(input).await.map(Json)
}
```

**Optional auth:**
```rust
#[orpc(method = "POST", path = "/ping")]
pub async fn ping(
    State(_state): State<AppState>,
    session: OptionalSession,  // None if unauthenticated
) -> Json<String> {
    let msg = match session.0 {
        Some(s) => format!("pong (authenticated as {})", s.user.email()),
        None => "pong (anonymous)".to_string(),
    };
    Json(msg)
}
```

**SSE streaming:**
```rust
#[orpc(method = "GET", path = "/stream", stream_event = "message")]
pub async fn stream_events() -> Sse<impl Stream<Item = Event>> {
    let stream = stream! {
        for i in 0..5 {
            let data = serde_json::to_string(&StreamEvent {
                message: format!("Event {}", i),
                count: i,
            }).unwrap();
            yield Event::default().data(data).event("message");
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::default())
}
```

### 3. Custom Auth Extractors

**`Session` (protected, 401 on failure):**
```rust
#[async_trait]
impl<S> FromRequestParts<S> for Session
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = AppState::from_ref(state);
        let cookies = extract_cookie_jar(parts)?;
        
        let session = state.auth
            .validate_session(&cookies, parts.headers, &parts.uri)
            .await
            .ok_or(StatusCode::UNAUTHORIZED)?;
            
        Ok(Session { session })
    }
}
```

**`OptionalSession` (never fails):**
```rust
pub struct OptionalSession(pub Option<Session>);
// Returns None instead of 401 when unauthenticated
```

**`SessionExt` trait:**
```rust
pub trait SessionExt {
    fn user_id(&self) -> &str;
    fn user_email(&self) -> Option<String>;
}
```

### 4. Error Handling with TypeScript Generation

```rust
#[derive(Debug, OrpcErrors)]
pub enum AppError {
    NotFound,
    BadRequest { reason: String },
    Internal(String),
    Unauthorized,
}
```

Generates TypeScript error schemas in the contract:

```typescript
errors: {
  NOT_FOUND: {},
  BAD_REQUEST: { data: z.object({ reason: z.string() }) },
  INTERNAL: { data: z.string() },
  UNAUTHORIZED: {}
}
```

### 5. Type-Safe Domain Models

```rust
#[derive(Serialize, Deserialize, ZodTs)]
pub struct Planet {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize, Serialize, ZodTs)]
pub struct CreatePlanetInput {
    pub name: String,
    pub description: Option<String>,
}
```

Automatically generates Zod schemas for TypeScript:

```typescript
export const PlanetSchema = z.object({
  id: z.number().int(),
  name: z.string(),
  description: z.string().optional()
});

export type Planet = z.infer<typeof PlanetSchema>;
```

## Running the Example

### Prerequisites

- Rust 1.80+ with Cargo
- Node.js 20+ with npm/pnpm (for the React client)

### Start the Backend

```bash
# From repo root
cargo run -p better-auth-orpc-example
```

The server:
- Starts on `http://localhost:3001`
- Generates TypeScript contract to `../client/src/rpc/bindings.ts` (debug mode)
- Uses in-memory SQLite database with Better Auth schema
- Enables CORS for `http://localhost:3000` (React dev server)

### Start the Frontend

```bash
# From examples/axum-react/client/
pnpm install
pnpm dev
```

The client runs on `http://localhost:3000` with:
- Type-safe RPC calls via `@orpc/client` and TanStack Query
- Auto-generated Zod schemas for validation
- Better Auth client integration

## API Endpoints

All endpoints prefixed with `/rpc` in production (see `server/axum.rs`).

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `POST` | `/ping` | Optional | Health check, returns auth status |
| `POST` | `/profile` | Required | Get current user profile |
| `POST` | `/planet/list` | Public | List all planets |
| `POST` | `/planet/list-paginated` | Public | Paginated planet list |
| `POST` | `/planet/find` | Public | Find planet by ID |
| `POST` | `/planet/create` | Required | Create new planet (protected) |
| `GET` | `/stream` | Public | SSE stream (5 events, 1s interval) |
| `GET` | `/stream-async` | Public | SSE stream (async version) |

Better Auth endpoints are mounted at `/` (no `/rpc` prefix):
- `POST /api/sign-in/email-password`
- `POST /api/sign-up/email-password`
- `POST /api/sign-out`
- `GET /api/session`

## TypeScript Contract Structure

The generated `bindings.ts` contains:

```typescript
// 1. Zod schemas for all domain types
export const PlanetSchema = z.object({ ... });
export type Planet = z.infer<typeof PlanetSchema>;

// 2. Contract with @orpc/contract format
export const contract = {
  ping: oc.meta(openapi({ method: "POST", path: "/ping" }))
    .output(z.string()),
  
  planet: {
    listPlanets: oc.meta(openapi({ method: "POST", path: "/planet/list" }))
      .output(z.array(PlanetSchema))
      .errors({ NOT_FOUND: {}, ... }),
    
    createPlanet: oc.meta(openapi({ method: "POST", path: "/planet/create" }))
      .input(CreatePlanetInputSchema)
      .output(PlanetSchema)
      .errors({ UNAUTHORIZED: {}, ... }),
  },
} as const;

export type Contract = typeof contract;
```

## Client Usage Example

See [`better-auth-contract.ts`](../client/src/rpc/better-auth-contract.ts) for the configured client:

```typescript
import { createORPCClient } from "@orpc/client";
import { OpenAPILink } from "@orpc/openapi/fetch";
import { createTanstackQueryUtils } from "@orpc/tanstack-query";
import { contract } from "./bindings";

const link = new OpenAPILink(contract, {
  origin: "http://localhost:3001",
  url: "/rpc",
  fetch(url, init) {
    return globalThis.fetch(url, {
      ...init,
      credentials: "include",  // Send cookies for auth
    });
  },
});

export const client = createORPCClient(link);
export const orpc = createTanstackQueryUtils(client);
```

**Usage in React components:**

```tsx
import { orpc } from "#/rpc/better-auth-contract";

function PlanetList() {
  const { data, isLoading } = orpc.planet.listPlanets.useQuery();
  
  if (isLoading) return <div>Loading...</div>;
  return <ul>{data?.map(p => <li key={p.id}>{p.name}</li>)}</ul>;
}

function CreatePlanet() {
  const mutation = orpc.planet.createPlanet.useMutation();
  
  const handleSubmit = async (name: string) => {
    await mutation.mutateAsync({ name, description: null });
  };
  
  return <form onSubmit={...}>...</form>;
}
```

## Key Dependencies

**Backend:**
- `orpc` — Macros and contract generation
- `axum` 0.8 — Web framework
- `better-auth` 1.0.0-alpha.2 — Authentication (with `axum` and `seaorm2` features)
- `tower-http` — CORS middleware
- `tokio` — Async runtime
- `serde` — Serialization

**Frontend:**
- `@orpc/client` — Type-safe RPC client
- `@orpc/contract` — Contract definitions
- `@orpc/openapi` — OpenAPI-compatible link
- `@orpc/tanstack-query` — TanStack Query integration
- `@tanstack/react-query` — React data fetching
- `better-auth` — Auth client
- `zod` — Runtime validation

## Design Patterns

1. **Clean Architecture** — Domain logic independent of frameworks
2. **Dependency Injection** — `AppState` passed via Axum's `State` extractor
3. **Repository Pattern** — `PlanetRepository` trait with in-memory implementation
4. **Error as Values** — `Result<T, AppError>` instead of unwrap/expect
5. **Type-Driven Development** — Rust types generate TypeScript schemas automatically

## What Makes This Different

Unlike traditional RPC frameworks:
- ✅ **No code generation step** — Write Rust handlers, get TypeScript contracts automatically
- ✅ **Plain Axum handlers** — Works with existing Axum middleware, extractors, and patterns
- ✅ **Compile-time safety** — Handler signatures validated at compile time
- ✅ **Zero runtime overhead** — Metadata collection via `inventory` crate (link-time only)
- ✅ **Framework agnostic auth** — Better Auth works alongside orpc without special integration
- ✅ **Streaming support** — SSE streams with proper TypeScript async iterator types

## License

MIT OR Apache-2.0
