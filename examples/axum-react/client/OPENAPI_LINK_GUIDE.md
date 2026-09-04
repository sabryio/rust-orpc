# OpenAPILink Behavior Guide

A comprehensive guide to understanding how OpenAPILink in oRPC handles HTTP methods, serialization, and communication patterns.

> **Sources**: This document synthesizes information from [orpc.dev official documentation](https://orpc.dev/docs/openapi/link) and HTTP/OpenAPI specifications. Content has been rephrased for compliance with licensing restrictions.

---

## Table of Contents

1. [Overview](#overview)
2. [HTTP Method Behavior](#http-method-behavior)
3. [Request Serialization](#request-serialization)
4. [Response Handling](#response-handling)
5. [Client Setup](#client-setup)
6. [Advanced Features](#advanced-features)
7. [Error Handling](#error-handling)
8. [Real-World Examples](#real-world-examples)

---

## Overview

**OpenAPILink** is the communication layer between an oRPC client and an OpenAPI-compliant HTTP server. It handles:

- **Request encoding**: Converting TypeScript values to HTTP requests
- **Response decoding**: Converting HTTP responses back to TypeScript values
- **Method-appropriate transport**: Different behavior for GET vs POST/PUT/PATCH/DELETE
- **Type safety**: Full TypeScript inference from contract to wire format

### Key Characteristics

```typescript
const link = new OpenAPILink(contract, {
  origin: 'https://api.example.com',  // Server URL
  url: '/api',                        // Path prefix
  fetch: customFetch,                 // Optional fetch customization
});

const client = createORPCClient(link);
```

---

## HTTP Method Behavior

OpenAPILink follows standard HTTP semantics for different methods. Here's how each method behaves:

### GET - Query Parameters

**Purpose**: Retrieve resources  
**Input Location**: URL query parameters  
**Body**: Never used  
**Idempotent**: Yes  
**Cacheable**: Yes

```typescript
// Contract definition
get: {
  echo: oc
    .meta(openapi({ method: "GET", path: "/get/echo" }))
    .input(z.object({ 
      message: z.string(),
      times: z.number().optional() 
    }))
}

// Client call
client.get.echo({ message: "hello", times: 3 })

// Actual HTTP request
// GET /get/echo?message=hello&times=3
// Headers: { ... }
// Body: (none)
```

**When to use GET**:
- Fetching data
- List/search operations
- Read-only operations
- Operations safe to repeat

**Limitations**:
- URL length limits (~2000 chars in most browsers)
- Query params visible in logs, browser history
- Cannot send complex nested objects
- No arrays of objects (only primitives)

---

### POST - Request Body (Create)

**Purpose**: Create new resources  
**Input Location**: Request body as JSON  
**Body**: Required for input  
**Idempotent**: No  
**Cacheable**: No

```typescript
// Contract definition
post: {
  createUser: oc
    .meta(openapi({ method: "POST", path: "/post/create-user" }))
    .input(z.object({
      name: z.string(),
      email: z.string().email(),
      roles: z.array(z.string()).optional(),
      metadata: z.record(z.string(), z.any()).optional(),
    }))
}

// Client call
client.post.createUser({
  name: "John Doe",
  email: "john@example.com",
  roles: ["admin", "user"],
  metadata: { region: "US" }
})

// Actual HTTP request
// POST /post/create-user
// Headers: { "Content-Type": "application/json" }
// Body: {
//   "name": "John Doe",
//   "email": "john@example.com",
//   "roles": ["admin", "user"],
//   "metadata": { "region": "US" }
// }
```

**When to use POST**:
- Creating new resources
- Complex operations that don't fit in URL
- File uploads
- Batch operations
- Operations with side effects

**Characteristics**:
- Can send any JSON structure
- No size limits (server-dependent)
- Body not visible in URL
- Each call may create duplicate resources

---

### PUT - Request Body (Full Replacement)

**Purpose**: Replace an entire resource  
**Input Location**: Request body as JSON  
**Body**: Required for input  
**Idempotent**: Yes  
**Cacheable**: No

```typescript
// Contract definition
put: {
  updateUser: oc
    .meta(openapi({ method: "PUT", path: "/put/update-user" }))
    .input(z.object({
      id: z.number(),
      name: z.string(),
      email: z.string().email(),
      age: z.number().optional(),
      roles: z.array(z.string()).optional(),
    }))
}

// Client call
client.put.updateUser({
  id: 1,
  name: "Jane Updated",
  email: "jane.updated@example.com",
  age: 35,
  roles: ["admin", "moderator"]
})

// Actual HTTP request
// PUT /put/update-user
// Headers: { "Content-Type": "application/json" }
// Body: {
//   "id": 1,
//   "name": "Jane Updated",
//   "email": "jane.updated@example.com",
//   "age": 35,
//   "roles": ["admin", "moderator"]
// }
```

**When to use PUT**:
- Replacing entire resources
- Updates requiring all fields
- Ensuring full resource consistency

**Key Difference from POST**:
- PUT is **idempotent**: calling it multiple times with same data produces same result
- PUT typically targets specific resource (`/users/123`)
- PUT expects **complete** resource representation

---

### PATCH - Request Body (Partial Update)

**Purpose**: Update specific fields of a resource  
**Input Location**: Request body as JSON  
**Body**: Required for input  
**Idempotent**: Yes (usually)  
**Cacheable**: No

```typescript
// Contract definition
patch: {
  patchUser: oc
    .meta(openapi({ method: "PATCH", path: "/patch/patch-user" }))
    .input(z.object({
      id: z.number(),
      changes: z.object({
        name: z.string().optional(),
        email: z.string().email().optional(),
        age: z.number().optional(),
      }),
    }))
}

// Client call
client.patch.patchUser({
  id: 1,
  changes: {
    name: "Partially Updated Name",
    age: 40
    // email omitted - won't be changed
  }
})

// Actual HTTP request
// PATCH /patch/patch-user
// Headers: { "Content-Type": "application/json" }
// Body: {
//   "id": 1,
//   "changes": {
//     "name": "Partially Updated Name",
//     "age": 40
//   }
// }
```

**When to use PATCH**:
- Updating only specific fields
- Partial updates to large resources
- Avoiding sending unchanged data

**Key Difference from PUT**:
- PATCH is **partial**: only specified fields change
- PUT is **complete**: entire resource replaced
- PATCH more bandwidth-efficient for large resources

---

### DELETE - Query Parameters or Body

**Purpose**: Remove resources  
**Input Location**: Query parameters (preferred) or body  
**Body**: Discouraged by HTTP spec, but supported  
**Idempotent**: Yes  
**Cacheable**: No

```typescript
// Contract definition
delete: {
  deleteUser: oc
    .meta(openapi({ method: "DELETE", path: "/delete/delete-user" }))
    .input(z.object({ id: z.number() }))
}

// Client call
client.delete.deleteUser({ id: 1 })

// Actual HTTP request (OpenAPILink uses body for DELETE with input)
// DELETE /delete/delete-user
// Headers: { "Content-Type": "application/json" }
// Body: { "id": 1 }
```

**When to use DELETE**:
- Removing resources
- Cascade deletions
- Bulk delete operations

**Important Notes**:
- HTTP spec says DELETE body has "no defined semantics"
- OpenAPILink **does support body** for DELETE when input is defined
- Some proxies/firewalls may strip DELETE bodies
- For maximum compatibility, use query params or POST for bulk deletes

---

## Request Serialization

### How OpenAPILink Serializes Different Types

#### Primitives
```typescript
// Input
{ name: "John", age: 30, active: true }

// JSON body
{
  "name": "John",
  "age": 30,
  "active": true
}
```

#### Arrays
```typescript
// Input
{ tags: ["typescript", "orpc", "api"] }

// JSON body
{
  "tags": ["typescript", "orpc", "api"]
}

// Query string (GET only, primitive arrays)
// ?tags=typescript&tags=orpc&tags=api
```

#### Nested Objects
```typescript
// Input
{
  user: {
    profile: {
      name: "John",
      settings: { theme: "dark" }
    }
  }
}

// JSON body (POST/PUT/PATCH)
{
  "user": {
    "profile": {
      "name": "John",
      "settings": { "theme": "dark" }
    }
  }
}

// Query string (GET - flattened or unsupported)
// Complex nesting not well-supported in query params
```

#### Optional Fields
```typescript
// Input with optional field present
{ name: "John", age: 30 }
// → { "name": "John", "age": 30 }

// Input with optional field omitted
{ name: "John" }
// → { "name": "John" }  (age not sent)

// Input with optional field explicitly undefined
{ name: "John", age: undefined }
// → { "name": "John" }  (age stripped)
```

#### Dates
```typescript
// Input
{ createdAt: new Date("2024-01-15") }

// JSON body (ISO 8601 string)
{
  "createdAt": "2024-01-15T00:00:00.000Z"
}
```

---

## Response Handling

### Success Responses

OpenAPILink expects responses with:
- **Status**: 2xx (200, 201, 204, etc.)
- **Content-Type**: `application/json` (usually)
- **Body**: JSON matching output schema

```typescript
// Server response
HTTP/1.1 200 OK
Content-Type: application/json

{
  "id": 1,
  "name": "John Doe",
  "email": "john@example.com"
}

// Client receives (parsed and typed)
const user: User = await client.post.createUser({ ... })
// user.id === 1
// user.name === "John Doe"
```

### Error Responses

OpenAPILink handles two types of errors:

#### 1. Typed Errors (Expected)

Errors defined in the contract:

```typescript
// Contract
create: oc
  .errors({ BAD_REQUEST: {}, AUTHENTICATION_REQUIRED: {} })
  .input(...)
  .output(...)

// Server error response
HTTP/1.1 400 Bad Request
Content-Type: application/json

{
  "code": "BAD_REQUEST",
  "message": "Planet name cannot be empty"
}

// Client handling
import { isInferableError } from "@orpc/client";

try {
  await client.planet.create({ name: "" })
} catch (error) {
  if (isInferableError(error) && error.code === "BAD_REQUEST") {
    console.log(error.message) // "Planet name cannot be empty"
  }
}
```

#### 2. Malformed Responses

When OpenAPILink cannot decode a response:

```typescript
// Server response (unexpected format)
HTTP/1.1 502 Bad Gateway
Content-Type: text/html

<html>Gateway timeout</html>

// Client error
import { MalformedResponseError, ORPCError } from "@orpc/client";

try {
  await client.someCall()
} catch (error) {
  if (error instanceof ORPCError && 
      error.cause instanceof MalformedResponseError) {
    console.log("Malformed response:", error.cause.response.status)
    console.log("Body:", error.cause.response.body)
  }
}
```

---

## Client Setup

### Basic Setup

```typescript
import { createORPCClient } from "@orpc/client";
import { OpenAPILink } from "@orpc/openapi/fetch";
import { type RouterContractClient } from "@orpc/contract";

const link = new OpenAPILink(contract, {
  origin: "http://localhost:3001",
  url: "/rpc",
});

export const client: RouterContractClient<typeof contract> = 
  createORPCClient(link);
```

### With Credentials (Cookies)

```typescript
const link = new OpenAPILink(contract, {
  origin: "http://localhost:3001",
  url: "/rpc",
  fetch(url, init) {
    return globalThis.fetch(url, {
      ...init,
      credentials: "include", // Send cookies cross-origin
    });
  },
});
```

### With Custom Headers

```typescript
const link = new OpenAPILink(contract, {
  origin: "http://localhost:3001",
  url: "/rpc",
  headers: {
    "x-api-key": "secret-key",
    "x-request-id": crypto.randomUUID(),
  },
});

// Or dynamic headers from context
type ClientContext = { token?: string };

const link = new OpenAPILink<ClientContext>(contract, {
  headers: ({ context }) => ({
    authorization: context?.token ? `Bearer ${context.token}` : undefined,
  }),
});

// Use with context
client.someCall(input, { context: { token: "abc123" } });
```

---

## Advanced Features

### Interceptors

#### Request/Response Logging

```typescript
const link = new OpenAPILink(contract, {
  interceptors: [
    async ({ next, path, input }) => {
      console.log(`→ ${path.join(".")}`, input);
      const output = await next();
      console.log(`← ${path.join(".")}`, output);
      return output;
    },
  ],
});
```

#### Error Handling

```typescript
const link = new OpenAPILink(contract, {
  interceptors: [
    async ({ next, path }) => {
      try {
        return await next();
      } catch (error) {
        console.error(`Error in ${path.join(".")}:`, error);
        throw error;
      }
    },
  ],
});
```

#### Retry Logic

```typescript
import { RetryAfterLinkPlugin } from "@orpc/openapi";

const link = new OpenAPILink(contract, {
  plugins: [
    new RetryAfterLinkPlugin(), // Respects Retry-After header
  ],
});
```

### Transport Interceptors

Modify the request before sending:

```typescript
const link = new OpenAPILink(contract, {
  transportInterceptors: [
    async (options) => {
      return await options.next({
        ...options,
        request: {
          ...options.request,
          headers: {
            ...options.request.headers,
            "x-request-id": crypto.randomUUID(),
          },
        },
      });
    },
  ],
});
```

### Fetch Interceptors

Lowest-level control (fetch adapter only):

```typescript
const link = new OpenAPILink(contract, {
  fetchInterceptors: [
    async (options) => {
      return await options.next({
        ...options,
        init: {
          ...options.init,
          credentials: "include",
          cache: "no-cache",
        },
      });
    },
  ],
});
```

---

## Error Handling

### Error Types

1. **Network Errors**: Connection failures, timeouts
2. **Typed Errors**: Expected errors from contract
3. **Malformed Responses**: Unexpected response format
4. **Validation Errors**: Input fails Zod validation

### Comprehensive Error Handling

```typescript
import { 
  isInferableError, 
  ORPCError, 
  MalformedResponseError 
} from "@orpc/client";

async function callAPI() {
  try {
    const result = await client.planet.create({
      name: "Mars",
      description: "The red planet"
    });
    return result;
  } catch (error) {
    // 1. Check for typed/expected errors
    if (isInferableError(error)) {
      if (error.code === "BAD_REQUEST") {
        console.error("Validation error:", error.message);
      } else if (error.code === "AUTHENTICATION_REQUIRED") {
        console.error("Not authenticated");
        // Redirect to login
      }
      return null;
    }

    // 2. Check for malformed responses
    if (error instanceof ORPCError && 
        error.cause instanceof MalformedResponseError) {
      console.error("Server error:", error.cause.response.status);
      return null;
    }

    // 3. Network or unknown errors
    console.error("Unexpected error:", error);
    throw error;
  }
}
```

---

## Real-World Examples

### Authentication Flow

```typescript
// 1. Define contract with auth
const contract = {
  auth: {
    login: oc
      .meta(openapi({ method: "POST", path: "/auth/login" }))
      .input(z.object({ email: z.string(), password: z.string() }))
      .output(z.object({ token: z.string() }))
  },
  profile: oc
    .meta(openapi({ method: "GET", path: "/profile" }))
    .errors({ AUTHENTICATION_REQUIRED: {} })
    .output(z.object({ id: z.number(), name: z.string() }))
};

// 2. Setup link with dynamic auth
type Context = { token?: string };

const link = new OpenAPILink<Context>(contract, {
  origin: "http://localhost:3001",
  headers: ({ context }) => ({
    authorization: context?.token ? `Bearer ${context.token}` : undefined,
  }),
});

const client: RouterContractClient<typeof contract> = createORPCClient(link);

// 3. Login and store token
const { token } = await client.auth.login({
  email: "user@example.com",
  password: "secret"
});
localStorage.setItem("token", token);

// 4. Use token in subsequent calls
const profile = await client.profile(undefined, {
  context: { token: localStorage.getItem("token") || undefined }
});
```

### Pagination

```typescript
const contract = {
  posts: {
    list: oc
      .meta(openapi({ method: "POST", path: "/posts/search" }))
      .input(z.object({
        page: z.number().default(1),
        limit: z.number().default(10),
      }))
      .output(z.object({
        items: z.array(PostSchema),
        total: z.number(),
        hasMore: z.boolean(),
      }))
  }
};

// Usage
async function loadAllPosts() {
  const allPosts = [];
  let page = 1;
  let hasMore = true;

  while (hasMore) {
    const response = await client.posts.list({ page, limit: 50 });
    allPosts.push(...response.items);
    hasMore = response.hasMore;
    page++;
  }

  return allPosts;
}
```

### File Upload (Metadata)

```typescript
const contract = {
  upload: oc
    .meta(openapi({ method: "POST", path: "/upload" }))
    .input(z.object({
      filename: z.string(),
      size: z.number(),
      mimeType: z.string(),
      base64Content: z.string(),
    }))
    .output(z.object({
      id: z.string(),
      url: z.string(),
    }))
};

// Usage
async function uploadFile(file: File) {
  const reader = new FileReader();
  const base64 = await new Promise<string>((resolve) => {
    reader.onload = () => resolve(reader.result as string);
    reader.readAsDataURL(file);
  });

  const result = await client.upload({
    filename: file.name,
    size: file.size,
    mimeType: file.type,
    base64Content: base64.split(',')[1], // Remove data URL prefix
  });

  return result;
}
```

---

## Summary Table

| Method | Input Location | Body | Idempotent | Cacheable | Best For |
|--------|---------------|------|------------|-----------|----------|
| GET | Query params | No | Yes | Yes | Reading data, searches |
| POST | Request body | Yes | No | No | Creating resources, complex operations |
| PUT | Request body | Yes | Yes | No | Replacing entire resources |
| PATCH | Request body | Yes | Usually | No | Partial updates |
| DELETE | Body or query | Optional | Yes | No | Removing resources |

---

## Key Takeaways

1. **GET uses query parameters**, all others use request body
2. **Type safety** is preserved end-to-end through Zod schemas
3. **Error handling** distinguishes typed errors from network/malformed errors
4. **Interceptors** provide logging, retry, auth at multiple levels
5. **Context** enables per-call customization (auth tokens, trace IDs)
6. **OpenAPILink is isomorphic**: works in browser, Node, Deno, Bun, Workers

---

## Additional Resources

- [oRPC OpenAPI Link Documentation](https://orpc.dev/docs/openapi/link)
- [OpenAPI Specification](https://learn.openapis.org/)
- [HTTP Methods (MDN)](https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods)

---

**Document Version**: 1.0  
**Last Updated**: 2026-09-04  
**License**: Educational Use
