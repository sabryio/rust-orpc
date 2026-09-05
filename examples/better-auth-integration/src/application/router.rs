use crate::application::handlers::{ping, planet, profile, stream};
use crate::domain::models::planet::*;
use crate::infrastructure::auth::middleware::{require_auth, BaseContext};
use orpc_core::{openapi, os, router, AsyncIterator, HttpMethod};

/// Assembles the orpc router.
/// SRP: Sole responsibility is mapping routes to handlers and applying middleware.
pub fn build_orpc_router() -> impl orpc_axum::AxumRouter<BaseContext> {
    // Define once — base procedure with auth middleware already applied
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

            create: protected.clone()
                .route(HttpMethod::Post, "/planet/create")
                .input::<CreatePlanetInput>()
                .output::<Planet>()
                .handler(planet::create_planet),
        },

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
