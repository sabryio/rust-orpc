use axum::{extract::State, Json};
use orpc::orpc;

use crate::{
    application::errors::AppError,
    domain::models::planet::*,
    infrastructure::{auth::extractors::Session, context::AppState},
};

#[orpc(method = "POST", path = "/planet/list")]
pub async fn list_planets(State(state): State<AppState>) -> Result<Json<Vec<Planet>>, AppError> {
    state
        .planet_repo
        .list()
        .await
        .map(Json)
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[orpc(method = "POST", path = "/planet/list-paginated")]
pub async fn list_planets_paginated(
    State(state): State<AppState>,
    Json(input): Json<ListPlanetsPaginatedInput>,
) -> Result<Json<ListPlanetsPaginatedOutput>, AppError> {
    state
        .planet_repo
        .list_paginated(input)
        .await
        .map(Json)
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[orpc(method = "POST", path = "/planet/find")]
pub async fn find_planet(
    State(state): State<AppState>,
    Json(input): Json<FindPlanetInput>,
) -> Result<Json<Planet>, AppError> {
    state
        .planet_repo
        .find(input)
        .await
        .map(Json)
        .map_err(|_| AppError::NotFound)
}

/// Protected — `CurrentSession` returns 401 automatically if not authenticated.
#[orpc(method = "POST", path = "/planet/create")]
pub async fn create_planet(
    State(state): State<AppState>,
    _session: Session,
    Json(input): Json<CreatePlanetInput>,
) -> Result<Json<Planet>, AppError> {
    state
        .planet_repo
        .create(input)
        .await
        .map(Json)
        .map_err(|e| AppError::Internal(e.to_string()))
}
