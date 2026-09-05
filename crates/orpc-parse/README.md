# orpc-parse

[![Documentation](https://docs.rs/orpc-parse/badge.svg)](https://docs.rs/orpc-parse)
[![License](https://img.shields.io/crates/l/orpc-parse.svg)](https://github.com/yourusername/rust-orpc)

AST parsing utilities and code generation for orpc proc macros.

## Overview

This is a regular (non-proc-macro) library containing all the parsing, validation, and code generation logic for the `orpc` ecosystem. By extracting implementation from the proc-macro crate, everything becomes testable with normal `#[test]` functions.

The `orpc-macros` crate is a thin bridge that calls functions from this crate.

## Architecture

```
orpc-macros (proc-macro bridge, lib.rs only)
     └── orpc-parse (this crate — all implementation)
           └── syn 2.0, quote, proc-macro2, inventory
```

### Module Structure

- **`errors`** — Structured `Error` type with span information and compile-time suggestions
- **`types`** — AST-based wrapper extraction (`Json<T>`, `Result<T,E>`, `Option<T>`, etc.)
- **`attributes`** — `#[serde(...)]` and `#[zod(...)]` attribute parsing
- **`functions`** — Handler function signature analysis → `HandlerSignature`
- **`codegen`** — Code generation; each sub-module corresponds to one proc macro:
  - `orpc.rs` — `#[orpc]` attribute macro expansion
  - `router.rs` — `router!` macro expansion
  - `zod_ts.rs` — `#[derive(ZodTs)]` expansion
  - `error_derive.rs` — `#[derive(OrpcErrors)]` expansion

## Features

### AST-Based Type Matching

No fragile string comparisons. All wrapper extraction uses AST path segment identifiers:

```rust
use orpc_parse::types::{try_extract_wrapper, JSON, RESULT, OPTION};

// Works for: Json<T>, axum::Json<T>, axum::extract::Json<T>, etc.
if let Some(wrapper) = try_extract_wrapper(ty, JSON) {
    let inner = wrapper.first_type(); // The T in Json<T>
}

// Works for: Result<T, E>, std::result::Result<T, E>, core::result::Result<T, E>
if let Some(wrapper) = try_extract_wrapper(ty, RESULT) {
    let ok_type = wrapper.first_type();
    let err_type = wrapper.second_type();
}
```

### Structured Error Types

Compile-time diagnostics with span-accurate errors and actionable suggestions:

```rust
use orpc_parse::errors::{Error, ErrorKind};

// Example error with help text
Error::new(
    ErrorKind::MissingWrapper {
        expected: "Json",
        found: "String".to_string(),
        suggestion: "Wrap the return type in Json<T>".to_string(),
    },
    span,
)
```

**Error kinds:**
- `MissingWrapper` — Expected `Json<T>`, found plain type
- `EmptyGenericArgs` — Wrapper has no generic arguments
- `UnsupportedType` — Type not supported in current context
- `InvalidAttrValue` — Attribute value doesn't match expected format
- `MissingRequiredAttr` — Required attribute is missing
- `ConflictingAttrs` — Two attributes can't be used together
- `MissingReturnType` — Function missing return type
- `InvalidHandlerSig` — Handler signature doesn't match expected pattern
- `UnknownKey` — Unknown key in attribute
- `SynError` — Wrapped `syn::Error`

### Handler Signature Analysis

Extracts and validates Axum handler signatures:

```rust
use orpc_parse::functions::{extract_handler_signature, HandlerSignature};

let sig = extract_handler_signature(&func)?;

// HandlerSignature {
//     fn_name: "list_planets",
//     fn_span: Span,
//     state_type: Some(Type::Path("Db")),      // From State<Db>
//     input_type: None,                        // No Json<T> param
//     output_type: Type::Path("Vec<Planet>"),  // Unwrapped from Json<Vec<Planet>>
//     error_type: Some(Type::Path("AppError")),// From Result<_, AppError>
//     is_streaming: false,                     // true for Sse<...>
//     is_async: true,
// }
```

**Supported patterns:**
- Parameters: `State<S>`, `Json<T>`, or both
- Return types: `Json<T>`, `Result<Json<T>, E>`, `Sse<impl Stream>`
- Streaming: `Sse<...>` return sets `is_streaming = true`

### Attribute Parsing

Dual syntax support for `#[zod(...)]` constraints:

```rust
use orpc_parse::attributes::parse_zod_attrs;

// Both forms supported:
#[zod(min_length = 3)]      // key = value
#[zod(min_length(3))]       // key(value)
```

**Supported constraints:**
- **Strings:** `min_length`, `max_length`, `length`, `email`, `url`, `regex`, `starts_with`, `ends_with`, `includes`
- **Numbers:** `min`, `max`, `int`, `positive`, `negative`, `nonnegative`, `nonpositive`, `finite`
- **Arrays:** `min_length`, `max_length`, `length`

### Serde Attribute Parsing

Respects `#[serde(rename = "...")]` and `#[serde(rename_all = "...")]`:

```rust
use orpc_parse::attributes::{parse_serde_attrs, apply_rename_rule};

let attrs = parse_serde_attrs(&field.attrs)?;
if let Some(rename) = attrs.rename {
    // Use explicit rename
} else if let Some(rule) = container_rename_all {
    let renamed = apply_rename_rule(&field_name, rule);
}
```

## Type Extraction Constants

```rust
pub const JSON: &str = "Json";
pub const RESULT: &str = "Result";
pub const OPTION: &str = "Option";
pub const VEC: &str = "Vec";
pub const STATE: &str = "State";
pub const SSE: &str = "Sse";
```

## Testing

64 unit tests covering:
- Wrapper extraction with various path forms
- Error type construction and messages
- Attribute parsing (both syntaxes)
- Handler signature validation
- Edge cases (empty generics, nested types, etc.)

Run tests:

```bash
cargo test -p orpc-parse
```

## Usage

This crate is an implementation detail of `orpc-macros`. You typically don't depend on it directly unless you're building your own macros using the same parsing utilities.

```toml
[dependencies]
orpc-parse = "0.1"
```

## Dependencies

- **syn 2.0** — AST parsing (with `full`, `parsing`, `extra-traits` features)
- **quote** — Code generation
- **proc-macro2** — Token stream manipulation
- **inventory** — Compile-time registration system

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
