use crate::domain::models::planet::*;
use crate::domain::ports::planet_repository::{PlanetRepository, RepoError};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

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
    async fn list(&self) -> Result<Vec<Planet>, RepoError> {
        Ok(self.planets.read().await.clone())
    }

    async fn list_paginated(
        &self,
        input: ListPlanetsPaginatedInput,
    ) -> Result<ListPlanetsPaginatedOutput, RepoError> {
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

    async fn find(&self, input: FindPlanetInput) -> Result<Planet, RepoError> {
        self.planets
            .read()
            .await
            .iter()
            .find(|p| p.id == input.id)
            .cloned()
            .ok_or_else(|| format!("Planet {} not found", input.id).into())
    }

    async fn create(&self, input: CreatePlanetInput) -> Result<Planet, RepoError> {
        if input.name.trim().is_empty() {
            return Err("Planet name cannot be empty".into());
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

pub fn sample_planets() -> Vec<Planet> {
    vec![
        Planet {
            id: 1,
            name: "Mercury".into(),
            description: Some("The smallest planet".into()),
        },
        Planet {
            id: 2,
            name: "Venus".into(),
            description: Some("The hottest planet".into()),
        },
        Planet {
            id: 3,
            name: "Earth".into(),
            description: Some("The blue planet".into()),
        },
        Planet {
            id: 4,
            name: "Mars".into(),
            description: Some("The red planet".into()),
        },
        Planet {
            id: 5,
            name: "Jupiter".into(),
            description: Some("The largest planet".into()),
        },
        Planet {
            id: 6,
            name: "Saturn".into(),
            description: Some("The ringed planet".into()),
        },
        Planet {
            id: 7,
            name: "Uranus".into(),
            description: Some("The ice giant that rotates on its side".into()),
        },
        Planet {
            id: 8,
            name: "Neptune".into(),
            description: Some("The windiest planet".into()),
        },
        Planet {
            id: 9,
            name: "Pluto".into(),
            description: Some("The dwarf planet at the edge of the solar system".into()),
        },
        Planet {
            id: 10,
            name: "Ceres".into(),
            description: Some("The largest object in the asteroid belt".into()),
        },
        Planet {
            id: 11,
            name: "Eris".into(),
            description: Some("The most massive known dwarf planet".into()),
        },
        Planet {
            id: 12,
            name: "Makemake".into(),
            description: Some("A dwarf planet in the Kuiper belt".into()),
        },
    ]
}
