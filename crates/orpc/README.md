# orpc

[![Crates.io](https://img.shields.io/crates/v/orpc.svg)](https://crates.io/crates/orpc)
[![Documentation](https://docs.rs/orpc/badge.svg)](https://docs.rs/orpc)
[![License](https://img.shields.io/crates/l/orpc.svg)](https://github.com/yourusername/rust-orpc)

Unified facade for handler metadata collection, auto-router construction, and TypeScript contract generation for Axum.

## Overview

`orpc` bridges Rust Axum handlers with TypeScript clients through compile-time metadata collection and automatic code generation. Annotate handlers with `#[orpc]`, derive Zod schemas, and generate type-safe TypeScript contracts with zero runtime overhead.

## Features

- **Zero boilerplate routing** — Auto-discover handlers with `router!()`
- **Type-safe contracts** — Generate TypeScript Zod schemas from Rust types
- **Compile-time registration** — Uses `inventory` crate for link-time collection
- **Streaming support** — SSE endpoints with `Sse<impl Stream>`
- **Error schemas** — TypeScript error enums from Rust error types
- **Module filtering** — Include/exclude handlers by module path

## Quick Start

```rust
use axum::{extract::State, Json};
use orpc::{orpc, router, ZodTs};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
struct AppState {
    db: Database,
}

#[derive(Serialize, Deserialize, ZodTs)]
pub struct Planet {
    pub id: i32,
    #[zod(min_length(1), max_length(100))]
    pub name: String,
    pub description: Option<String>,
}

// Annotate handlers
#[orpc(method = "POST", path = "/planet/list")]
async fn list_planets(State(db): State<Database>) -> Json<Vec<Planet>> {
    Json(db.list().await)
}

#[orpc(method = "POST", path = "/planet/find")]
async fn find_planet(
    State(db): State<Database>,
    Json(id): Json<i32>,
) -> Result<Json<Planet>, AppError> {
    db.find(id)
        .await
        .map(Json)
        .ok_or(AppError::NotFound)
}

#[tokio::main]
async fn main() {
    let state = AppState { db: Database::connect().await };

    // Auto-discover all handlers
    let app = router!(state);

    // Generate TypeScript contract
    orpc::generate_contract()
        .output("../client/src/rpc/index.ts")
        .unwrap();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

## Macros

### `#[orpc]`

Annotate Axum handlers to register metadata. The function remains a valid Axum handler.

**Required attributes:**
- `method` — HTTP method (`"GET"`, `"POST"`, etc.)
- `path` — Route path (e.g. `"/planet/list"`)

**Optional attributes:**
- `stream_event` — SSE event type name for streaming handlers

**Supported handler signatures:**
```rust
// State only
#[orpc(method = "GET", path = "/ping")]
async fn ping(State(s): State<AppState>) -> Json<String>

// Input only  
#[orpc(method = "POST", path = "/echo")]
async fn echo(Json(input): Json<String>) -> Json<String>

// State + Input
#[orpc(method = "POST", path = "/create")]
async fn create(
    State(db): State<Db>,
    Json(data): Json<CreateInput>,
) -> Json<Planet>

// With Result for errors
#[orpc(method = "POST", path = "/find")]
async fn find(
    State(db): State<Db>,
    Json(id): Json<i32>,
) -> Result<Json<Planet>, AppError>

// Streaming with SSE
#[orpc(method = "GET", path = "/events", stream_event = "message")]
async fn stream() -> Sse<impl Stream<Item = Event>>
```

### `router!`

Auto-discover and build an Axum `Router` from all `#[orpc]`-annotated handlers.

```rust
// All handlers, no state
let app = router!();

// With state
let app = router!(state);

// Module filtering (exact match or children)
let app = router!("handlers::planet");

// Multiple patterns
let app = router!(["handlers::planet", "api::v1"]);

// Brace expansion
let app = router!("handlers::{planet,user}");

// Wildcard (all children)
let app = router!("handlers::*");

// Filter + state (any order)
let app = router!("handlers::planet", state);
let app = router!(state, "handlers::planet");
```

### `#[derive(ZodTs)]`

Generate TypeScript Zod schemas from Rust types.

```rust
use orpc::ZodTs;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, ZodTs)]
pub struct User {
    pub id: i32,
    #[zod(email)]
    pub email: String,
    #[zod(min_length(8), max_length(100))]
    pub password: String,
    #[zod(min(13), max(120))]
    pub age: Option<i32>,
}
```

**Supported `#[zod(...)]` attributes:**

- **Strings:** `min_length(n)`, `max_length(n)`, `length(n)`, `email`, `url`, `regex("pattern")`, `starts_with("s")`, `ends_with("s")`, `includes("s")`
- **Numbers:** `min(n)`, `max(n)`, `int`, `positive`, `negative`, `nonnegative`, `nonpositive`, `finite`
- **Arrays (`Vec<T>`):** `min_length(n)`, `max_length(n)`, `length(n)`

### `#[derive(OrpcErrors)]`

Generate TypeScript error schemas from Rust error enums.

```rust
use orpc::OrpcErrors;

#[derive(OrpcErrors)]
pub enum AppError {
    NotFound,                    // → NOT_FOUND: {}
    Unauthorized,                // → UNAUTHORIZED: {}
    Conflict { reason: String }, // → CONFLICT: { data: z.object({ reason: z.string() }) }
    DatabaseError(String),       // → DATABASE_ERROR: { data: z.string() }
}
```

## Contract Generation

Generate TypeScript contracts with `generate_contract()`:

```rust
orpc::generate_contract()
    .output("../client/src/rpc/index.ts")
    .unwrap();
```

**Generated file structure:**

```typescript
import { z } from "zod";

// Type schemas
export const Planet = z.object({
  id: z.number(),
  name: z.string().min(1).max(100),
  description: z.string().optional(),
});
export type Planet = z.infer<typeof Planet>;

// RPC contract
export const contract = {
  planet: {
    list: {
      method: "POST",
      path: "/planet/list",
      input: z.void(),
      output: z.array(Planet),
      errors: {},
    },
    find: {
      method: "POST",
      path: "/planet/find",
      input: z.number(),
      output: Planet,
      errors: {
        NOT_FOUND: {},
        DATABASE_ERROR: { data: z.string() },
      },
    },
  },
};
```

Use with TypeScript RPC clients like [TanStack Query](https://tanstack.com/query) or custom fetch wrappers.

## Architecture

```
orpc (runtime + facade)
  ├── orpc-macros (proc-macro bridge)
  │     └── orpc-parse (parsing + codegen)
  └── dependencies:
        ├── inventory (compile-time registration)
        ├── axum (web framework)
        └── serde (serialization)
```

### Module Structure

- **`metadata`** — `HandlerMetadata` collected by `#[orpc]` for contract generation
- **`registration`** — `HandlerRegistration` with type-erased route factories for `router!()`
- **`schema_registry`** — `SchemaRegistration` for Zod schemas from `#[derive(ZodTs)]`
- **`error_registry`** — `ErrorRegistration` for error enums from `#[derive(OrpcErrors)]`
- **`codegen`** — TypeScript contract generation:
  - `contract.rs` — Main contract builder
  - `typescript.rs` — Schema and import generation
  - `mod.rs` — `ContractBuilder` API

## How It Works

### Compile-Time Registration

The `inventory` crate collects metadata at link time:

1. **`#[orpc]`** emits two `inventory::submit!` calls per handler:
   - `HandlerMetadata` — for contract generation
   - `HandlerRegistration` — for runtime routing

2. **`#[derive(ZodTs)]`** emits:
   - `SchemaRegistration` with a `fn zod_ts() -> String` factory

3. **`#[derive(OrpcErrors)]`** emits:
   - `ErrorRegistration` with variant mappings

### Runtime Discovery

```rust
// router!() expands to:
inventory::iter::<HandlerRegistration>
    .into_iter()
    .fold(Router::new(), |router, reg| {
        router.merge((reg.factory)(state))
    })
```

### Contract Generation

```rust
// generate_contract() expands to:
let handlers = inventory::iter::<HandlerMetadata>.into_iter().collect();
let schemas = inventory::iter::<SchemaRegistration>.into_iter().collect();
let errors = inventory::iter::<ErrorRegistration>.into_iter().collect();

// Topological sort schemas by dependencies
// Generate TypeScript with Zod schemas + contract
```

## Installation

```toml
[dependencies]
orpc = "0.1"
axum = "0.8"
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

## Comparison with Alternatives

| Feature | orpc | tRPC | gRPC | GraphQL |
|---------|------|------|------|---------|
| Type safety | ✅ Compile-time | ✅ Runtime | ✅ Compile-time | ✅ Runtime |
| Codegen direction | Rust → TS | TS → TS | Proto → Both | Schema → Both |
| HTTP/JSON native | ✅ | ✅ | ❌ (Protobuf) | ✅ |
| Zero runtime overhead | ✅ | ❌ (Runtime reflection) | ❌ (Serialization) | ❌ (Resolvers) |
| Streaming | ✅ SSE | ✅ | ✅ Bidirectional | ✅ Subscriptions |
| Rust-first | ✅ | ❌ | ⚠️ | ⚠️ |

## Examples

See `examples/` directory:
- `examples/axum-react/server` — Basic example
- `examples/axum-react/better-auth-integration` — Auth integration

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
