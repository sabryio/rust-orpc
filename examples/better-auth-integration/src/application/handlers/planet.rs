use crate::domain::models::planet::*;
use crate::infrastructure::auth::guard::Authenticated;
use crate::infrastructure::auth::middleware::BaseContext;
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

/// Protected handler: receives Authenticated wrapper with guaranteed session.
/// The Deref impl allows accessing planet_repo through authenticated.planet_repo.
///
/// SRP: Orchestrates planet creation, delegates to repository.
/// DIP: Depends on PlanetRepository trait, not concrete implementation.
pub async fn create_planet(
    authenticated: Authenticated,
    input: CreatePlanetInput,
) -> Result<Planet, OrpcError> {
    // authenticated.session.0.user.id() is available if needed for audit
    // authenticated.planet_repo is accessible via Deref
    authenticated.planet_repo.create(input).await
}
