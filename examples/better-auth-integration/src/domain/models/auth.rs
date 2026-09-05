use crate::infrastructure::auth::schema::AppAuthSchema;
use better_auth::integrations::axum::{CurrentSession, OptionalSession};
use std::ops::Deref;

/// Newtype wrapper around Better-Auth's OptionalSession.
///
/// This type does NOT implement Clone (since OptionalSession doesn't),
/// so it must be wrapped in Arc when used in BaseContext.
///
/// DIP: Depends on Better-Auth's session abstraction, not concrete session storage.
pub struct AuthenticatedUser(OptionalSession<AppAuthSchema>);

impl Deref for AuthenticatedUser {
    type Target = OptionalSession<AppAuthSchema>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AuthenticatedUser {
    /// Creates a new AuthenticatedUser from an OptionalSession
    pub fn new(session: OptionalSession<AppAuthSchema>) -> Self {
        Self(session)
    }

    /// Returns true if a session exists
    pub fn is_authenticated(&self) -> bool {
        self.0 .0.is_some()
    }

    /// Returns a reference to the inner session if it exists
    pub fn session(&self) -> Option<&CurrentSession<AppAuthSchema>> {
        self.0 .0.as_ref()
    }
}
