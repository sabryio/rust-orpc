# orpc-macros

[![Crates.io](https://img.shields.io/crates/v/orpc-macros.svg)](https://crates.io/crates/orpc-macros)
[![Documentation](https://docs.rs/orpc-macros/badge.svg)](https://docs.rs/orpc-macros)
[![License](https://img.shields.io/crates/l/orpc-macros.svg)](https://github.com/yourusername/rust-orpc)

Procedural macros for [orpc](https://crates.io/crates/orpc-core) — declarative router syntax inspired by TypeScript oRPC.

## Overview

This crate provides the `r!` macro for defining RPC routers with a clean, nested object syntax that mirrors TypeScript oRPC's plain object pattern. It's syntactic sugar over `RouterBuilder`, eliminating boilerplate while maintaining full type safety.

## Features

- **Declarative syntax** — Define routers like TypeScript objects
- **Zero runtime overhead** — Expands to `RouterBuilder` at compile time
- **Type-safe** — Full compile-time type checking
- **Flexible keys** — Supports both identifiers and string literals
- **Deep nesting** — Arbitrary router hierarchy
- **Optional trailing commas** — Write idiomatic Rust

## Installation

This crate is typically used via `orpc-core`, which re-exports the macro:

```toml
[dependencies]
orpc-core = "0.1"
```

Or add it directly:

```toml
[dependencies]
orpc-macros = "0.1"
```

## Usage

```rust
use orpc_core::{router, os, OrpcError};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
struct AppContext {
    db: Database,
}

#[derive(Deserialize)]
struct FindInput {
    id: i32,
}

#[derive(Serialize)]
struct Planet {
    id: i32,
    name: String,
}

let router = router! {
    ping: os()
        .context::<AppContext>()
        .output::<String>()
        .handler(|_ctx, _: ()| async { Ok("pong".to_string()) }),

    planet: {
        list: os()
            .context::<AppContext>()
            .output::<Vec<Planet>>()
            .handler(|ctx, _: ()| async move {
                Ok(ctx.db.list_planets().await)
            }),

        find: os()
            .context::<AppContext>()
            .input::<FindInput>()
            .output::<Planet>()
            .handler(|ctx, input| async move {
                ctx.db.find_planet(input.id).await
                    .ok_or_else(|| OrpcError::not_found("Planet not found"))
            })
    }
};
```

### String Literal Keys

Use string literals for keys with special characters:

```rust
router! {
    ping: os()...,
    "list-paginated": os()...,  // kebab-case
    "users:create": os()...,    // colons
}
```

### Comparison with Manual Builder

The macro is syntactic sugar. These are equivalent:

**Macro:**

```rust
router! {
    ping: os()...,
    planet: { list: os()... }
}
```

**Manual:**

```rust
r()
    .add("ping", os()...)
    .nest("planet", r().add("list", os()...))
```

## Architecture

This crate follows Clean Architecture principles:

- **Domain (`ast.rs`)** — Pure AST types (RouterKey, RouterItem, RouterMacroInput)
- **Ports (`parse.rs`)** — Parser adapters implementing syn's Parse trait
- **Adapters (`generate.rs`)** — Code generators producing TokenStreams
- **Composition (`lib.rs`)** — Entry point wiring parser → generator

Benefits:

- **Testability** — Domain types can be unit-tested without macro machinery
- **Maintainability** — Parser and generator are isolated and independently changeable
- **Extensibility** — Future features (context inference, validation) can be added to domain layer

## Documentation

For full API documentation and examples, see:

- [docs.rs/orpc-macros](https://docs.rs/orpc-macros) — This crate
- [docs.rs/orpc-core](https://docs.rs/orpc-core) — Core abstractions
- [docs.rs/orpc-axum](https://docs.rs/orpc-axum) — Axum integration

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
