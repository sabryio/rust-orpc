use crate::infrastructure::auth::schema::AppAuthSchema;
use better_auth::{integrations::axum::CurrentSession, prelude::AuthUser};
use orpc_axum::{
    BetterAuthSession as GenericSession, OptionalBetterAuthSession as GenericOptionalSession,
};

/// Type alias for authenticated session extractor in this app.
///
/// Use this in handlers that require authentication:
/// ```rust,ignore
/// pub async fn protected_handler(
///     session: Session,
/// ) -> Result<Output, OrpcError> {
///     let user_id = session.user_id();
/// }
/// ```
pub type Session = GenericSession<AppAuthSchema>;

/// Extension trait for session ergonomics.
pub trait SessionExt {
    /// Get the current session (guaranteed to exist for `BetterAuthSession`).
    fn current(&self) -> &CurrentSession<AppAuthSchema>;

    /// Get the user ID as a String.
    fn user_id(&self) -> String;

    /// Get the user email.
    fn user_email(&self) -> Option<&str>;
}

impl SessionExt for GenericSession<AppAuthSchema> {
    fn current(&self) -> &CurrentSession<AppAuthSchema> {
        self.0
            .as_ref()
            .0
            .as_ref()
            .expect("BetterAuthSession guarantees session exists")
    }

    fn user_id(&self) -> String {
        self.current().user.id().to_string()
    }

    fn user_email(&self) -> Option<&str> {
        self.current().user.email()
    }
}

/// Type alias for optional session extractor in this app.
///
/// Use this in handlers that work with or without authentication:
/// ```rust,ignore
/// pub async fn flexible_handler(
///     OptionalSession(session): OptionalSession,
/// ) -> Result<Output, OrpcError> {
///     if let Some(current) = session.0.as_ref() {
///         // authenticated
///     }
/// }
/// ```
pub type OptionalSession = GenericOptionalSession<AppAuthSchema>;
