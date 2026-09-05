//! Better-Auth integration for orpc-axum.
//!
//! Enable with the `better-auth` feature flag:
//!
//! ```toml
//! [dependencies]
//! orpc-axum = { version = "0.1", features = ["better-auth"] }
//! ```
//!
//! ## Usage
//!
//! 1. Implement `WithBetterAuth` on your context (associated type — no turbofish!):
//!
//! ```rust,ignore
//! impl WithBetterAuth for BaseContext {
//!     type Schema = AppAuthSchema;
//!
//!     fn inject_session(&mut self, session: Arc<OptionalSession<AppAuthSchema>>) {
//!         self.session = session;
//!     }
//! }
//! ```
//!
//! 2. Wire up — schema is inferred, no turbofish needed:
//!
//! ```rust,ignore
//! let app = orpc_router
//!     .into_axum_router_with_better_auth(base_ctx) // inferred from BaseContext impl
//!     .with_better_auth(auth);
//! ```

use axum::{middleware, Router};
use better_auth::{integrations::axum::OptionalSession, AuthSchema, BetterAuth};
use std::sync::Arc;

/// Implement this trait on your context to opt into automatic session injection.
///
/// The associated type `Schema` means callers never need to write `::<AppAuthSchema>` —
/// the compiler infers it from the `impl`.
///
/// # Example
///
/// ```rust,ignore
/// impl WithBetterAuth for BaseContext {
///     type Schema = AppAuthSchema;
///
///     fn inject_session(&mut self, session: Arc<OptionalSession<AppAuthSchema>>) {
///         self.session = session;
///     }
/// }
/// ```
pub trait WithBetterAuth: Clone + Send + Sync + 'static {
    /// The Better-Auth schema — inferred from the impl, never written by callers.
    type Schema: AuthSchema + 'static;

    /// Injects the extracted Better-Auth session into this context.
    /// Called automatically per-request.
    fn inject_session(&mut self, session: Arc<OptionalSession<Self::Schema>>);
}

/// Extension trait added to `axum::Router` when `better-auth` feature is enabled.
pub trait BetterAuthExt<Schema>
where
    Schema: AuthSchema + 'static,
{
    /// Wire Better-Auth into this router.
    ///
    /// Adds session extraction middleware — the session is stored in request
    /// extensions as `Arc<OptionalSession<Schema>>` and automatically injected
    /// into contexts that implement `WithBetterAuth`.
    fn with_better_auth(self, auth: Arc<BetterAuth<Schema>>) -> Router;
}

impl<Schema> BetterAuthExt<Schema> for Router
where
    Schema: AuthSchema + 'static,
{
    fn with_better_auth(self, auth: Arc<BetterAuth<Schema>>) -> Router {
        self.layer(middleware::from_fn_with_state(
            auth,
            session_extraction_layer::<Schema>,
        ))
    }
}

/// Internal middleware: extracts Better-Auth session into request extensions.
pub async fn session_extraction_layer<Schema>(
    session: OptionalSession<Schema>,
    mut req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response
where
    Schema: AuthSchema,
{
    req.extensions_mut().insert(Arc::new(session));
    next.run(req).await
}
