# Custom Link Implementation Guide

A comprehensive guide to implementing custom transport layers (Links) in oRPC, including TauriLink for Tauri desktop applications.

> **Sources**: This document synthesizes information from [orpc.dev documentation](https://orpc.dev/docs), [orpc-rs GitHub](https://github.com/ahonn/orpc-rs), and oRPC adapter implementations. Content has been rephrased for compliance with licensing restrictions.

---

## Table of Contents

1. [Understanding Links](#understanding-links)
2. [Link Interface Contract](#link-interface-contract)
3. [Built-in Link Examples](#built-in-link-examples)
4. [Implementing a Custom Link](#implementing-a-custom-link)
5. [TauriLink Implementation](#taurilink-implementation)
6. [Testing Your Link](#testing-your-link)
7. [Real-World Examples](#real-world-examples)

---

## Understanding Links

### What is a Link?

A **Link** in oRPC is the transport layer that handles communication between the client and server. It defines:

- **How** requests are sent (HTTP, WebSocket, IPC, MessagePort)
- **How** responses are received
- **How** errors are handled
- **How** streams/subscriptions work

```typescript
// Link is the bridge between client API calls and server responses
const link = new SomeLink(contract, options);
const client = createORPCClient(link);

// When you call client.someMethod(input)
// The link handles serialization, transport, and deserialization
const result = await client.someMethod(input);
```

### Built-in Link Types

oRPC provides several built-in links:

| Link | Transport | Use Case |
|------|-----------|----------|
| `RPCLink` | HTTP/Fetch | Standard web apps, SSR |
| `OpenAPILink` | HTTP/Fetch | REST APIs, OpenAPI servers |
| `RPCLink` (MessagePort) | MessagePort | Browser extensions, window-to-window |
| `RPCLink` (WebSocket) | WebSocket | Real-time apps, bi-directional |
| `TauriLink` (custom) | Tauri IPC | Desktop apps (Electron/Tauri) |

---

## Link Interface Contract

While oRPC doesn't export a formal `Link` interface, custom links must follow this conceptual contract:

### Core Interface

```typescript
interface CustomLink<TContext = unknown> {
  /**
   * Execute a remote procedure call
   * @param path - Array of route segments (e.g., ['user', 'create'])
   * @param input - Serialized input data
   * @param options - Call options including context
   * @returns Promise resolving to the output or throwing ORPCError
   */
  call<TInput, TOutput>(
    path: string[],
    input: TInput,
    options?: { context?: TContext; signal?: AbortSignal }
  ): Promise<TOutput>;

  /**
   * Subscribe to a streaming procedure (optional, for streaming support)
   * @param path - Array of route segments
   * @param input - Serialized input data
   * @param options - Call options including context
   * @returns AsyncIterator for streaming data
   */
  subscribe?<TInput, TOutput>(
    path: string[],
    input: TInput,
    options?: { context?: TContext; signal?: AbortSignal }
  ): AsyncIterator<TOutput>;
}
```

### Key Requirements

1. **Path Handling**: Links receive procedure paths as string arrays
   - Example: `client.user.create` → `['user', 'create']`
   
2. **Serialization**: Links must handle input/output serialization
   - Most use JSON by default
   - Custom links can use binary protocols (protobuf, MessagePack)

3. **Error Handling**: Links must throw `ORPCError` for errors
   ```typescript
   import { ORPCError } from '@orpc/client';
   
   throw new ORPCError('NOT_FOUND', { 
     message: 'User not found' 
   });
   ```

4. **Context**: Links receive per-call context (auth tokens, headers, etc.)

5. **AbortSignal**: Links should respect `signal` for cancellation

---

## Built-in Link Examples

### Example 1: RPCLink (HTTP)

```typescript
import { RPCLink } from '@orpc/client/fetch';

const link = new RPCLink({
  origin: 'http://localhost:3000',
  url: '/rpc',
  fetch: (url, init) => {
    return globalThis.fetch(url, {
      ...init,
      credentials: 'include',
    });
  },
});
```

**How it works**:
- Sends POST request to `{origin}{url}/{path.join('.')}`
- Body contains `{ input: ... }`
- Response is JSON: `{ result: ... }` or `{ error: ... }`

### Example 2: RPCLink (MessagePort)

```typescript
import { RPCLink } from '@orpc/client/message-port';

const port = browser.runtime.connect();
const link = new RPCLink({ port });
```

**How it works**:
- Sends messages via `port.postMessage()`
- Listens for responses via `port.onMessage`
- Uses unique message IDs for request/response matching

### Example 3: OpenAPILink (HTTP)

```typescript
import { OpenAPILink } from '@orpc/openapi/fetch';

const link = new OpenAPILink(contract, {
  origin: 'http://localhost:3000',
  url: '/api',
});
```

**How it works**:
- Uses HTTP methods from contract (GET, POST, PUT, DELETE)
- GET sends input as query params
- POST/PUT/PATCH/DELETE send input as JSON body
- Follows OpenAPI/REST conventions

---

## Implementing a Custom Link

### Step-by-Step Guide

#### 1. Define Your Link Class

```typescript
import { ORPCError } from '@orpc/client';

interface CustomLinkOptions<TContext = unknown> {
  // Your configuration options
  endpoint?: string;
  timeout?: number;
  // ... other options
}

export class CustomLink<TContext = unknown> {
  private options: CustomLinkOptions<TContext>;

  constructor(options: CustomLinkOptions<TContext> = {}) {
    this.options = options;
  }

  // Main call method
  async call<TInput, TOutput>(
    path: string[],
    input: TInput,
    callOptions?: { context?: TContext; signal?: AbortSignal }
  ): Promise<TOutput> {
    // Your implementation here
  }

  // Optional: streaming support
  async *subscribe<TInput, TOutput>(
    path: string[],
    input: TInput,
    callOptions?: { context?: TContext; signal?: AbortSignal }
  ): AsyncIterator<TOutput> {
    // Your streaming implementation
  }
}
```

#### 2. Implement the `call` Method

```typescript
async call<TInput, TOutput>(
  path: string[],
  input: TInput,
  callOptions?: { context?: TContext; signal?: AbortSignal }
): Promise<TOutput> {
  const procedurePath = path.join('.');

  try {
    // 1. Serialize input
    const serializedInput = JSON.stringify(input);

    // 2. Send request via your transport
    const response = await this.transport(procedurePath, serializedInput, callOptions);

    // 3. Handle response
    if (response.error) {
      throw new ORPCError(
        response.error.code,
        {
          message: response.error.message,
          data: response.error.data,
        }
      );
    }

    // 4. Deserialize and return
    return response.result as TOutput;
  } catch (error) {
    // 5. Handle transport errors
    if (error instanceof ORPCError) {
      throw error;
    }

    // Wrap unknown errors
    throw new ORPCError('INTERNAL_ERROR', {
      message: error instanceof Error ? error.message : 'Unknown error',
      cause: error,
    });
  }
}

// Your custom transport method
private async transport(
  path: string,
  input: string,
  options?: { context?: TContext; signal?: AbortSignal }
): Promise<{ result?: unknown; error?: { code: string; message: string; data?: unknown } }> {
  // Implement your transport logic
  // This could be HTTP, WebSocket, IPC, etc.
}
```

#### 3. Add Streaming Support (Optional)

```typescript
async *subscribe<TInput, TOutput>(
  path: string[],
  input: TInput,
  callOptions?: { context?: TContext; signal?: AbortSignal }
): AsyncIterator<TOutput> {
  const procedurePath = path.join('.');

  try {
    // Open streaming connection
    const stream = await this.openStream(procedurePath, input, callOptions);

    // Yield events as they arrive
    for await (const event of stream) {
      if (event.error) {
        throw new ORPCError(event.error.code, {
          message: event.error.message,
        });
      }

      if (event.data !== undefined) {
        yield event.data as TOutput;
      }

      // Handle close event
      if (event.type === 'close') {
        break;
      }
    }
  } catch (error) {
    if (error instanceof ORPCError) {
      throw error;
    }

    throw new ORPCError('INTERNAL_ERROR', {
      message: error instanceof Error ? error.message : 'Stream error',
      cause: error,
    });
  }
}
```

#### 4. Handle Context and Interceptors

```typescript
export class CustomLink<TContext = unknown> {
  private interceptors: Array<Interceptor<TContext>> = [];

  constructor(options: CustomLinkOptions<TContext> = {}) {
    this.options = options;
    this.interceptors = options.interceptors || [];
  }

  async call<TInput, TOutput>(
    path: string[],
    input: TInput,
    callOptions?: { context?: TContext; signal?: AbortSignal }
  ): Promise<TOutput> {
    // Run interceptors
    let index = 0;

    const runInterceptor = async (): Promise<TOutput> => {
      if (index >= this.interceptors.length) {
        // Final: execute the actual call
        return this.executeCall(path, input, callOptions);
      }

      const interceptor = this.interceptors[index++];
      return interceptor({
        path,
        input,
        context: callOptions?.context,
        next: runInterceptor,
      });
    };

    return runInterceptor();
  }

  private async executeCall<TInput, TOutput>(
    path: string[],
    input: TInput,
    callOptions?: { context?: TContext; signal?: AbortSignal }
  ): Promise<TOutput> {
    // Your actual transport logic
  }
}

type Interceptor<TContext> = (options: {
  path: string[];
  input: unknown;
  context?: TContext;
  next: () => Promise<unknown>;
}) => Promise<unknown>;
```

---

## TauriLink Implementation

### Overview

**TauriLink** enables communication between a Tauri (or Electron) frontend and Rust/Node backend without HTTP overhead.

### Architecture

```
┌─────────────────────────────────────┐
│         TypeScript Client            │
│                                      │
│  client.user.create({ ... })         │
│          ↓                           │
│      TauriLink                       │
│   (invoke Tauri command)             │
└──────────────┬──────────────────────┘
               │ IPC
┌──────────────┴──────────────────────┐
│         Rust Backend                 │
│                                      │
│  #[tauri::command]                   │
│  async fn orpc_call(...)             │
│          ↓                           │
│    orpc_router.handle(...)           │
└─────────────────────────────────────┘
```

### Implementation

```typescript
// packages/@orpc-rs/tauri/src/index.ts

import { ORPCError } from '@orpc/client';
import { invoke } from '@tauri-apps/api/core';

export interface TauriLinkOptions<TContext = unknown> {
  /**
   * Name of the Tauri command to invoke
   * @default 'orpc_call'
   */
  command?: string;

  /**
   * Interceptors for logging, auth, etc.
   */
  interceptors?: Array<TauriInterceptor<TContext>>;
}

export class TauriLink<TContext = unknown> {
  private command: string;
  private interceptors: Array<TauriInterceptor<TContext>>;

  constructor(options: TauriLinkOptions<TContext> = {}) {
    this.command = options.command || 'orpc_call';
    this.interceptors = options.interceptors || [];
  }

  async call<TInput, TOutput>(
    path: string[],
    input: TInput,
    callOptions?: { context?: TContext; signal?: AbortSignal }
  ): Promise<TOutput> {
    const procedurePath = path.join('.');

    try {
      // Run interceptors
      const result = await this.runInterceptors(
        path,
        input,
        callOptions,
        async () => {
          // Invoke Tauri command
          const response = await invoke<TauriResponse<TOutput>>(this.command, {
            path: procedurePath,
            input: input,
          });

          if ('error' in response) {
            throw new ORPCError(response.error.code, {
              message: response.error.message,
              data: response.error.data,
            });
          }

          return response.result;
        }
      );

      return result;
    } catch (error) {
      if (error instanceof ORPCError) {
        throw error;
      }

      throw new ORPCError('INTERNAL_ERROR', {
        message: error instanceof Error ? error.message : 'Tauri invoke failed',
        cause: error,
      });
    }
  }

  async *subscribe<TInput, TOutput>(
    path: string[],
    input: TInput,
    callOptions?: { context?: TContext; signal?: AbortSignal }
  ): AsyncIterator<TOutput> {
    const procedurePath = path.join('.');

    try {
      // Use Tauri event channel for streaming
      const { listen } = await import('@tauri-apps/api/event');
      const channelId = crypto.randomUUID();

      // Start the stream on backend
      await invoke(this.command, {
        path: procedurePath,
        input: input,
        channelId,
      });

      // Listen for events
      const unlisten = await listen<TauriStreamEvent<TOutput>>(
        `orpc:${channelId}`,
        (event) => {
          // Events are queued and yielded below
        }
      );

      try {
        // Event queue
        const queue: TauriStreamEvent<TOutput>[] = [];
        let done = false;

        const unlistenPromise = listen<TauriStreamEvent<TOutput>>(
          `orpc:${channelId}`,
          (event) => {
            queue.push(event.payload);
          }
        );

        while (!done) {
          // Wait for event
          while (queue.length === 0 && !done) {
            await new Promise((resolve) => setTimeout(resolve, 10));
          }

          const event = queue.shift();
          if (!event) continue;

          if (event.error) {
            throw new ORPCError(event.error.code, {
              message: event.error.message,
            });
          }

          if (event.data !== undefined) {
            yield event.data;
          }

          if (event.type === 'close') {
            done = true;
          }
        }
      } finally {
        (await unlistenPromise)();
      }
    } catch (error) {
      if (error instanceof ORPCError) {
        throw error;
      }

      throw new ORPCError('INTERNAL_ERROR', {
        message: error instanceof Error ? error.message : 'Stream error',
        cause: error,
      });
    }
  }

  private async runInterceptors<TInput, TOutput>(
    path: string[],
    input: TInput,
    callOptions: { context?: TContext; signal?: AbortSignal } | undefined,
    execute: () => Promise<TOutput>
  ): Promise<TOutput> {
    let index = 0;

    const next = async (): Promise<TOutput> => {
      if (index >= this.interceptors.length) {
        return execute();
      }

      const interceptor = this.interceptors[index++];
      return interceptor({
        path,
        input,
        context: callOptions?.context,
        next,
      });
    };

    return next();
  }
}

type TauriInterceptor<TContext> = (options: {
  path: string[];
  input: unknown;
  context?: TContext;
  next: () => Promise<unknown>;
}) => Promise<unknown>;

interface TauriResponse<T> {
  result?: T;
  error?: {
    code: string;
    message: string;
    data?: unknown;
  };
}

interface TauriStreamEvent<T> {
  type: 'data' | 'close' | 'error';
  data?: T;
  error?: {
    code: string;
    message: string;
  };
}
```

### Rust Backend Handler

```rust
// src-tauri/src/main.rs

use serde::{Deserialize, Serialize};
use tauri::Manager;

#[derive(Debug, Serialize, Deserialize)]
struct ORPCRequest {
    path: String,
    input: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct ORPCResponse<T> {
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ORPCError>,
}

#[derive(Debug, Serialize)]
struct ORPCError {
    code: String,
    message: String,
}

#[tauri::command]
async fn orpc_call(
    path: String,
    input: serde_json::Value,
) -> Result<serde_json::Value, ORPCError> {
    // Route to your orpc router
    let result = match path.as_str() {
        "user.create" => user::create(input).await,
        "user.list" => user::list(input).await,
        _ => Err(ORPCError {
            code: "NOT_FOUND".into(),
            message: format!("Procedure {} not found", path),
        }),
    };

    match result {
        Ok(data) => Ok(serde_json::to_value(ORPCResponse {
            result: Some(data),
            error: None,
        }).unwrap()),
        Err(error) => Ok(serde_json::to_value(ORPCResponse::<()> {
            result: None,
            error: Some(error),
        }).unwrap()),
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![orpc_call])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Usage

```typescript
// Client setup
import { TauriLink } from '@orpc-rs/tauri';
import { createORPCClient } from '@orpc/client';
import { type RouterContractClient } from '@orpc/contract';

const link = new TauriLink({
  interceptors: [
    async ({ path, input, next }) => {
      console.log(`→ ${path.join('.')}`, input);
      const output = await next();
      console.log(`← ${path.join('.')}`, output);
      return output;
    },
  ],
});

const client: RouterContractClient<typeof contract> = createORPCClient(link);

// Use like any oRPC client
const user = await client.user.create({
  name: "John Doe",
  email: "john@example.com"
});
```

---

## Testing Your Link

### Unit Tests

```typescript
import { describe, it, expect, vi } from 'vitest';
import { ORPCError } from '@orpc/client';
import { CustomLink } from './custom-link';

describe('CustomLink', () => {
  it('should successfully call a procedure', async () => {
    const mockTransport = vi.fn().mockResolvedValue({
      result: { id: 1, name: 'John' },
    });

    const link = new CustomLink({
      transport: mockTransport,
    });

    const result = await link.call(['user', 'create'], { name: 'John' });

    expect(mockTransport).toHaveBeenCalledWith('user.create', '{"name":"John"}', expect.anything());
    expect(result).toEqual({ id: 1, name: 'John' });
  });

  it('should handle errors correctly', async () => {
    const mockTransport = vi.fn().mockResolvedValue({
      error: {
        code: 'NOT_FOUND',
        message: 'User not found',
      },
    });

    const link = new CustomLink({
      transport: mockTransport,
    });

    await expect(
      link.call(['user', 'find'], { id: 999 })
    ).rejects.toThrow(ORPCError);
  });

  it('should respect AbortSignal', async () => {
    const controller = new AbortController();
    const mockTransport = vi.fn().mockImplementation(
      () => new Promise((resolve) => {
        setTimeout(() => resolve({ result: 'too late' }), 1000);
      })
    );

    const link = new CustomLink({
      transport: mockTransport,
    });

    setTimeout(() => controller.abort(), 100);

    await expect(
      link.call(['slow', 'operation'], {}, { signal: controller.signal })
    ).rejects.toThrow();
  });
});
```

### Integration Tests

```typescript
describe('CustomLink Integration', () => {
  it('should work with createORPCClient', async () => {
    const link = new CustomLink({
      endpoint: 'http://localhost:3000',
    });

    const client = createORPCClient(link);

    // Test actual calls
    const result = await client.user.create({
      name: 'Test User',
      email: 'test@example.com',
    });

    expect(result).toHaveProperty('id');
    expect(result.name).toBe('Test User');
  });
});
```

---

## Real-World Examples

### Example 1: WebSocket Link

```typescript
import { ORPCError } from '@orpc/client';

export class WebSocketLink<TContext = unknown> {
  private ws: WebSocket;
  private pendingCalls = new Map<string, {
    resolve: (value: unknown) => void;
    reject: (error: unknown) => void;
  }>();

  constructor(options: { url: string }) {
    this.ws = new WebSocket(options.url);
    this.ws.onmessage = this.handleMessage.bind(this);
  }

  async call<TInput, TOutput>(
    path: string[],
    input: TInput,
  ): Promise<TOutput> {
    const id = crypto.randomUUID();
    const procedurePath = path.join('.');

    return new Promise((resolve, reject) => {
      this.pendingCalls.set(id, { resolve, reject });

      this.ws.send(JSON.stringify({
        id,
        type: 'call',
        path: procedurePath,
        input,
      }));
    });
  }

  private handleMessage(event: MessageEvent) {
    const message = JSON.parse(event.data);
    const pending = this.pendingCalls.get(message.id);

    if (!pending) return;

    this.pendingCalls.delete(message.id);

    if (message.error) {
      pending.reject(new ORPCError(message.error.code, {
        message: message.error.message,
      }));
    } else {
      pending.resolve(message.result);
    }
  }
}
```

### Example 2: Local/In-Memory Link (Testing)

```typescript
import { createRouterClient } from '@orpc/server';

export class LocalLink<TRouter> {
  private serverClient: any;

  constructor(router: TRouter) {
    this.serverClient = createRouterClient(router, {
      context: {},
    });
  }

  async call<TInput, TOutput>(
    path: string[],
    input: TInput,
  ): Promise<TOutput> {
    // Navigate the router client by path
    let target = this.serverClient;
    for (const segment of path) {
      target = target[segment];
    }

    // Call the procedure directly
    return target(input);
  }
}

// Usage in tests
const link = new LocalLink(router);
const client = createORPCClient(link);

// No HTTP overhead - direct function calls
const result = await client.user.create({ name: 'Test' });
```

### Example 3: Batch Link

```typescript
export class BatchLink<TContext = unknown> {
  private batch: Array<{
    path: string[];
    input: unknown;
    resolve: (value: unknown) => void;
    reject: (error: unknown) => void;
  }> = [];
  private batchTimeout: NodeJS.Timeout | null = null;

  constructor(
    private baseLink: any,
    private options: { maxBatchSize?: number; batchWindowMs?: number } = {}
  ) {}

  async call<TInput, TOutput>(
    path: string[],
    input: TInput,
  ): Promise<TOutput> {
    return new Promise((resolve, reject) => {
      this.batch.push({ path, input, resolve, reject });

      // Schedule batch execution
      if (!this.batchTimeout) {
        this.batchTimeout = setTimeout(
          () => this.executeBatch(),
          this.options.batchWindowMs || 10
        );
      }

      // Execute immediately if batch is full
      if (this.batch.length >= (this.options.maxBatchSize || 10)) {
        clearTimeout(this.batchTimeout);
        this.executeBatch();
      }
    });
  }

  private async executeBatch() {
    const currentBatch = this.batch.splice(0);
    this.batchTimeout = null;

    try {
      // Send batch request
      const results = await this.baseLink.call(['$batch'], {
        operations: currentBatch.map(({ path, input }) => ({
          path: path.join('.'),
          input,
        })),
      });

      // Resolve individual promises
      currentBatch.forEach((item, index) => {
        const result = results[index];
        if (result.error) {
          item.reject(new ORPCError(result.error.code, {
            message: result.error.message,
          }));
        } else {
          item.resolve(result.data);
        }
      });
    } catch (error) {
      // Reject all on batch failure
      currentBatch.forEach((item) => item.reject(error));
    }
  }
}
```

---

## Summary

### Key Takeaways

1. **Links are transport adapters** - They define how client and server communicate
2. **Simple interface** - Implement `call()` (and optionally `subscribe()`)
3. **Error handling** - Always throw `ORPCError` for expected errors
4. **Context support** - Enable per-call customization (auth, tracing)
5. **Interceptors** - Add logging, retry, transformation layers
6. **Serialization** - Handle JSON or custom formats (protobuf, MessagePack)

### When to Create a Custom Link

- **New transport protocol**: gRPC, Socket.IO, custom WebSocket protocol
- **Desktop apps**: Tauri, Electron IPC
- **Browser extensions**: MessagePort between scripts
- **Performance**: Binary protocols, batching, compression
- **Testing**: In-memory link without HTTP overhead
- **Special requirements**: Custom auth, encryption, logging

### Resources

- [oRPC Official Docs](https://orpc.dev/docs)
- [oRPC GitHub](https://github.com/middleapi/orpc)
- [orpc-rs (Tauri/Rust)](https://github.com/ahonn/orpc-rs)
- [Browser Adapter Example](https://orpc.dev/docs/adapters/browser)

---

**Document Version**: 1.0  
**Last Updated**: 2026-09-04  
**License**: Educational Use
