import { createFileRoute } from "@tanstack/react-router";
import {
  QueryClient,
  QueryClientProvider,
  useQuery,
  useMutation,
  useQueryClient,
} from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { useState, useEffect, useRef } from "react";
import { client, orpc } from "#/rpc";

export const Route = createFileRoute("/")({ component: Home });

const queryClient = new QueryClient();

function Home() {
  return (
    <QueryClientProvider client={queryClient}>
      <div className="min-h-screen bg-gray-50">
        <div className="max-w-4xl mx-auto p-8">
          <h1 className="text-4xl font-bold text-gray-900 mb-8">
            🪐 Planet Explorer
          </h1>

          <div className="space-y-8">
            <PingTest />
            <PlanetList />
            <PlanetFind />
            <CreatePlanet />
            <StreamEvents />
          </div>
        </div>
      </div>
      <ReactQueryDevtools initialIsOpen={false} />
    </QueryClientProvider>
  );
}

function PingTest() {
  const mutation = useMutation(orpc.ping.mutationOptions());

  return (
    <div className="bg-white rounded-lg shadow p-6">
      <h2 className="text-2xl font-semibold mb-4">Test Connection</h2>
      <button
        onClick={() => mutation.mutate(undefined)}
        disabled={mutation.isPending}
        className="bg-blue-500 hover:bg-blue-600 text-white px-6 py-2 rounded-md disabled:opacity-50"
      >
        {mutation.isPending ? "Pinging..." : "Ping Server"}
      </button>
      {mutation.data && (
        <p className="mt-4 text-lg">
          Response: <span className="font-mono font-bold">{mutation.data}</span>
        </p>
      )}
      {mutation.error && (
        <p className="mt-4 text-red-500">Error: {String(mutation.error)}</p>
      )}
    </div>
  );
}

function PlanetList() {
  const {
    data: planets,
    isLoading,
    error,
    refetch,
  } = useQuery(orpc.planet.list.queryOptions());

  if (isLoading) {
    return (
      <div className="bg-white rounded-lg shadow p-6">
        <p>Loading planets...</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-white rounded-lg shadow p-6">
        <p className="text-red-500">Error loading planets: {String(error)}</p>
      </div>
    );
  }

  return (
    <div className="bg-white rounded-lg shadow p-6">
      <div className="flex justify-between items-center mb-4">
        <h2 className="text-2xl font-semibold">All Planets</h2>
        <button
          onClick={() => refetch()}
          className="text-sm text-blue-500 hover:text-blue-600"
        >
          Refresh
        </button>
      </div>

      <div className="space-y-3">
        {planets?.map((planet) => (
          <div
            key={planet.id}
            className="border border-gray-200 rounded-md p-4 hover:border-blue-300 transition-colors"
          >
            <h3 className="text-lg font-medium">{planet.name}</h3>
            {planet.description && (
              <p className="text-gray-600 mt-1">{planet.description}</p>
            )}
            <p className="text-sm text-gray-400 mt-2">ID: {planet.id}</p>
          </div>
        ))}
      </div>
    </div>
  );
}

function PlanetFind() {
  const [id, setId] = useState<number>(1);

  const {
    data: planet,
    isLoading,
    error,
  } = useQuery(orpc.planet.find.queryOptions({ input: { id } }));

  return (
    <div className="bg-white rounded-lg shadow p-6">
      <h2 className="text-2xl font-semibold mb-4">Find Planet by ID</h2>

      <div className="flex gap-4 items-center mb-4">
        <label className="text-sm font-medium text-gray-700">Planet ID:</label>
        <input
          type="number"
          value={id}
          min={1}
          onChange={(e) => setId(Number(e.target.value))}
          className="w-24 border border-gray-300 rounded-md px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
      </div>

      {isLoading && <p className="text-gray-500">Looking up planet...</p>}

      {error && <p className="text-red-500">Error: {String(error)}</p>}

      {planet && (
        <div className="border border-green-200 bg-green-50 rounded-md p-4">
          <h3 className="text-lg font-medium">{planet.name}</h3>
          {planet.description && (
            <p className="text-gray-600 mt-1">{planet.description}</p>
          )}
          <p className="text-sm text-gray-400 mt-2">ID: {planet.id}</p>
        </div>
      )}
    </div>
  );
}

function CreatePlanet() {
  const qc = useQueryClient();
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");

  const mutation = useMutation(
    orpc.planet.create.mutationOptions({
      onSuccess: () => {
        // Invalidate the planet list so it refetches
        qc.invalidateQueries({ queryKey: orpc.planet.key() });
        setName("");
        setDescription("");
      },
    }),
  );

  const handleCreate = (e: React.FormEvent) => {
    e.preventDefault();
    mutation.mutate({ name, description: description || undefined });
  };

  return (
    <div className="bg-white rounded-lg shadow p-6">
      <h2 className="text-2xl font-semibold mb-4">Create New Planet</h2>

      <form onSubmit={handleCreate} className="space-y-4">
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            Planet Name *
          </label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            required
            className="w-full border border-gray-300 rounded-md px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
            placeholder="e.g., Neptune"
          />
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            Description
          </label>
          <textarea
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            rows={3}
            className="w-full border border-gray-300 rounded-md px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
            placeholder="Optional description..."
          />
        </div>

        <button
          type="submit"
          disabled={mutation.isPending || !name.trim()}
          className="bg-green-500 hover:bg-green-600 text-white px-6 py-2 rounded-md disabled:opacity-50"
        >
          {mutation.isPending ? "Creating..." : "Create Planet"}
        </button>
      </form>

      {mutation.data && (
        <div className="mt-4 p-4 bg-green-50 border border-green-200 rounded-md">
          <p className="text-green-800 font-medium">✓ Planet created!</p>
          <pre className="mt-2 text-sm text-gray-700">
            {JSON.stringify(mutation.data, null, 2)}
          </pre>
        </div>
      )}

      {mutation.error && (
        <div className="mt-4 p-4 bg-red-50 border border-red-200 rounded-md">
          <p className="text-red-800">{String(mutation.error)}</p>
        </div>
      )}
    </div>
  );
}

function StreamEvents() {
  const [events, setEvents] = useState<{ message: string; count: number }[]>([])
  const [streaming, setStreaming] = useState(false)
  const [error, setError] = useState<string>('')
  const abortRef = useRef<AbortController | null>(null)

  const handleStart = async () => {
    setEvents([])
    setError('')
    setStreaming(true)

    const controller = new AbortController()
    abortRef.current = controller

    try {
      const iterator = await client.stream(undefined, { signal: controller.signal })
      for await (const event of iterator) {
        setEvents((prev) => [...prev, event])
      }
    } catch (err: unknown) {
      if (err instanceof Error && err.name !== 'AbortError') {
        setError(String(err))
      }
    } finally {
      setStreaming(false)
      abortRef.current = null
    }
  }

  const handleStop = () => {
    abortRef.current?.abort()
  }

  // cleanup on unmount
  useEffect(() => () => abortRef.current?.abort(), [])

  return (
    <div className="bg-white rounded-lg shadow p-6">
      <h2 className="text-2xl font-semibold mb-4">Stream Events (SSE)</h2>

      <div className="flex gap-3 mb-4">
        <button
          onClick={handleStart}
          disabled={streaming}
          className="bg-purple-500 hover:bg-purple-600 text-white px-6 py-2 rounded-md disabled:opacity-50"
        >
          {streaming ? 'Streaming...' : 'Start Stream'}
        </button>
        <button
          onClick={handleStop}
          disabled={!streaming}
          className="bg-red-500 hover:bg-red-600 text-white px-6 py-2 rounded-md disabled:opacity-50"
        >
          Stop
        </button>
      </div>

      {error && (
        <p className="text-red-500 mb-3">{error}</p>
      )}

      <div className="space-y-1 font-mono text-sm">
        {events.map((e) => (
          <div key={e.count} className="flex gap-3 p-2 bg-gray-50 rounded">
            <span className="text-gray-400">#{e.count}</span>
            <span>{e.message}</span>
          </div>
        ))}
        {streaming && (
          <div className="p-2 text-purple-500 animate-pulse">Waiting for next event…</div>
        )}
        {!streaming && events.length === 0 && (
          <p className="text-gray-400">No events yet. Press Start Stream.</p>
        )}
      </div>
    </div>
  )
}
