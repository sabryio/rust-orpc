use axum::{extract::State, Json};
use orpc::orpc;

use crate::errors::AppError;
use crate::models::{CreatePlanetInput, Db, FindPlanetInput, Planet};

/// List all planets.
///
/// Annotated with `#[orpc]` — the handler is a plain Axum handler.
/// The macro registers its metadata so `orpc::router()` can auto-build
/// the route and `orpc::generate_contract()` can generate TypeScript.
#[orpc(method = "POST", path = "/planet/list")]
pub async fn list_planets(State(db): State<Db>) -> Json<Vec<Planet>> {
    Json(db.list())
}

/// Find a planet by ID.
#[orpc(method = "POST", path = "/planet/find")]
pub async fn find_planet(
    State(db): State<Db>,
    Json(input): Json<FindPlanetInput>,
) -> Result<Json<Planet>, AppError> {
    db.find(input.id).map(Json).ok_or(AppError::NotFound)
}

/// Create a new planet.
#[orpc(method = "POST", path = "/planet/create")]
pub async fn create_planet(
    State(mut db): State<Db>,
    Json(input): Json<CreatePlanetInput>,
) -> Json<Planet> {
    Json(db.create(input))
}
