use crate::domain::ports::planet_repository::PlanetRepository;
use std::sync::Arc;

/// Base application context — user-defined fields only.
///
/// No Better-Auth session or Axum-specific state.
/// Authentication is handled via extractors in handlers.
#[derive(Clone)]
pub struct BaseContext {
    pub planet_repo: Arc<dyn PlanetRepository>,
}
