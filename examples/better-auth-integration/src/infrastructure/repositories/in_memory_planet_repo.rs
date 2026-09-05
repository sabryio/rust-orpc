use crate::domain::models::planet::*;
use crate::domain::ports::planet_repository::PlanetRepository;
use async_trait::async_trait;
use orpc_core::OrpcError;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Concrete implementation of PlanetRepository.
/// LSP: Can be substituted anywhere `dyn PlanetRepository` is expected.
pub struct InMemoryPlanetRepository {
    planets: Arc<RwLock<Vec<Planet>>>,
}

impl InMemoryPlanetRepository {
    pub fn new(initial_planets: Vec<Planet>) -> Self {
        Self {
            planets: Arc::new(RwLock::new(initial_planets)),
        }
    }
}

#[async_trait]
impl PlanetRepository for InMemoryPlanetRepository {
    async fn list(&self) -> Result<Vec<Planet>, OrpcError> {
        Ok(self.planets.read().await.clone())
    }

    async fn list_paginated(
        &self,
        input: ListPlanetsPaginatedInput,
    ) -> Result<ListPlanetsPaginatedOutput, OrpcError> {
        let planets = self.planets.read().await;
        let offset = input.offset.unwrap_or(0);
        let items: Vec<Planet> = planets
            .iter()
            .skip(offset)
            .take(input.limit)
            .cloned()
            .collect();

        let next_page_param = if offset + input.limit < planets.len() {
            Some(offset + input.limit)
        } else {
            None
        };

        Ok(ListPlanetsPaginatedOutput {
            items,
            next_page_param,
        })
    }

    async fn find(&self, input: FindPlanetInput) -> Result<Planet, OrpcError> {
        self.planets
            .read()
            .await
            .iter()
            .find(|p| p.id == input.id)
            .cloned()
            .ok_or_else(|| OrpcError::not_found(format!("Planet {} not found", input.id)))
    }

    async fn create(&self, input: CreatePlanetInput) -> Result<Planet, OrpcError> {
        if input.name.trim().is_empty() {
            return Err(OrpcError::bad_request("Planet name cannot be empty"));
        }

        let mut planets = self.planets.write().await;
        let new_id = planets.iter().map(|p| p.id).max().unwrap_or(0) + 1;
        let planet = Planet {
            id: new_id,
            name: input.name,
            description: input.description,
        };
        planets.push(planet.clone());
        Ok(planet)
    }
}

/// Helper to generate sample data.
pub fn sample_planets() -> Vec<Planet> {
    vec![
        Planet {
            id: 1,
            name: "Mercury".to_string(),
            description: Some("The smallest planet".to_string()),
        },
        Planet {
            id: 2,
            name: "Venus".to_string(),
            description: Some("The hottest planet".to_string()),
        },
        Planet {
            id: 3,
            name: "Earth".to_string(),
            description: Some("The blue planet".to_string()),
        },
        Planet {
            id: 4,
            name: "Mars".to_string(),
            description: Some("The red planet".to_string()),
        },
        Planet {
            id: 5,
            name: "Jupiter".to_string(),
            description: Some("The largest planet".to_string()),
        },
    ]
}
