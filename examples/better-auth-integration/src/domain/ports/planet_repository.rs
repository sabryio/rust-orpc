use crate::domain::models::planet::{
    CreatePlanetInput, FindPlanetInput, ListPlanetsPaginatedInput, ListPlanetsPaginatedOutput,
    Planet,
};
use async_trait::async_trait;
use orpc_core::OrpcError;

#[async_trait]
pub trait PlanetRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<Planet>, OrpcError>;
    async fn list_paginated(
        &self,
        input: ListPlanetsPaginatedInput,
    ) -> Result<ListPlanetsPaginatedOutput, OrpcError>;
    async fn find(&self, input: FindPlanetInput) -> Result<Planet, OrpcError>;
    async fn create(&self, input: CreatePlanetInput) -> Result<Planet, OrpcError>;
}
