use crate::domain::models::planet::{
    CreatePlanetInput, FindPlanetInput, ListPlanetsPaginatedInput, ListPlanetsPaginatedOutput,
    Planet,
};
use async_trait::async_trait;

pub type RepoError = Box<dyn std::error::Error + Send + Sync>;

#[async_trait]
pub trait PlanetRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<Planet>, RepoError>;
    async fn list_paginated(
        &self,
        input: ListPlanetsPaginatedInput,
    ) -> Result<ListPlanetsPaginatedOutput, RepoError>;
    async fn find(&self, input: FindPlanetInput) -> Result<Planet, RepoError>;
    async fn create(&self, input: CreatePlanetInput) -> Result<Planet, RepoError>;
}
