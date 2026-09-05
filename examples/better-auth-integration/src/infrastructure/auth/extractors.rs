use crate::infrastructure::auth::schema::AppAuthSchema;
use better_auth::integrations::axum;
use better_auth::prelude::AuthUser;

/// Required session extractor — returns 401 automatically if not authenticated.
pub type Session = axum::CurrentSession<AppAuthSchema>;

/// Optional session extractor — works for both authenticated and anonymous requests.
pub type OptionalSession = axum::OptionalSession<AppAuthSchema>;

/// Ergonomic helpers on the required session.
pub trait SessionExt {
    fn user_id(&self) -> String;
    fn user_email(&self) -> Option<&str>;
}

impl SessionExt for axum::CurrentSession<AppAuthSchema> {
    fn user_id(&self) -> String {
        self.user.id().to_string()
    }

    fn user_email(&self) -> Option<&str> {
        self.user.email()
    }
}
