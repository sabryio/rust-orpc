use crate::domain::ports::planet_repository::PlanetRepository;
use std::sync::Arc;

/// User-defined context — only the fields you care about.
/// No session field needed! BetterAuthContext wraps this and manages the session.
#[derive(Clone)]
pub struct BaseContext {
    pub planet_repo: Arc<dyn PlanetRepository>,
}
