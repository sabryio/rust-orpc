use axum::{
    extract::{Query, State},
    Json,
};

use crate::{
    application::errors::AppError,
    domain::models::planet::*,
    infrastructure::{auth::extractors::Session, context::AppState},
};

#[rorpc::get("/planet/list")]
pub async fn list_planets(State(state): State<AppState>) -> Result<Json<Vec<Planet>>, AppError> {
    state
        .planet_repo
        .list()
        .await
        .map(Json)
        .map_err(|e| AppError::Internal { msg: e.to_string() })
}

#[rorpc::get("/planet/list-paginated")]
pub async fn list_planets_paginated(
    State(state): State<AppState>,
    Query(input): Query<ListPlanetsPaginatedInput>,
) -> Result<Json<ListPlanetsPaginatedOutput>, AppError> {
    state
        .planet_repo
        .list_paginated(input)
        .await
        .map(Json)
        .map_err(|e| AppError::Internal { msg: e.to_string() })
}

#[rorpc::get("/planet/find")]
pub async fn find_planet(
    State(state): State<AppState>,
    Query(input): Query<FindPlanetInput>,
) -> Result<Json<Planet>, AppError> {
    state
        .planet_repo
        .find(input)
        .await
        .map(Json)
        .map_err(|_| AppError::NotFound)
}

/// Protected — `CurrentSession` returns 401 automatically if not authenticated.
#[rorpc::post("/planet/create")]
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
        .map_err(|e| AppError::Internal { msg: e.to_string() })
}

/// Protected — `CurrentSession` returns 401 automatically if not authenticated.
#[rorpc::delete("/planet/delete")]
pub async fn delete_planet(
    State(state): State<AppState>,
    _session: Session,
    Json(input): Json<DeletePlanetInput>,
) -> Result<Json<()>, AppError> {
    state
        .planet_repo
        .delete(input)
        .await
        .map(Json)
        .map_err(|_| AppError::NotFound)
}
