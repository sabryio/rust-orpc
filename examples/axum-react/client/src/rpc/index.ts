import { createORPCClient, isInferableError } from "@orpc/client";
import { RPCLink } from "@orpc/client/websocket";
import { OpenAPILink } from "@orpc/openapi/fetch";
import { openapi } from "@orpc/openapi";
import { oc, type RouterContractClient } from "@orpc/contract";
import { asyncIteratorObject } from "@orpc/contract";
import { createTanstackQueryUtils } from "@orpc/tanstack-query";
import { z } from "zod";

// Define Zod schemas for validation
const PlanetSchema = z.object({
  id: z.number(),
  name: z.string(),
  description: z.string().optional(),
});

// Contract router matching the Rust server's routes
export const contract = {
  ping: oc.meta(openapi({ method: "POST", path: "/ping" })).output(z.string()),
  planet: {
    list: oc
      .meta(openapi({ method: "POST", path: "/planet/list" }))
      .output(z.array(PlanetSchema)),
    listPaginated: oc
      .meta(openapi({ method: "POST", path: "/planet/list-paginated" }))
      .input(
        z.object({
          limit: z.number(),
          offset: z.number().optional(),
        }),
      )
      .output(
        z.object({
          items: z.array(PlanetSchema),
          nextPageParam: z.number().optional(),
        }),
      ),
    find: oc
      .meta(openapi({ method: "POST", path: "/planet/find" }))
      .errors({
        NOT_FOUND: {},
      })
      .input(z.object({ id: z.number() }))
      .output(PlanetSchema),
    create: oc
      .meta(openapi({ method: "POST", path: "/planet/create" }))
      .errors({
        BAD_REQUEST: {},
        INTERNAL_ERROR: {},
      })
      .input(z.object({ name: z.string(), description: z.string().optional() }))
      .output(PlanetSchema),
  },

  stream: oc
    .meta(openapi({ method: "POST", path: "/stream" }))
    .output(
      asyncIteratorObject(z.object({ message: z.string(), count: z.number() })),
    ),

  streamAsync: oc
    .meta(openapi({ method: "POST", path: "/stream-async" }))
    .output(
      asyncIteratorObject(z.object({ message: z.string(), count: z.number() })),
    ),
} as const;

const link = new OpenAPILink(contract, {
  origin: "http://127.0.0.1:3001",
  url: "/rpc",
});

// Create typed client from contract — HTTP (OpenAPILink)
export const client: RouterContractClient<typeof contract> =
  createORPCClient(link);

// Create typed client from contract — WebSocket (RPCLink)
export const wsClient: RouterContractClient<typeof contract> = createORPCClient(
  new RPCLink({
    connect: () => new WebSocket("ws://127.0.0.1:3001/ws"),
    reconnect: { enabled: true },
  }),
);

// Create TanStack Query utilities
export const orpc = createTanstackQueryUtils(client);

export { isInferableError };
