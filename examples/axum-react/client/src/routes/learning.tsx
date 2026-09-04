import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { learningOrpc } from "#/rpc/learning";

export const Route = createFileRoute("/learning")({
  component: LearningPage,
});

function LearningPage() {
  return (
    <div className="min-h-screen bg-neutral-50">
      <header className="border-b border-neutral-200 bg-white">
        <div className="max-w-7xl mx-auto px-6 py-6">
          <div className="flex items-baseline gap-3">
            <h1 className="text-2xl font-semibold tracking-tight text-neutral-900">
              oRPC Learning Lab
            </h1>
            <span className="text-sm text-neutral-400 font-mono">
              Request/Response Inspector
            </span>
          </div>
          <p className="text-sm text-neutral-600 mt-2">
            Open your browser console (F12) to see detailed request/response
            logs
          </p>
        </div>
      </header>

      <div className="max-w-7xl mx-auto px-6 py-12 space-y-12">
        <GetMethodsSection />
        <PostMethodsSection />
        <PutMethodsSection />
        <PatchMethodsSection />
        <DeleteMethodsSection />
        <SpecialCasesSection />
      </div>
    </div>
  );
}

// ===== GET Methods Section =====

function GetMethodsSection() {
  const [userId, setUserId] = useState(1);
  const [echoMessage, setEchoMessage] = useState("Hello");

  const helloMutation = useMutation(learningOrpc.get.hello.mutationOptions());
  const echoMutation = useMutation(learningOrpc.get.echo.mutationOptions());
  const userQuery = useQuery(
    learningOrpc.get.user.queryOptions({ input: { id: userId } }),
  );

  return (
    <section>
      <h2 className="text-xs font-medium uppercase tracking-wider text-neutral-500 mb-6">
        GET Methods (Query Parameters)
      </h2>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Hello */}
        <div className="bg-white border border-neutral-200 rounded-lg p-6">
          <h3 className="text-sm font-semibold text-neutral-900 mb-4">
            Simple GET
          </h3>
          <p className="text-xs text-neutral-500 mb-4">
            GET /get/hello - No parameters
          </p>
          <button
            onClick={() => helloMutation.mutate(undefined)}
            disabled={helloMutation.isPending}
            className="w-full px-4 py-2 bg-green-600 text-white text-sm font-medium rounded hover:bg-green-700 disabled:opacity-50"
          >
            {helloMutation.isPending ? "Loading..." : "Call"}
          </button>
          {helloMutation.data && (
            <pre className="mt-4 p-3 bg-neutral-50 rounded text-xs overflow-auto">
              {JSON.stringify(helloMutation.data, null, 2)}
            </pre>
          )}
          {helloMutation.error && (
            <div className="mt-4 p-3 bg-red-50 text-red-800 rounded text-xs">
              {String(helloMutation.error)}
            </div>
          )}
        </div>

        {/* Echo */}
        <div className="bg-white border border-neutral-200 rounded-lg p-6">
          <h3 className="text-sm font-semibold text-neutral-900 mb-4">
            GET with Query Params
          </h3>
          <p className="text-xs text-neutral-500 mb-4">
            GET /get/echo?message=...&times=...
          </p>
          <input
            type="text"
            value={echoMessage}
            onChange={(e) => setEchoMessage(e.target.value)}
            className="w-full px-3 py-2 text-sm border border-neutral-300 rounded mb-3"
            placeholder="Message"
          />
          <button
            onClick={() =>
              echoMutation.mutate({ message: echoMessage, times: 3 })
            }
            disabled={echoMutation.isPending}
            className="w-full px-4 py-2 bg-green-600 text-white text-sm font-medium rounded hover:bg-green-700 disabled:opacity-50"
          >
            {echoMutation.isPending ? "Loading..." : "Echo"}
          </button>
          {echoMutation.data && (
            <pre className="mt-4 p-3 bg-neutral-50 rounded text-xs overflow-auto">
              {JSON.stringify(echoMutation.data, null, 2)}
            </pre>
          )}
        </div>

        {/* User */}
        <div className="bg-white border border-neutral-200 rounded-lg p-6">
          <h3 className="text-sm font-semibold text-neutral-900 mb-4">
            GET with Errors
          </h3>
          <p className="text-xs text-neutral-500 mb-4">
            GET /get/user?id=... - Can return NOT_FOUND
          </p>
          <input
            type="number"
            value={userId}
            onChange={(e) => setUserId(Number(e.target.value))}
            className="w-full px-3 py-2 text-sm border border-neutral-300 rounded mb-3"
            placeholder="User ID"
          />
          {userQuery.isLoading && (
            <div className="text-sm text-neutral-500">Loading...</div>
          )}
          {userQuery.data && (
            <pre className="p-3 bg-neutral-50 rounded text-xs overflow-auto">
              {JSON.stringify(userQuery.data, null, 2)}
            </pre>
          )}
          {userQuery.error && (
            <div className="p-3 bg-red-50 text-red-800 rounded text-xs">
              {String(userQuery.error)}
            </div>
          )}
        </div>
      </div>
    </section>
  );
}

// ===== POST Methods Section =====

function PostMethodsSection() {
  const [userName, setUserName] = useState("John Doe");
  const [userEmail, setUserEmail] = useState("john@example.com");

  const createUserMutation = useMutation(
    learningOrpc.post.createUser.mutationOptions(),
  );

  const createPostMutation = useMutation(
    learningOrpc.post.createPost.mutationOptions(),
  );

  const searchMutation = useMutation(
    learningOrpc.post.search.mutationOptions(),
  );

  return (
    <section>
      <h2 className="text-xs font-medium uppercase tracking-wider text-neutral-500 mb-6">
        POST Methods (Request Body)
      </h2>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Create User */}
        <div className="bg-white border border-neutral-200 rounded-lg p-6">
          <h3 className="text-sm font-semibold text-neutral-900 mb-4">
            Create User
          </h3>
          <p className="text-xs text-neutral-500 mb-4">
            POST /post/create-user - Body with validation
          </p>
          <input
            type="text"
            value={userName}
            onChange={(e) => setUserName(e.target.value)}
            className="w-full px-3 py-2 text-sm border border-neutral-300 rounded mb-2"
            placeholder="Name"
          />
          <input
            type="email"
            value={userEmail}
            onChange={(e) => setUserEmail(e.target.value)}
            className="w-full px-3 py-2 text-sm border border-neutral-300 rounded mb-3"
            placeholder="Email"
          />
          <button
            onClick={() =>
              createUserMutation.mutate({
                name: userName,
                email: userEmail,
                age: 30,
                roles: ["user", "admin"],
              })
            }
            disabled={createUserMutation.isPending}
            className="w-full px-4 py-2 bg-blue-600 text-white text-sm font-medium rounded hover:bg-blue-700 disabled:opacity-50"
          >
            {createUserMutation.isPending ? "Creating..." : "Create"}
          </button>
          {createUserMutation.data && (
            <pre className="mt-4 p-3 bg-neutral-50 rounded text-xs overflow-auto">
              {JSON.stringify(createUserMutation.data, null, 2)}
            </pre>
          )}
          {createUserMutation.error && (
            <div className="mt-4 p-3 bg-red-50 text-red-800 rounded text-xs">
              {String(createUserMutation.error)}
            </div>
          )}
        </div>

        {/* Create Post */}
        <div className="bg-white border border-neutral-200 rounded-lg p-6">
          <h3 className="text-sm font-semibold text-neutral-900 mb-4">
            Create Post (Nested)
          </h3>
          <p className="text-xs text-neutral-500 mb-4">
            POST /post/create-post - Complex nested object
          </p>
          <button
            onClick={() =>
              createPostMutation.mutate({
                title: "Learning oRPC",
                content: "This is a test post with nested metadata",
                tags: ["tutorial", "orpc", "typescript"],
                published: true,
                metadata: {
                  category: "tech",
                  readTime: 5,
                  featured: true,
                },
              })
            }
            disabled={createPostMutation.isPending}
            className="w-full px-4 py-2 bg-blue-600 text-white text-sm font-medium rounded hover:bg-blue-700 disabled:opacity-50"
          >
            {createPostMutation.isPending ? "Creating..." : "Create Post"}
          </button>
          {createPostMutation.data && (
            <pre className="mt-4 p-3 bg-neutral-50 rounded text-xs overflow-auto max-h-48">
              {JSON.stringify(createPostMutation.data, null, 2)}
            </pre>
          )}
        </div>

        {/* Search */}
        <div className="bg-white border border-neutral-200 rounded-lg p-6">
          <h3 className="text-sm font-semibold text-neutral-900 mb-4">
            Search with Pagination
          </h3>
          <p className="text-xs text-neutral-500 mb-4">
            POST /post/search - Complex filters + pagination
          </p>
          <button
            onClick={() =>
              searchMutation.mutate({
                query: "typescript",
                filters: {
                  published: true,
                  tags: ["tutorial"],
                },
                pagination: {
                  page: 1,
                  limit: 10,
                  sortBy: "createdAt",
                  sortOrder: "desc",
                },
              })
            }
            disabled={searchMutation.isPending}
            className="w-full px-4 py-2 bg-blue-600 text-white text-sm font-medium rounded hover:bg-blue-700 disabled:opacity-50"
          >
            {searchMutation.isPending ? "Searching..." : "Search"}
          </button>
          {searchMutation.data && (
            <pre className="mt-4 p-3 bg-neutral-50 rounded text-xs overflow-auto max-h-48">
              {JSON.stringify(searchMutation.data, null, 2)}
            </pre>
          )}
        </div>
      </div>
    </section>
  );
}

// ===== PUT Methods Section =====

function PutMethodsSection() {
  const updateMutation = useMutation(
    learningOrpc.put.updateUser.mutationOptions(),
  );

  return (
    <section>
      <h2 className="text-xs font-medium uppercase tracking-wider text-neutral-500 mb-6">
        PUT Methods (Full Replacement)
      </h2>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="bg-white border border-neutral-200 rounded-lg p-6">
          <h3 className="text-sm font-semibold text-neutral-900 mb-4">
            Update User (Full)
          </h3>
          <p className="text-xs text-neutral-500 mb-4">
            PUT /put/update-user - Replace entire resource
          </p>
          <button
            onClick={() =>
              updateMutation.mutate({
                id: 1,
                name: "Jane Updated",
                email: "jane.updated@example.com",
                age: 35,
                roles: ["admin", "moderator"],
              })
            }
            disabled={updateMutation.isPending}
            className="w-full px-4 py-2 bg-yellow-600 text-white text-sm font-medium rounded hover:bg-yellow-700 disabled:opacity-50"
          >
            {updateMutation.isPending ? "Updating..." : "Update"}
          </button>
          {updateMutation.data && (
            <pre className="mt-4 p-3 bg-neutral-50 rounded text-xs overflow-auto">
              {JSON.stringify(updateMutation.data, null, 2)}
            </pre>
          )}
        </div>
      </div>
    </section>
  );
}

// ===== PATCH Methods Section =====

function PatchMethodsSection() {
  const patchMutation = useMutation(
    learningOrpc.patch.patchUser.mutationOptions(),
  );

  return (
    <section>
      <h2 className="text-xs font-medium uppercase tracking-wider text-neutral-500 mb-6">
        PATCH Methods (Partial Update)
      </h2>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="bg-white border border-neutral-200 rounded-lg p-6">
          <h3 className="text-sm font-semibold text-neutral-900 mb-4">
            Patch User (Partial)
          </h3>
          <p className="text-xs text-neutral-500 mb-4">
            PATCH /patch/patch-user - Update specific fields only
          </p>
          <button
            onClick={() =>
              patchMutation.mutate({
                id: 1,
                changes: {
                  name: "Partially Updated Name",
                  age: 40,
                },
              })
            }
            disabled={patchMutation.isPending}
            className="w-full px-4 py-2 bg-purple-600 text-white text-sm font-medium rounded hover:bg-purple-700 disabled:opacity-50"
          >
            {patchMutation.isPending ? "Patching..." : "Patch"}
          </button>
          {patchMutation.data && (
            <pre className="mt-4 p-3 bg-neutral-50 rounded text-xs overflow-auto">
              {JSON.stringify(patchMutation.data, null, 2)}
            </pre>
          )}
        </div>
      </div>
    </section>
  );
}

// ===== DELETE Methods Section =====

function DeleteMethodsSection() {
  const deleteMutation = useMutation(
    learningOrpc.delete.deleteUser.mutationOptions(),
  );

  const bulkDeleteMutation = useMutation(
    learningOrpc.delete.bulkDelete.mutationOptions(),
  );

  return (
    <section>
      <h2 className="text-xs font-medium uppercase tracking-wider text-neutral-500 mb-6">
        DELETE Methods
      </h2>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="bg-white border border-neutral-200 rounded-lg p-6">
          <h3 className="text-sm font-semibold text-neutral-900 mb-4">
            Delete Single User
          </h3>
          <p className="text-xs text-neutral-500 mb-4">
            DELETE /delete/delete-user - Remove one resource
          </p>
          <button
            onClick={() => deleteMutation.mutate({ id: 1 })}
            disabled={deleteMutation.isPending}
            className="w-full px-4 py-2 bg-red-600 text-white text-sm font-medium rounded hover:bg-red-700 disabled:opacity-50"
          >
            {deleteMutation.isPending ? "Deleting..." : "Delete"}
          </button>
          {deleteMutation.data && (
            <pre className="mt-4 p-3 bg-neutral-50 rounded text-xs overflow-auto">
              {JSON.stringify(deleteMutation.data, null, 2)}
            </pre>
          )}
        </div>

        <div className="bg-white border border-neutral-200 rounded-lg p-6">
          <h3 className="text-sm font-semibold text-neutral-900 mb-4">
            Bulk Delete
          </h3>
          <p className="text-xs text-neutral-500 mb-4">
            DELETE /delete/bulk-delete - Remove multiple resources
          </p>
          <button
            onClick={() => bulkDeleteMutation.mutate({ ids: [1, 2, 3, 4, 5] })}
            disabled={bulkDeleteMutation.isPending}
            className="w-full px-4 py-2 bg-red-600 text-white text-sm font-medium rounded hover:bg-red-700 disabled:opacity-50"
          >
            {bulkDeleteMutation.isPending ? "Deleting..." : "Delete Multiple"}
          </button>
          {bulkDeleteMutation.data && (
            <pre className="mt-4 p-3 bg-neutral-50 rounded text-xs overflow-auto">
              {JSON.stringify(bulkDeleteMutation.data, null, 2)}
            </pre>
          )}
        </div>
      </div>
    </section>
  );
}

// ===== Special Cases Section =====

function SpecialCasesSection() {
  const randomMutation = useMutation(
    learningOrpc.special.random.mutationOptions(),
  );

  const validateMutation = useMutation(
    learningOrpc.special.validateEmail.mutationOptions(),
  );

  return (
    <section>
      <h2 className="text-xs font-medium uppercase tracking-wider text-neutral-500 mb-6">
        Special Cases
      </h2>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="bg-white border border-neutral-200 rounded-lg p-6">
          <h3 className="text-sm font-semibold text-neutral-900 mb-4">
            No Input Required
          </h3>
          <p className="text-xs text-neutral-500 mb-4">
            POST /special/random - Returns random data
          </p>
          <button
            onClick={() => randomMutation.mutate(undefined)}
            disabled={randomMutation.isPending}
            className="w-full px-4 py-2 bg-neutral-900 text-white text-sm font-medium rounded hover:bg-neutral-800 disabled:opacity-50"
          >
            {randomMutation.isPending ? "Generating..." : "Generate Random"}
          </button>
          {randomMutation.data && (
            <pre className="mt-4 p-3 bg-neutral-50 rounded text-xs overflow-auto">
              {JSON.stringify(randomMutation.data, null, 2)}
            </pre>
          )}
        </div>

        <div className="bg-white border border-neutral-200 rounded-lg p-6">
          <h3 className="text-sm font-semibold text-neutral-900 mb-4">
            Complex Validation
          </h3>
          <p className="text-xs text-neutral-500 mb-4">
            POST /special/validate-email - Business logic validation
          </p>
          <button
            onClick={() =>
              validateMutation.mutate({ email: "test@example.com" })
            }
            disabled={validateMutation.isPending}
            className="w-full px-4 py-2 bg-neutral-900 text-white text-sm font-medium rounded hover:bg-neutral-800 disabled:opacity-50"
          >
            {validateMutation.isPending ? "Validating..." : "Validate Email"}
          </button>
          {validateMutation.data && (
            <pre className="mt-4 p-3 bg-neutral-50 rounded text-xs overflow-auto">
              {JSON.stringify(validateMutation.data, null, 2)}
            </pre>
          )}
        </div>
      </div>
    </section>
  );
}
