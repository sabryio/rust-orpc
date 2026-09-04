import {
  createORPCClient,
  MalformedResponseError,
  onError,
  ORPCError,
} from "@orpc/client";
import { OpenAPILink } from "@orpc/openapi/fetch";
import { openapi } from "@orpc/openapi";
import { oc, type RouterContractClient } from "@orpc/contract";
import { createTanstackQueryUtils } from "@orpc/tanstack-query";
import { z } from "zod";

/**
 * Learning Contract - demonstrates all oRPC patterns
 *
 * This contract shows:
 * - Different HTTP methods (GET, POST, PUT, PATCH, DELETE)
 * - Input/output validation with Zod schemas
 * - Error handling with typed errors
 * - Query parameters vs body parameters
 * - Optional vs required fields
 * - Nested objects and arrays
 */

// ===== Schemas =====

const UserSchema = z.object({
  id: z.number(),
  name: z.string(),
  email: z.email(),
  age: z.number().optional(),
  roles: z.array(z.string()).optional(),
  metadata: z.record(z.string(), z.any()).optional(),
});

const PostSchema = z.object({
  id: z.number(),
  title: z.string(),
  content: z.string(),
  authorId: z.number(),
  tags: z.array(z.string()),
  published: z.boolean(),
  createdAt: z.string(),
});

const PaginationInputSchema = z.object({
  page: z.number().min(1).default(1),
  limit: z.number().min(1).max(100).default(10),
  sortBy: z.string().optional(),
  sortOrder: z.enum(["asc", "desc"]).optional(),
});

const PaginatedResponseSchema = <T extends z.ZodType>(itemSchema: T) =>
  z.object({
    items: z.array(itemSchema),
    total: z.number(),
    page: z.number(),
    limit: z.number(),
    hasMore: z.boolean(),
  });

// ===== Contract =====

export const learningContract = oc.meta(openapi({ prefix: "/api/v1" })).router({
  // ===== GET methods (query params) =====

  get: oc.meta(openapi({ method: "GET", prefix: "/get" })).router({
    // Simple GET with no params
    hello: oc
      .meta(openapi({ path: "/hello" }))
      .output(z.object({ message: z.string() })),

    // GET with query params
    echo: oc
      .meta(openapi({ path: "/echo" }))
      .input(
        z.object({
          message: z.string(),
          times: z.number().optional(),
        }),
      )
      .output(
        z.object({
          original: z.string(),
          repeated: z.string(),
        }),
      ),

    // GET with typed error
    user: oc
      .meta(openapi({ path: "/user" }))
      .errors({ NOT_FOUND: {}, UNAUTHORIZED: {} })
      .input(z.object({ id: z.number() }))
      .output(UserSchema),
  }),

  // ===== POST methods (body params) =====

  post: oc.meta(openapi({ method: "POST", prefix: "/post" })).router({
    // Simple POST with body
    createUser: oc
      .meta(openapi({ path: "/create-user" }))
      .errors({ BAD_REQUEST: {}, CONFLICT: {} })
      .input(
        z.object({
          name: z.string().min(1),
          email: z.string().email(),
          age: z.number().min(0).optional(),
          roles: z.array(z.string()).optional(),
        }),
      )
      .output(UserSchema),

    // POST with complex nested object
    createPost: oc
      .meta(openapi({ path: "/create-post" }))
      .errors({ BAD_REQUEST: {}, AUTHENTICATION_REQUIRED: {} })
      .input(
        z.object({
          title: z.string().min(1).max(200),
          content: z.string().min(1),
          tags: z.array(z.string()),
          published: z.boolean().default(false),
          metadata: z
            .object({
              category: z.string(),
              readTime: z.number(),
              featured: z.boolean().optional(),
            })
            .optional(),
        }),
      )
      .output(PostSchema),

    // POST with array input
    bulkCreate: oc
      .meta(openapi({ path: "/bulk-create" }))
      .input(
        z.object({
          users: z.array(
            z.object({
              name: z.string(),
              email: z.string().email(),
            }),
          ),
        }),
      )
      .output(
        z.object({
          created: z.array(UserSchema),
          failed: z.array(
            z.object({
              index: z.number(),
              reason: z.string(),
            }),
          ),
        }),
      ),

    // POST with pagination
    search: oc
      .meta(openapi({ path: "/search" }))
      .input(
        z.object({
          query: z.string(),
          filters: z
            .object({
              published: z.boolean().optional(),
              authorId: z.number().optional(),
              tags: z.array(z.string()).optional(),
            })
            .optional(),
          pagination: PaginationInputSchema.optional(),
        }),
      )
      .output(PaginatedResponseSchema(PostSchema)),
  }),

  // ===== PUT methods (full replacement) =====

  put: oc.meta(openapi({ method: "PUT", prefix: "/put" })).router({
    // Update entire resource
    updateUser: oc
      .meta(openapi({ path: "/update-user" }))
      .errors({ NOT_FOUND: {}, BAD_REQUEST: {} })
      .input(
        z.object({
          id: z.number(),
          name: z.string().min(1),
          email: z.string().email(),
          age: z.number().min(0).optional(),
          roles: z.array(z.string()).optional(),
        }),
      )
      .output(UserSchema),
  }),

  // ===== PATCH methods (partial update) =====

  patch: oc.meta(openapi({ method: "PATCH", prefix: "/patch" })).router({
    // Update specific fields
    patchUser: oc
      .meta(openapi({ path: "/patch-user" }))
      .errors({ NOT_FOUND: {}, BAD_REQUEST: {} })
      .input(
        z.object({
          id: z.number(),
          changes: z.object({
            name: z.string().optional(),
            email: z.string().email().optional(),
            age: z.number().min(0).optional(),
            roles: z.array(z.string()).optional(),
          }),
        }),
      )
      .output(UserSchema),
  }),

  // ===== DELETE methods =====

  delete: oc.meta(openapi({ method: "DELETE", prefix: "/delete" })).router({
    // Delete single resource
    deleteUser: oc
      .meta(openapi({ path: "/delete-user" }))
      .errors({ NOT_FOUND: {}, FORBIDDEN: {} })
      .input(z.object({ id: z.number() }))
      .output(
        z.object({
          deleted: z.boolean(),
          id: z.number(),
        }),
      ),

    // Bulk delete
    bulkDelete: oc
      .meta(openapi({ path: "/bulk-delete" }))
      .input(
        z.object({
          ids: z.array(z.number()),
        }),
      )
      .output(
        z.object({
          deleted: z.array(z.number()),
          failed: z.array(
            z.object({
              id: z.number(),
              reason: z.string(),
            }),
          ),
        }),
      ),
  }),

  // ===== Special cases =====

  special: oc.meta(openapi({ method: "POST", prefix: "/special" })).router({
    // No input, just output
    random: oc.meta(openapi({ path: "/random" })).output(
      z.object({
        value: z.number(),
        timestamp: z.string(),
      }),
    ),

    // Complex validation
    validateEmail: oc
      .meta(openapi({ path: "/validate-email" }))
      .input(
        z.object({
          email: z.string().email(),
        }),
      )
      .output(
        z.object({
          valid: z.boolean(),
          domain: z.string(),
          disposable: z.boolean(),
          suggestions: z.array(z.string()).optional(),
        }),
      ),

    // File upload metadata (body contains file info)
    uploadFile: oc
      .meta(openapi({ path: "/upload-file" }))
      .input(
        z.object({
          filename: z.string(),
          size: z.number(),
          mimeType: z.string(),
          base64Content: z.string(),
        }),
      )
      .output(
        z.object({
          id: z.string(),
          url: z.string(),
          uploadedAt: z.string(),
        }),
      ),
  }),
});

// ===== Client Setup =====

interface ClientContext {
  token?: string;
}

const link = new OpenAPILink<ClientContext>(learningContract, {
  origin: "http://localhost:3002", // Point to inspector server
  url: "/",

  // Static headers sent with EVERY request
  // These will be visible in the inspector server terminal
  headers: ({ context }) => ({
    authorization: context?.token ? `Bearer ${context.token}` : undefined,
    "x-api-key": "learning-demo-key",
    "x-client-version": "1.0.0",
  }),

  fetchInterceptors: [
    async (options) => {
      const response = await options.next({
        ...options,
        init: {
          ...options.init,
          credentials: "include",
        },
      });

      return response;
    },
  ],

  async fetch(url, init, _options, path) {
    console.group(`🔍 oRPC Request: ${path.join(".")}`);
    console.log("URL:", url);
    console.log("Method:", init?.method || "POST");
    console.log("Headers:", init?.headers);
    console.log("Body:", init?.body ? JSON.parse(init.body as string) : null);
    console.groupEnd();

    const response = await globalThis.fetch(url, {
      ...init,
      credentials: "include",
    });
    const clone = response.clone();
    const body = await clone.json().catch(() => null);
    console.group(`📥 oRPC Response: ${path.join(".")}`);
    console.log("Status:", response.status, response.statusText);
    console.log(
      "Headers:",
      Object.fromEntries([...response.headers.entries()]),
    );
    console.log("Body:", body);
    console.groupEnd();
    return response;
  },
  interceptors: [
    async ({ next, path, input: _, context }) => {
      console.log("Client context:", context);
      console.time(path.join("."));

      try {
        const output = await next();
        return output;
      } catch (err) {
        console.error(`${path.join(".")}:`, err);
        throw err;
      } finally {
        console.timeEnd(path.join("."));
      }
    },
    onError((error) => {
      if (
        error instanceof ORPCError &&
        error.cause instanceof MalformedResponseError
      ) {
        console.error(
          "Malformed response:",
          error.cause.response.status,
          error.cause.response.body,
        );
      }
    }),
  ],
  transportInterceptors: [
    async (options) => {
      const response = await options.next({
        ...options,
        request: {
          ...options.request,
          headers: {
            ...options.request.headers,
            "x-request-id": crypto.randomUUID(),
          },
        },
      });

      return response;
    },
  ],
});

export const learningClient: RouterContractClient<
  typeof learningContract,
  ClientContext
> = createORPCClient(link);

export const learningOrpc = createTanstackQueryUtils(learningClient);

// Type exports for use in components
export type User = z.infer<typeof UserSchema>;
export type Post = z.infer<typeof PostSchema>;
