# rorpc-parse

[![Documentation](https://docs.rs/rorpc-parse/badge.svg)](https://docs.rs/rorpc-parse)
[![License](https://img.shields.io/crates/l/rorpc-parse.svg)](https://github.com/yourusername/rorpc)

AST parsing utilities and code generation for rorpc proc macros.

## Overview

This is a regular (non-proc-macro) library containing all the parsing, validation, and code generation logic for the `rorpc` ecosystem. By extracting implementation from the proc-macro crate, everything becomes testable with normal `#[test]` functions.

The `rorpc-macros` crate is a thin bridge that calls functions from this crate.

## Architecture

```
rorpc-macros (proc-macro bridge, lib.rs only)
     └── rorpc-parse (this crate — all implementation)
           └── syn 2.0, quote, proc-macro2, inventory
```

### Module Structure

- **`errors`** — Structured `Error` type with span information and compile-time suggestions
- **`types`** — AST-based wrapper extraction (`Json<T>`, `Result<T,E>`, `Option<T>`, etc.)
- **`attributes`** — `#[serde(...)]` and `#[zod(...)]` attribute parsing
- **`functions`** — Handler function signature analysis → `HandlerSignature`
- **`codegen`** — Code generation; each sub-module corresponds to one proc macro:
  - `orpc.rs` — `#[rorpc]` attribute macro expansion
  - `router.rs` — `router!` macro expansion
  - `zod_ts.rs` — `#[derive(ZodTs)]` expansion
  - `error_derive.rs` — `#[derive(OrpcErrors)]` expansion

## Features

### AST-Based Type Matching

No fragile string comparisons. All wrapper extraction uses AST path segment identifiers:

```rust
use rorpc_parse::types::{try_extract_wrapper, JSON, RESULT, OPTION};

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
use rorpc_parse::errors::{Error, ErrorKind};

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
use rorpc_parse::functions::{extract_handler_signature, HandlerSignature};

let sig = extract_handler_signature(&func)?;

// HandlerSignature {
//     fn_name: "list_planets",
//     state_type: Some(Type::Path("Db")),
//     input_type: None,
//     output_type: Type::Path("Vec<Planet>"),
//     error_type: Some(Type::Path("AppError")),
//     is_streaming: false,
//     is_async: true,
// }
```

### Attribute Parsing

Dual syntax support for `#[zod(...)]` constraints:

```rust
#[zod(min_length = 3)]   // key = value
#[zod(min_length(3))]    // key(value)
```

### Runtime Type Conversion Utilities

String-based type-to-Zod conversion for runtime contract generation (shared with the `rorpc` crate):

```rust
use rorpc_parse::codegen::{rust_type_to_ts_schema, to_schema_name, base_type_name};
use rorpc_parse::types::{is_primitive_type_name, extract_first_generic_arg_string};

assert_eq!(rust_type_to_ts_schema("Json<Planet>"), "PlanetSchema");
assert_eq!(rust_type_to_ts_schema("Json<Vec<Planet>>"), "z.array(PlanetSchema)");
assert!(is_primitive_type_name("String"));
assert!(!is_primitive_type_name("Planet"));
```

## Testing

64+ unit tests covering:

- Wrapper extraction with various path forms
- Error type construction and messages
- Attribute parsing (both syntaxes)
- Handler signature validation
- Runtime type-to-Zod conversion
- Edge cases (empty generics, nested types, etc.)

```bash
cargo test -p rorpc-parse
```

## Dependencies

- **syn 2.0** — AST parsing (with `full`, `parsing`, `extra-traits` features)
- **quote** — Code generation
- **proc-macro2** — Token stream manipulation
- **inventory** — Compile-time registration system

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE))
- MIT license ([LICENSE-MIT](../../LICENSE-MIT))

at your option.
