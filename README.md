# rorpc

Rust oRPC — annotate plain Axum handlers with `#[orpc]` and get automatic TypeScript contract generation, type-safe Zod schemas, and zero-boilerplate routing.

```rust
use axum::{extract::State, Json};
use rorpc::{orpc, ZodTs};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, ZodTs)]
pub struct Planet {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
}

#[orpc(method = "GET", path = "/planet/list")]
pub async fn list_planets(State(db): State<AppState>) -> Result<Json<Vec<Planet>>, AppError> {
    db.planet_repo.list().await.map(Json).map_err(AppError::from)
}
```

The macro leaves the handler unchanged. At startup, call `generate_contract()` once and the TypeScript side is always in sync:

```rust
rorpc::generate_contract()
    .output(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../client/src/rpc/bindings.ts"
    ))
    .expect("contract generation failed");
```

```typescript
// Auto-generated — do not edit
export const contract = {
  planet: {
    listPlanets: oc
      .meta(openapi({ method: "GET", path: "/planet/list" }))
      .output(z.array(PlanetSchema))
      .errors({ NOT_FOUND: {}, INTERNAL: { data: z.string() } }),
  },
} as const;
```

## How It Works

1. `#[orpc]` registers `HandlerMetadata` and `HandlerRegistration` at link time via the [`inventory`](https://crates.io/crates/inventory) crate — no central list, no startup registry call.
2. `router!(state)` collects all registered handlers and builds an Axum `Router` automatically.
3. `generate_contract()` iterates the same registrations, collects Zod schemas from `#[derive(ZodTs)]` types, and writes a TypeScript file.

Zero runtime overhead — all discovery happens at link time.

## Workspace

```
crates/
├── rorpc           # Facade — macros re-export, generate_contract(), runtime types
├── rorpc-macros    # Proc-macro bridge (thin lib.rs only)
└── rorpc-parse     # All AST parsing + codegen, testable with normal #[test]

examples/axum-react/
├── better-auth-integration/  # Full-stack example: Axum + Better Auth + rorpc
│   └── src/
│       ├── application/handlers/   # #[orpc]-annotated handlers
│       ├── domain/models/          # #[derive(ZodTs)] types
│       ├── infrastructure/auth/    # Session, OptionalSession extractors
│       └── main.rs                 # Runs generate_contract() then starts server
└── client/                   # React + TanStack Query + @orpc/client
    └── src/rpc/
        ├── bindings.ts             # Auto-generated — do not edit
        └── better-auth-contract.ts # Hand-written client config
```

## Dependency Graph

```
rorpc (runtime)
  └── rorpc-macros (proc-macro bridge)
        └── rorpc-parse (all parsing + codegen)
              └── syn 3.0, quote, proc-macro2, inventory
```

`rorpc-parse` has no dependency on `rorpc` or `rorpc-macros`. It can be tested independently.

## Macros

### `#[orpc(method, path)]`

Annotates any valid Axum handler. The function is emitted unchanged plus two `inventory::submit!` calls — one for contract generation, one for `router!`.

```rust
use rorpc::orpc;

// GET — no input, returns list
#[orpc(method = "GET", path = "/planet/list")]
pub async fn list_planets(State(s): State<AppState>) -> Result<Json<Vec<Planet>>, AppError>

// GET — with query parameters (Query<T> extractor)
#[orpc(method = "GET", path = "/planet/find")]
pub async fn find_planet(
    State(s): State<AppState>,
    Query(params): Query<FindPlanetInput>,
) -> Result<Json<Planet>, AppError>

// POST — with JSON body (Json<T> extractor), protected endpoint
#[orpc(method = "POST", path = "/planet/create")]
pub async fn create_planet(
    State(s): State<AppState>,
    _session: Session,
    Json(input): Json<CreatePlanetInput>,
) -> Result<Json<Planet>, AppError>

// DELETE — with JSON body (Json<T> extractor), protected endpoint
#[orpc(method = "DELETE", path = "/planet/delete")]
pub async fn delete_planet(
    State(s): State<AppState>,
    _session: Session,
    Json(input): Json<DeletePlanetInput>,
) -> Result<Json<()>, AppError>

// SSE streaming — data takes a type path (no quotes)
#[orpc(method = "GET", path = "/stream", data = StreamEvent)]
pub async fn stream_events() -> Sse<impl Stream<Item = Event>>
```

**Supported HTTP methods:** GET, POST, PUT, PATCH, DELETE

**Request body vs query parameters:**

- **POST/PUT/PATCH/DELETE:** Use `Json<T>` extractor → data sent as JSON request body
- **GET:** Use `Query<T>` extractor → data sent as URL query parameters
- Both render as `.input()` in the TypeScript contract

**Important:** The oRPC client automatically handles the difference:

- For GET: sends input as query string parameters
- For POST/PUT/PATCH: sends input as JSON request body

### `router!`

Builds an Axum `Router` from all discovered handlers.

```rust
use rorpc::router;

let app = router!(state);                          // all handlers + state
let app = router!("handlers::planet", state);      // module filter
let app = router!("handlers::{planet,user}");      // brace expansion
let app = router!("handlers::*");                  // wildcard
```

### `#[derive(ZodTs)]`

Generates `fn zod_ts() -> String` and registers the schema for contract generation.

```rust
use rorpc::ZodTs;

#[derive(Serialize, Deserialize, ZodTs)]
pub struct Planet {
    pub id: i32,
    #[zod(min_length(1), max_length(100))]
    pub name: String,
    pub description: Option<String>,
}
```

Supported `#[zod(...)]` constraints: `min_length`, `max_length`, `length`, `email`, `url`, `regex`, `min`, `max`, `int`, `positive`, `negative`, `nonnegative`, `nonpositive`, `finite`.

### `#[derive(OrpcError)]`

Registers error enum variants for TypeScript `.errors({...})` generation.

```rust
use rorpc::OrpcError;

#[derive(OrpcError)]
pub enum AppError {
    NotFound,                    // → NOT_FOUND: {}
    BadRequest { reason: String },  // → BAD_REQUEST: { data: z.object({ reason: z.string() }) }
    Internal(String),            // → INTERNAL: { data: z.string() }
    Unauthorized,                // → UNAUTHORIZED: {}
}
```

## TypeScript Client

The example client uses [`@orpc/client`](https://www.npmjs.com/package/@orpc/client) with TanStack Query:

```typescript
// examples/axum-react/client/src/rpc/better-auth-contract.ts
import { createORPCClient, isInferableError, ORPCError } from "@orpc/client";
import { type RouterContractClient } from "@orpc/contract";
import { OpenAPILink } from "@orpc/openapi/fetch";
import { createTanstackQueryUtils } from "@orpc/tanstack-query";
import { contract } from "./bindings";
export { consumeAsyncIterator, getEventMeta } from "@orpc/client";

const link = new OpenAPILink(contract, {
  origin: "http://localhost:3001",
  url: "/rpc",
  fetch: (url, init) => fetch(url, { ...init, credentials: "include" }),
});

export const client: RouterContractClient<typeof contract> =
  createORPCClient(link);

export const orpc = createTanstackQueryUtils(client);

export { isInferableError, ORPCError };
```

Usage in React:

```tsx
import { orpc } from "@/rpc/better-auth-contract";
import { useQuery, useMutation, useInfiniteQuery } from "@tanstack/react-query";

// Query
const { data } = useQuery(orpc.planet.listPlanets.queryOptions());

// Mutation with cache invalidation
const mutation = useMutation(
  orpc.planet.createPlanet.mutationOptions({
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: orpc.planet.key() }),
  }),
);

// Delete mutation (also requires authentication)
const deleteMutation = useMutation(
  orpc.planet.deletePlanet.mutationOptions({
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: orpc.planet.key() }),
  }),
);
deleteMutation.mutate({ id: 1 });

// Infinite scroll
const { data, fetchNextPage } = useInfiniteQuery(
  orpc.planet.listPlanetsPaginated.infiniteOptions({
    input: (pageParam: number | undefined) => ({
      limit: 10,
      offset: pageParam ?? 0,
    }),
    initialPageParam: undefined,
    getNextPageParam: (lastPage) => lastPage.next_page_param,
  }),
);

// Direct call (outside React)
const planet = await orpc.planet.findPlanet.call({ id: 1 });
```

## Running the Example

**Backend** (port 3001, generates `bindings.ts` on startup):

```bash
cargo run -p better-auth-rorpc-example
```

**Frontend** (port 3000):

```bash
cd examples/axum-react/client
pnpm install
pnpm dev
```

## Testing

```bash
# All tests including rorpc-parse unit tests (67+)
cargo test --workspace

# rorpc-parse only (fast, no proc-macro overhead)
cargo test -p rorpc-parse
```

## Dependencies

| Crate          | Key dependencies                                                                 |
| -------------- | -------------------------------------------------------------------------------- |
| `rorpc`        | `axum 0.8`, `inventory 0.3`, `serde 1.0`                                         |
| `rorpc-macros` | `syn 3.0`, `proc-macro2 1.0`                                                     |
| `rorpc-parse`  | `syn 3.0` (full + extra-traits), `quote 1.0`, `proc-macro2 1.0`, `inventory 0.3` |

## License

MIT OR Apache-2.0
