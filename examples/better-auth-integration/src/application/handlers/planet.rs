use crate::domain::models::planet::*;
use crate::infrastructure::auth::guard::AppContext;
use orpc_core::{OrpcContext, OrpcError, OrpcJson};

pub async fn list_planets(
    OrpcContext(ctx): OrpcContext<AppContext>,
) -> Result<Vec<Planet>, OrpcError> {
    ctx.planet_repo.list().await
}

pub async fn list_planets_paginated(
    OrpcContext(ctx): OrpcContext<AppContext>,
    OrpcJson(input): OrpcJson<ListPlanetsPaginatedInput>,
) -> Result<ListPlanetsPaginatedOutput, OrpcError> {
    ctx.planet_repo.list_paginated(input).await
}

pub async fn find_planet(
    OrpcContext(ctx): OrpcContext<AppContext>,
    OrpcJson(input): OrpcJson<FindPlanetInput>,
) -> Result<Planet, OrpcError> {
    ctx.planet_repo.find(input).await
}

/// Protected handler — require_auth guard ensures session exists before this runs.
pub async fn create_planet(
    OrpcContext(ctx): OrpcContext<AppContext>,
    OrpcJson(input): OrpcJson<CreatePlanetInput>,
) -> Result<Planet, OrpcError> {
    ctx.planet_repo.create(input).await
}
