import { orpc } from "#/rpc";
import { useSession } from "@/lib/auth-client";
import {
  QueryClient,
  QueryClientProvider,
  useQuery,
} from "@tanstack/react-query";
import { createFileRoute, Link } from "@tanstack/react-router";

export const Route = createFileRoute("/")({ component: Home });

const queryClient = new QueryClient();

function Home() {
  return (
    <QueryClientProvider client={queryClient}>
      <div className="min-h-screen bg-linear-to-br from-neutral-50 to-neutral-100">
        <div className="max-w-4xl mx-auto px-6 py-16">
          {/* Hero Section */}
          <div className="text-center mb-12">
            <h1 className="text-5xl font-bold tracking-tight text-neutral-900 mb-4">
              Welcome to oRPC
            </h1>
            <p className="text-xl text-neutral-600 mb-8">
              End-to-end type-safe RPC framework for Rust + TypeScript
            </p>
            <AuthStatus />
          </div>

          {/* Quick Links */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-12">
            <Link
              to="/demo"
              className="p-6 bg-white border-2 border-neutral-200 rounded-lg hover:border-neutral-900 hover:shadow-lg transition-all group"
            >
              <div className="text-2xl mb-2">🚀</div>
              <h3 className="text-lg font-semibold text-neutral-900 mb-2 group-hover:text-neutral-700">
                Full Demo
              </h3>
              <p className="text-sm text-neutral-600">
                Explore all oRPC features: CRUD, streaming, WebSocket
              </p>
            </Link>

            <Link
              to="/auth"
              className="p-6 bg-white border-2 border-neutral-200 rounded-lg hover:border-neutral-900 hover:shadow-lg transition-all group"
            >
              <div className="text-2xl mb-2">🔐</div>
              <h3 className="text-lg font-semibold text-neutral-900 mb-2 group-hover:text-neutral-700">
                Authentication
              </h3>
              <p className="text-sm text-neutral-600">
                Sign in or create an account with Better Auth
              </p>
            </Link>

            <Link
              to="/profile"
              className="p-6 bg-white border-2 border-neutral-200 rounded-lg hover:border-neutral-900 hover:shadow-lg transition-all group"
            >
              <div className="text-2xl mb-2">👤</div>
              <h3 className="text-lg font-semibold text-neutral-900 mb-2 group-hover:text-neutral-700">
                Profile
              </h3>
              <p className="text-sm text-neutral-600">
                View your account details and manage sessions
              </p>
            </Link>
          </div>

          {/* Simple Example */}
          <div className="mb-8">
            <h2 className="text-sm font-medium uppercase tracking-wider text-neutral-500 mb-4">
              Quick Example
            </h2>
            <SimplePlanetList />
          </div>

          {/* Tech Stack */}
          <div className="bg-white border border-neutral-200 rounded-lg p-6">
            <h3 className="text-sm font-semibold text-neutral-900 mb-4">
              Tech Stack
            </h3>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
              <div>
                <div className="font-mono text-xs text-neutral-500 mb-1">
                  Backend
                </div>
                <div className="text-neutral-900">Rust + Axum</div>
              </div>
              <div>
                <div className="font-mono text-xs text-neutral-500 mb-1">
                  Frontend
                </div>
                <div className="text-neutral-900">React + TanStack</div>
              </div>
              <div>
                <div className="font-mono text-xs text-neutral-500 mb-1">
                  Auth
                </div>
                <div className="text-neutral-900">Better Auth RS</div>
              </div>
              <div>
                <div className="font-mono text-xs text-neutral-500 mb-1">
                  RPC
                </div>
                <div className="text-neutral-900">oRPC Framework</div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </QueryClientProvider>
  );
}

function AuthStatus() {
  const { data: session, isPending } = useSession();

  if (isPending) {
    return (
      <div className="inline-flex items-center gap-2 px-4 py-2 bg-white border border-neutral-200 rounded-full">
        <div className="w-2 h-2 bg-neutral-300 rounded-full animate-pulse"></div>
        <span className="text-sm text-neutral-500">Checking auth...</span>
      </div>
    );
  }

  if (session?.user) {
    return (
      <div className="inline-flex items-center gap-2 px-4 py-2 bg-emerald-50 border border-emerald-200 rounded-full">
        <div className="w-2 h-2 bg-emerald-500 rounded-full"></div>
        <span className="text-sm text-emerald-800">
          Signed in as <strong>{session.user.email}</strong>
        </span>
      </div>
    );
  }

  return (
    <div className="inline-flex items-center gap-2 px-4 py-2 bg-neutral-100 border border-neutral-200 rounded-full">
      <div className="w-2 h-2 bg-neutral-400 rounded-full"></div>
      <span className="text-sm text-neutral-600">Not signed in</span>
    </div>
  );
}

function SimplePlanetList() {
  const {
    data: planets,
    isLoading,
    error,
  } = useQuery(orpc.planet.list.queryOptions());

  if (isLoading) {
    return (
      <div className="bg-white border border-neutral-200 rounded-lg p-8">
        <div className="animate-pulse space-y-3">
          <div className="h-4 bg-neutral-100 rounded w-32"></div>
          <div className="h-12 bg-neutral-100 rounded"></div>
          <div className="h-12 bg-neutral-100 rounded"></div>
          <div className="h-12 bg-neutral-100 rounded"></div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-white border border-red-200 rounded-lg p-8">
        <p className="text-sm text-red-600">{String(error)}</p>
      </div>
    );
  }

  return (
    <div className="bg-white border border-neutral-200 rounded-lg overflow-hidden">
      <div className="px-6 py-4 border-b border-neutral-200">
        <h3 className="text-sm font-semibold text-neutral-900">
          Planets ({planets?.length ?? 0})
        </h3>
      </div>

      <div className="divide-y divide-neutral-100">
        {planets?.slice(0, 5).map((planet) => (
          <div
            key={planet.id}
            className="px-6 py-3 hover:bg-neutral-50 transition-colors"
          >
            <div className="flex items-center justify-between">
              <div>
                <span className="text-sm font-medium text-neutral-900">
                  {planet.name}
                </span>
                {planet.description && (
                  <p className="text-xs text-neutral-500 mt-0.5">
                    {planet.description}
                  </p>
                )}
              </div>
              <span className="text-xs font-mono text-neutral-400">
                #{planet.id}
              </span>
            </div>
          </div>
        ))}
      </div>

      {planets && planets.length > 5 && (
        <div className="px-6 py-3 border-t border-neutral-200 bg-neutral-50">
          <Link
            to="/demo"
            className="text-sm text-neutral-600 hover:text-neutral-900 font-medium"
          >
            View all {planets.length} planets →
          </Link>
        </div>
      )}
    </div>
  );
}
