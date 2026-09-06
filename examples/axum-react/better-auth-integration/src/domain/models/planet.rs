use rorpc::ZodTs;
use serde::{Deserialize, Serialize};

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
pub struct DeletePlanetInput {
    pub id: i32,
}

#[derive(Debug, Deserialize, Serialize, ZodTs)]
pub struct CreatePlanetInput {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, ZodTs)]
pub struct ListPlanetsPaginatedInput {
    pub limit: usize,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize, ZodTs)]
pub struct ListPlanetsPaginatedOutput {
    pub items: Vec<Planet>,
    pub next_page_param: Option<usize>,
}

#[derive(Debug, Serialize, ZodTs)]
pub struct EventData {
    pub message: String,
    pub count: u32,
}
