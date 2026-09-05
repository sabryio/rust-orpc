use crate::application::handlers::{ping, planet, profile, stream};
use crate::domain::models::planet::*;
use crate::infrastructure::auth::guard::require_auth;
use crate::infrastructure::auth::middleware::BaseContext;
use orpc_core::{openapi, os, router, AsyncIterator, HttpMethod};

/// Assembles the orpc router using the guard-based API.
///
/// Guards use Better-Auth's native types:
/// - `BaseContext` → `Authenticated` (via require_auth guard)
/// - `Authenticated` combines BaseContext + CurrentSession<AppAuthSchema>
/// - Handlers receive guaranteed sessions without Option unwrapping
pub fn build_orpc_router() -> impl orpc_axum::AxumRouter<BaseContext> {
    // Define protected procedure builder with auth guard
    // Handlers using this will receive Authenticated instead of BaseContext
    let protected = os().context::<BaseContext>().use_middleware(require_auth);

    router! {
        ping: os()
            .context::<BaseContext>()
            .meta(openapi!{ method: "POST", path: "/ping" })
            .output::<String>()
            .handler(ping::ping),

        planet: {
            list: os()
                .context::<BaseContext>()
                .meta(openapi!{ method: "POST", path: "/planet/list" })
                .output::<Vec<Planet>>()
                .handler(planet::list_planets),

            listPaginated: os()
                .context::<BaseContext>()
                .route(HttpMethod::Post, "/planet/list-paginated")
                .input::<ListPlanetsPaginatedInput>()
                .output::<ListPlanetsPaginatedOutput>()
                .handler(planet::list_planets_paginated),

            find: os()
                .context::<BaseContext>()
                .meta(openapi!{ method: "POST", path: "/planet/find" })
                .input::<FindPlanetInput>()
                .output::<Planet>()
                .handler(planet::find_planet),

            // Protected route: handler receives Authenticated<BaseContext>
            create: protected.clone()
                .route(HttpMethod::Post, "/planet/create")
                .input::<CreatePlanetInput>()
                .output::<Planet>()
                .handler(planet::create_planet),
        },

        // Protected route: handler receives Authenticated<BaseContext>
        profile: protected.clone()
            .route(HttpMethod::Post, "/profile")
            .output::<serde_json::Value>()
            .handler(profile::get_profile),

        stream: os()
            .context::<BaseContext>()
            .route(HttpMethod::Post, "/stream")
            .output::<AsyncIterator<StreamEvent>>()
            .handler(stream::stream_events),

        stream_async: os()
            .context::<BaseContext>()
            .route(HttpMethod::Post, "/stream-async")
            .output::<AsyncIterator<StreamEvent>>()
            .handler(stream::stream_events_async),
    }
}
