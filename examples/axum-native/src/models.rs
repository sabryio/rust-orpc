use orpc::ZodTs;
use serde::{Deserialize, Serialize};

/// A planet in the solar system.
///
/// `#[derive(ZodTs)]` generates TypeScript Zod schema automatically.
/// The `#[orpc]` macro on handlers registers the schema via `inventory::submit!`
/// — no manual call to `generate_contract()` config needed.
#[derive(Debug, Clone, Serialize, Deserialize, ZodTs)]
pub struct Planet {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, ZodTs)]
pub struct FindPlanetInput {
    pub id: i32,
}

#[derive(Debug, Deserialize, Serialize, ZodTs)]
pub struct CreatePlanetInput {
    pub name: String,
    pub description: Option<String>,
}

/// In-memory database for demonstration purposes.
#[derive(Clone)]
pub struct Db {
    planets: Vec<Planet>,
}

impl Db {
    pub fn new() -> Self {
        Self {
            planets: vec![
                Planet {
                    id: 1,
                    name: "Mercury".into(),
                    description: Some("Closest to the Sun".into()),
                },
                Planet {
                    id: 2,
                    name: "Venus".into(),
                    description: Some("Brightest planet".into()),
                },
                Planet {
                    id: 3,
                    name: "Earth".into(),
                    description: Some("Our home".into()),
                },
            ],
        }
    }

    pub fn list(&self) -> Vec<Planet> {
        self.planets.clone()
    }

    pub fn find(&self, id: i32) -> Option<Planet> {
        self.planets.iter().find(|p| p.id == id).cloned()
    }

    pub fn create(&mut self, input: CreatePlanetInput) -> Planet {
        let id = self.planets.len() as i32 + 1;
        let planet = Planet {
            id,
            name: input.name,
            description: input.description,
        };
        self.planets.push(planet.clone());
        planet
    }
}
