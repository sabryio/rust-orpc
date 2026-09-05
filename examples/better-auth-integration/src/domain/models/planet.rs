use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Planet {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FindPlanetInput {
    pub id: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreatePlanetInput {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ListPlanetsPaginatedInput {
    pub limit: usize,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ListPlanetsPaginatedOutput {
    pub items: Vec<Planet>,
    #[serde(rename = "nextPageParam")]
    pub next_page_param: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct StreamEvent {
    pub message: String,
    pub count: u32,
}
