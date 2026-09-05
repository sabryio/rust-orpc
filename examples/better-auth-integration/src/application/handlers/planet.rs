use crate::domain::models::planet::*;
use crate::infrastructure::auth::guard::AppContext;
use orpc_core::OrpcError;

pub async fn list_planets(ctx: AppContext, _: ()) -> Result<Vec<Planet>, OrpcError> {
    ctx.planet_repo.list().await
}

pub async fn list_planets_paginated(
    ctx: AppContext,
    input: ListPlanetsPaginatedInput,
) -> Result<ListPlanetsPaginatedOutput, OrpcError> {
    ctx.planet_repo.list_paginated(input).await
}

pub async fn find_planet(ctx: AppContext, input: FindPlanetInput) -> Result<Planet, OrpcError> {
    ctx.planet_repo.find(input).await
}

/// Protected handler — require_auth guard ensures session exists before this runs.
pub async fn create_planet(ctx: AppContext, input: CreatePlanetInput) -> Result<Planet, OrpcError> {
    ctx.planet_repo.create(input).await
}
