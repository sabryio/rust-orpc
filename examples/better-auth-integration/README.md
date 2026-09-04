# Better Auth + oRPC Integration Example

This example demonstrates how to integrate [Better Auth RS](https://better-auth.rs) authentication with oRPC using Axum middleware.

## Features

- **Context-based authentication**: Session data extracted via middleware and stored in oRPC context
- **Better Auth integration**: Full authentication system with email/password, OAuth, sessions
- **SeaORM database**: SQLite/PostgreSQL support with auto-generated entities
- **Type-safe**: End-to-end type safety from database to API

## Architecture

```
Request → Axum Middleware (extracts session) → Extensions → oRPC Context → Handlers
```

1. Better Auth middleware extracts session from request
2. Session stored in `req.extensions()`
3. oRPC context reads from extensions
4. Handlers access `OptionalSession<AppAuthSchema>` from context

## Setup

### Prerequisites

**Windows Users**: This project uses Better Auth RS which includes WebAuthn/passkey support via `openssl`. You need to configure OpenSSL:

#### Option 1: Install OpenSSL (Recommended)

1. Download and install OpenSSL for Windows:
   - Using winget: `winget install OpenSSL.OpenSSL`
   - Or download from: https://slproweb.com/products/Win32OpenSSL.html

2. Set environment variables (restart terminal after):

   ```powershell
   setx OPENSSL_DIR "C:\Program Files\OpenSSL-Win64"
   setx OPENSSL_LIB_DIR "C:\Program Files\OpenSSL-Win64\lib\VC\x64\MD"
   setx OPENSSL_INCLUDE_DIR "C:\Program Files\OpenSSL-Win64\include"
   ```

3. **Restart your terminal** for the environment variables to take effect

#### Option 1b: Use Vendored OpenSSL (Compile from Source)

If you prefer to compile OpenSSL from source (useful for CI/CD or isolated builds), you'll need Perl:

1. Install Perl:

   ```powershell
   winget install StrawberryPerl.StrawberryPerl
   ```

   Or download from: https://strawberryperl.com/

2. Add to `Cargo.toml`:

   ```toml
   openssl = { version = "0.10", features = ["vendored"] }
   ```

3. Build (this compiles OpenSSL from source, takes 5-10 minutes):
   ```bash
   cargo build
   ```

**Note**: The `vendored` feature compiles OpenSSL statically into your binary, making it fully portable but increasing build time significantly.

#### Option 2: Use SQLite Only (No OpenSSL)

If you don't need PostgreSQL or WebAuthn/passkey support, you can remove the OpenSSL dependency:

```toml
# In Cargo.toml, override better-auth to disable default features
better-auth = { version = "1.0.0-alpha.2", default-features = false, features = ["axum", "seaorm2"] }

# Add explicit overrides to use SQLite only
sea-orm = { version = "2.0.0-rc.37", default-features = false, features = [
    "sqlx-sqlite",
    "runtime-tokio",
    "macros",
    "with-chrono",
    "with-json",
    "with-uuid",
] }

sqlx = { version = "0.9", default-features = false, features = ["runtime-tokio", "sqlite"] }
```

**Note**: This disables WebAuthn/passkey authentication features.

### Database Setup

1. Create a SQLite database (or PostgreSQL):

   ```bash
   # SQLite (default)
   touch auth.db
   ```

2. Set database URL:

   ```bash
   export DATABASE_URL="sqlite:auth.db"
   # Or for PostgreSQL:
   # export DATABASE_URL="postgres://user:pass@localhost/auth_db"
   ```

3. Run migrations:
   ```bash
   cargo run --bin migrate
   ```

## Running

```bash
# Build
cargo build

# Run
cargo run
```

The server will start at `http://localhost:3000`

## API Endpoints

### Better Auth Routes (Standard REST)

- `POST /api/auth/sign-in/email` - Sign in with email/password
- `POST /api/auth/sign-up/email` - Create new account
- `GET /api/auth/session` - Get current session
- `POST /api/auth/sign-out` - Sign out

### oRPC Routes (Type-safe RPC)

- `POST /rpc/user.profile` - Get user profile (requires auth)
- `POST /rpc/user.update` - Update user profile (requires auth)
- `POST /rpc/admin.listUsers` - List all users (requires admin role)

## Code Structure

```
src/
├── main.rs           # Server setup, middleware, router composition
├── auth_schema.rs    # SeaORM entities with AuthSchema trait
└── handlers.rs       # oRPC handlers with context-based auth
```

## Key Patterns

### Middleware Pattern

```rust
// Extract session in middleware
let auth = BetterAuth::new(auth_config).await?;

let middleware_layer = from_fn_with_state(
    auth.clone(),
    |State(auth): State<Arc<BetterAuth<AppAuthSchema>>>,
     mut req: Request,
     next: Next| async move {
        let session = auth.get_session(&req).await.ok();
        req.extensions_mut().insert(session);
        Ok::<_, Infallible>(next.run(req).await)
    }
);
```

### Context Extraction

```rust
#[derive(Clone)]
pub struct AppContext {
    pub db: Arc<DatabaseConnection>,
}

impl AppContext {
    pub fn session(&self, req: &axum::http::Request<Body>)
        -> OptionalSession<AppAuthSchema>
    {
        req.extensions()
            .get::<OptionalSession<AppAuthSchema>>()
            .cloned()
            .unwrap_or_default()
    }
}
```

### Protected Handler

```rust
let get_profile = os()
    .context::<AppContext>()
    .route(HttpMethod::Post, "/user/profile")
    .handler(|ctx, _: ()| async move {
        let session = ctx.session();

        let user = session.require_user()?; // Returns OrpcError::UNAUTHORIZED if not signed in

        Ok(user)
    });
```

## Error Handling

The example uses oRPC's built-in error codes:

- `UNAUTHORIZED` (401) - No session or invalid token
- `FORBIDDEN` (403) - Insufficient permissions
- `NOT_FOUND` (404) - Resource not found
- `INTERNAL_ERROR` (500) - Server error

## Troubleshooting

### OpenSSL Errors on Windows

If you see `openssl-sys` build errors:

**Error: "Could not find directory of OpenSSL installation"**

1. Verify OpenSSL is installed: `openssl version`
2. Check environment variables: `echo $env:OPENSSL_DIR`
3. Restart your terminal after setting variables
4. Try cleaning and rebuilding: `cargo clean && cargo build`

**Error: "OpenSSL libdir does not contain the required files"**

- The libraries are in a subdirectory. Set `OPENSSL_LIB_DIR` as shown in Option 1 above.

**Error: "Command 'perl' not found" (when using `vendored` feature)**

- Install Strawberry Perl: `winget install StrawberryPerl.StrawberryPerl`
- Restart your terminal
- Rebuild: `cargo clean && cargo build`

### Session Not Persisting

- Check that database migrations ran successfully
- Verify `DATABASE_URL` is set correctly
- Ensure cookies are enabled in your client

### CORS Errors

The example uses permissive CORS for development:

```rust
let cors = CorsLayer::new()
    .allow_origin(Any)
    .allow_methods(Any)
    .allow_headers(Any);
```

For production, configure strict CORS rules.

## Production Considerations

1. **Database**: Use PostgreSQL instead of SQLite
2. **Secrets**: Use environment variables for `BETTER_AUTH_SECRET`
3. **CORS**: Restrict origins to your frontend domain
4. **HTTPS**: Deploy behind a reverse proxy with TLS
5. **Sessions**: Configure session expiration and refresh tokens
6. **Rate Limiting**: Add rate limiting middleware

## Resources

- [Better Auth RS Documentation](https://better-auth.rs)
- [oRPC Documentation](https://github.com/yourusername/orpc)
- [Axum Documentation](https://docs.rs/axum)
- [SeaORM Documentation](https://www.sea-ql.org/SeaORM/)
