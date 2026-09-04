# orpc-axum

Axum integration for `orpc-core` — converts type-safe RPC procedure routers into Axum routers with automatic routing and JSON serialization.

## Features

- 🔒 **Type-safe**: Compile-time guarantees for procedure inputs, outputs, and context
- 🚀 **Zero-config routing**: Automatically generates Axum routes from your router structure
- 🌐 **CORS enabled**: Built-in CORS support for cross-origin requests
- 🎯 **Clean errors**: Structured error responses with status codes
- 📦 **Framework agnostic core**: `orpc-core` remains independent of Axum

## Usage

```rust
use orpc_axum::AxumRouter;
use orpc_core::{os, Procedure, ProcedureRegistry, Router};

#[derive(Clone)]
struct AppContext {
    data: String,
}

struct ApiRouter {
    ping: Procedure<AppContext, (), String>,
}

impl Router<AppContext> for ApiRouter {
    fn register_procedures(&self, prefix: &str, registry: &mut ProcedureRegistry<AppContext>) {
        registry.insert("ping", &self.ping);
    }
}

#[tokio::main]
async fn main() {
    let router = ApiRouter {
        ping: os()
            .context::<AppContext>()
            .output::<String>()
            .handler(|_ctx, _: ()| async { Ok("pong".to_string()) }),
    };

    let ctx = AppContext { data: "test".to_string() };

    // Convert to Axum router
    let app = router.into_axum_router(ctx);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

## How it Works

1. Define your API structure as nested structs with `Procedure` fields
2. Implement the `Router` trait to register procedures with paths
3. Call `.into_axum_router(context)` to generate an Axum router
4. All procedures are automatically registered as POST endpoints with JSON bodies

## Path Mapping

Procedure paths are converted to HTTP routes:

| Procedure Path | HTTP Route           |
| -------------- | -------------------- |
| `ping`         | `POST /ping`         |
| `user/profile` | `POST /user/profile` |
| `api/v1/data`  | `POST /api/v1/data`  |

## Error Handling

`OrpcError` is automatically converted to appropriate HTTP status codes:

- `OrpcError::not_found()` → 404 Not Found
- `OrpcError::bad_request()` → 400 Bad Request
- `OrpcError::internal()` → 500 Internal Server Error
- `OrpcError::custom()` → Status from error or 500

Error responses are JSON:

```json
{
  "code": "NOT_FOUND",
  "message": "Resource not found"
}
```

## Examples

See `examples/basic.rs` for a complete working example.

## License

MIT OR Apache-2.0
