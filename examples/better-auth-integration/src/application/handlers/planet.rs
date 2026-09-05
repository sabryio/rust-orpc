use crate::{
    domain::models::planet::*,
    infrastructure::{auth::schema::AppAuthSchema, context::BaseContext},
};
use orpc_axum::BetterAuthSession;
use orpc_core::{OrpcContext, OrpcError, OrpcJson};

pub async fn list_planets(
    OrpcContext(ctx): OrpcContext<BaseContext>,
) -> Result<Vec<Planet>, OrpcError> {
    ctx.planet_repo.list().await
}

pub async fn list_planets_paginated(
    OrpcContext(ctx): OrpcContext<BaseContext>,
    OrpcJson(input): OrpcJson<ListPlanetsPaginatedInput>,
) -> Result<ListPlanetsPaginatedOutput, OrpcError> {
    ctx.planet_repo.list_paginated(input).await
}

pub async fn find_planet(
    OrpcContext(ctx): OrpcContext<BaseContext>,
    OrpcJson(input): OrpcJson<FindPlanetInput>,
) -> Result<Planet, OrpcError> {
    ctx.planet_repo.find(input).await
}

/// Protected handler — BetterAuthSession extractor ensures authentication
pub async fn create_planet(
    OrpcContext(ctx): OrpcContext<BaseContext>,
    BetterAuthSession(_session): BetterAuthSession<AppAuthSchema>,
    OrpcJson(input): OrpcJson<CreatePlanetInput>,
) -> Result<Planet, OrpcError> {
    ctx.planet_repo.create(input).await
}
