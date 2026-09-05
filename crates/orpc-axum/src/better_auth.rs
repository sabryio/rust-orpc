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
//! 1. Define your context with only the fields you need:
//!
//! ```rust,ignore
//! #[derive(Clone)]
//! struct BaseContext {
//!     planet_repo: Arc<dyn PlanetRepository>,
//!     // No session field needed!
//! }
//! ```
//!
//! 2. Wire up — `BetterAuthContext<Schema, Ctx>` wraps your context and handles the session:
//!
//! ```rust,ignore
//! let app = orpc_router
//!     .into_axum_router_with_better_auth(base_ctx)
//!     .with_better_auth(auth);
//! ```
//!
//! 3. In handlers, session is available via `.session()`:
//!
//! ```rust,ignore
//! async fn get_profile(
//!     ctx: BetterAuthContext<AppAuthSchema, BaseContext>,
//!     _: (),
//! ) -> Result<Json<Value>, OrpcError> {
//!     let session = ctx.require_session()?; // Returns Err if not authenticated
//!     Ok(Json(json!({ "email": session.user.email() })))
//! }
//! ```

use axum::{middleware, Router};
use better_auth::{
    integrations::axum::{CurrentSession, OptionalSession},
    AuthSchema, BetterAuth,
};
use std::sync::Arc;

/// Wraps a user-defined context with a Better-Auth session slot.
///
/// Users define only their own fields. The session is managed here automatically
/// by `.with_better_auth()` — no session field needed in the inner context.
///
/// Access the inner context via `Deref`, and the session via `.session()` or `.require_session()`.
pub struct BetterAuthContext<Schema, Ctx>
where
    Schema: AuthSchema + 'static,
    Ctx: Clone + Send + Sync + 'static,
{
    pub inner: Ctx,
    session: Arc<OptionalSession<Schema>>,
}

impl<Schema, Ctx> Clone for BetterAuthContext<Schema, Ctx>
where
    Schema: AuthSchema + 'static,
    Ctx: Clone + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            session: Arc::clone(&self.session),
        }
    }
}

impl<Schema, Ctx> BetterAuthContext<Schema, Ctx>
where
    Schema: AuthSchema + 'static,
    Ctx: Clone + Send + Sync + 'static,
{
    /// Create a new BetterAuthContext with no session (used as the base context).
    pub fn new(inner: Ctx) -> Self {
        Self {
            inner,
            session: Arc::new(OptionalSession(None)),
        }
    }

    /// Returns the session if authenticated, otherwise `None`.
    pub fn session(&self) -> Option<&CurrentSession<Schema>> {
        self.session.0.as_ref()
    }

    /// Returns the session or an `OrpcError::unauthorized` if not authenticated.
    pub fn require_session(&self) -> Result<&CurrentSession<Schema>, orpc_core::OrpcError> {
        self.session
            .0
            .as_ref()
            .ok_or_else(|| orpc_core::OrpcError::unauthorized("Authentication required"))
    }

    /// Returns true if a session exists.
    pub fn is_authenticated(&self) -> bool {
        self.session.0.is_some()
    }
}

impl<Schema, Ctx> std::ops::Deref for BetterAuthContext<Schema, Ctx>
where
    Schema: AuthSchema + 'static,
    Ctx: Clone + Send + Sync + 'static,
{
    type Target = Ctx;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Implement this trait on your context to opt into automatic session injection.
///
/// The associated type `Schema` means callers never need to write `::<AppAuthSchema>` —
/// the compiler infers it from the `impl`.
pub trait WithBetterAuth: Clone + Send + Sync + 'static {
    /// The Better-Auth schema — inferred from the impl, never written by callers.
    type Schema: AuthSchema + 'static;

    /// Injects the extracted Better-Auth session into this context.
    /// Called automatically per-request.
    fn inject_session(&mut self, session: Arc<OptionalSession<Self::Schema>>);
}

/// Blanket implementation of `WithBetterAuth` for `BetterAuthContext<Schema, Ctx>`.
///
/// This means any `BetterAuthContext` automatically supports session injection
/// without the user writing any boilerplate.
impl<Schema, Ctx> WithBetterAuth for BetterAuthContext<Schema, Ctx>
where
    Schema: AuthSchema + 'static,
    Ctx: Clone + Send + Sync + 'static,
{
    type Schema = Schema;

    fn inject_session(&mut self, session: Arc<OptionalSession<Schema>>) {
        self.session = session;
    }
}

/// Extension trait added to `axum::Router` when `better-auth` feature is enabled.
pub trait BetterAuthExt<Schema>
where
    Schema: AuthSchema + 'static,
{
    /// Wire Better-Auth into this router.
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
