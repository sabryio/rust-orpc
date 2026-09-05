# orpc-macros

[![Crates.io](https://img.shields.io/crates/v/orpc-macros.svg)](https://crates.io/crates/orpc-macros)
[![Documentation](https://docs.rs/orpc-macros/badge.svg)](https://docs.rs/orpc-macros)
[![License](https://img.shields.io/crates/l/orpc-macros.svg)](https://github.com/yourusername/rust-orpc)

Procedural macro bridge for [orpc](https://crates.io/crates/orpc) — thin wrappers over [`orpc-parse`](https://crates.io/crates/orpc-parse).

## Overview

This crate contains only proc-macro entry points. All parsing, validation, and code generation logic lives in `orpc-parse` where it can be tested with normal `#[test]` functions.

The entire implementation is a single `lib.rs` file with four proc macros that delegate to `orpc-parse`.

## Macros

### `#[orpc]`

Annotate Axum handlers to register metadata for contract generation and auto-routing. The function remains unchanged — it's still a valid Axum handler.

```rust
use axum::{extract::State, Json};
use orpc::orpc;

#[orpc(method = "POST", path = "/planet/list")]
async fn list_planets(State(db): State<Db>) -> Json<Vec<Planet>> {
    Json(db.list().await)
}
```

**Required attributes:**

- `method` — HTTP method (`"GET"`, `"POST"`, etc.)
- `path` — Route path (e.g. `"/planet/list"`)

**Optional attributes:**

- `stream_event` — SSE event type name for streaming handlers

### `router!`

Auto-discovery macro that builds an Axum `Router` from all `#[orpc]`-annotated handlers using the `inventory` crate.

```rust
use orpc::router;

// All handlers, no state
let app = router!();

// With state
let app = router!(db);

// Module filtering
let app = router!("handlers::planet");
let app = router!(["handlers::planet", "api::v1"]);
let app = router!("handlers::{planet,user}");  // brace expansion
let app = router!("handlers::*");              // wildcard

// Filtering + state (any order)
let app = router!("handlers::planet", db);
let app = router!(db, "handlers::planet");
let app = router!(db, ["handlers::planet"]);
```

### `#[derive(ZodTs)]`

Generate a `fn zod_ts() -> String` method that returns TypeScript Zod schemas. The generated schema is registered via `inventory` for contract generation.

```rust
use orpc::ZodTs;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, ZodTs)]
pub struct Planet {
    pub id: i32,
    #[zod(min_length(1), max_length(100))]
    pub name: String,
    pub description: Option<String>,
}
```

**Supported `#[zod(...)]` attributes:**

- **Strings:** `min_length(n)`, `max_length(n)`, `length(n)`, `email`, `url`, `regex("pattern")`, `starts_with("s")`, `ends_with("s")`, `includes("s")`
- **Numbers:** `min(n)`, `max(n)`, `int`, `positive`, `negative`, `nonnegative`, `nonpositive`, `finite`
- **Arrays:** `min_length(n)`, `max_length(n)`, `length(n)`

### `#[derive(OrpcErrors)]`

Register error enum variants for TypeScript contract generation. Variant names are converted to `SCREAMING_SNAKE_CASE`.

```rust
use orpc::OrpcErrors;

#[derive(OrpcErrors)]
pub enum AppError {
    NotFound,                           // → NOT_FOUND: {}
    Conflict { reason: String },        // → CONFLICT: { data: z.object({...}) }
    DatabaseError(String),              // → DATABASE_ERROR: { data: z.string() }
}
```

## Installation

This crate is typically used via the `orpc` facade crate:

```toml
[dependencies]
orpc = "0.1"
```

Or add it directly (not recommended):

```toml
[dependencies]
orpc-macros = "0.1"
```

## Architecture

```
orpc-macros (proc-macro bridge, lib.rs only)
     └── orpc-parse (all implementation, fully testable)
           └── syn 2.0, quote, proc-macro2, inventory
```

**Why the split?**

- Proc-macro crates can't have normal `#[test]` functions
- All logic in `orpc-parse` can be unit-tested
- `orpc-macros` is just thin `TokenStream` conversion wrappers

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
