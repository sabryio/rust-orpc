use crate::domain::models::planet::*;
use crate::infrastructure::auth::middleware::{AuthContext, BaseContext};
use orpc_core::OrpcError;

pub async fn list_planets(ctx: BaseContext, _: ()) -> Result<Vec<Planet>, OrpcError> {
    ctx.planet_repo.list().await
}

pub async fn list_planets_paginated(
    ctx: BaseContext,
    input: ListPlanetsPaginatedInput,
) -> Result<ListPlanetsPaginatedOutput, OrpcError> {
    ctx.planet_repo.list_paginated(input).await
}

pub async fn find_planet(ctx: BaseContext, input: FindPlanetInput) -> Result<Planet, OrpcError> {
    ctx.planet_repo.find(input).await
}

pub async fn create_planet(
    ctx: AuthContext,
    input: CreatePlanetInput,
) -> Result<Planet, OrpcError> {
    // SRP: Handler only orchestrates. Validation and persistence are delegated.
    // ISP: `ctx.user` is directly accessible, no Option unwrapping required.
    ctx.planet_repo.create(input).await
}
