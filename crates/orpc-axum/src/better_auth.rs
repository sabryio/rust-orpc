//! Better-Auth integration for orpc-axum.
//!
//! Enable with the `better-auth` feature flag:
//!
//! ```toml
//! [dependencies]
//! orpc-axum = { version = "0.1", features = ["better-auth"] }
//! ```
//!
//! ## Usage with Extractors (Recommended)
//!
//! 1. Define your context with only the fields you need:
//!
//! ```rust,ignore
//! #[derive(Clone)]
//! struct BaseContext {
//!     planet_repo: Arc<dyn PlanetRepository>,
//! }
//! ```
//!
//! 2. Use extractors in handlers:
//!
//! ```rust,ignore
//! use orpc_core::{OrpcContext, OrpcError};
//! use orpc_axum::BetterAuthSession;
//!
//! async fn get_profile(
//!     OrpcContext(ctx): OrpcContext<BaseContext>,
//!     BetterAuthSession(session): BetterAuthSession<AppAuthSchema>,
//! ) -> Result<Output, OrpcError> {
//!     // session is guaranteed to exist (401 if not authenticated)
//!     Ok(Output { email: session.user.email().to_string() })
//! }
//! ```
//!
//! 3. Wire up the router:
//!
//! ```rust,ignore
//! let app = orpc_router
//!     .into_axum_router(base_ctx)
//!     .with_better_auth(auth);
//! ```

use async_trait::async_trait;
use axum::{middleware, Router};
use better_auth::{integrations::axum::OptionalSession, AuthSchema, BetterAuth};
use orpc_core::{FromOrpcRequest, OrpcError};
use std::sync::Arc;

/// Extract a required Better-Auth session from the request.
///
/// Returns `OrpcError::unauthorized` (401) if the user is not authenticated.
///
/// The session is wrapped in `Arc` since `CurrentSession` doesn't implement `Clone`.
///
/// # Example
///
/// ```rust,ignore
/// use orpc_axum::BetterAuthSession;
///
/// async fn handler(
///     BetterAuthSession(session): BetterAuthSession<AppAuthSchema>,
/// ) -> Result<Output, OrpcError> {
///     let email = session.user.email();
///     Ok(Output { email: email.to_string() })
/// }
/// ```
pub struct BetterAuthSession<Schema: AuthSchema>(pub Arc<OptionalSession<Schema>>);

#[async_trait]
impl<Ctx, Schema> FromOrpcRequest<Ctx> for BetterAuthSession<Schema>
where
    Ctx: Send + Sync + 'static,
    Schema: AuthSchema + 'static,
{
    async fn from_request(
        ctx: Ctx,
        input: serde_json::Value,
        extensions: Option<&Arc<dyn std::any::Any + Send + Sync>>,
    ) -> Result<(Self, Ctx, serde_json::Value), OrpcError> {
        let extensions = extensions.ok_or_else(|| {
            OrpcError::internal("Extensions not available (not using Axum transport?)")
        })?;

        // Downcast to Axum Extensions
        let axum_extensions = extensions
            .downcast_ref::<axum::http::Extensions>()
            .ok_or_else(|| {
                OrpcError::internal("Failed to downcast extensions to Axum Extensions")
            })?;

        // Extract OptionalSession from extensions
        let optional_session = axum_extensions
            .get::<Arc<OptionalSession<Schema>>>()
            .ok_or_else(|| {
                OrpcError::internal(
                    "Better-Auth session not found in extensions. Did you call .with_better_auth()?",
                )
            })?;

        // Require session to be present (clone the Arc, not the session)
        if optional_session.0.is_some() {
            Ok((BetterAuthSession(Arc::clone(optional_session)), ctx, input))
        } else {
            Err(OrpcError::unauthorized("Authentication required"))
        }
    }
}

/// Extract an optional Better-Auth session from the request.
///
/// Returns `None` if the user is not authenticated (does not return an error).
///
/// The session container is wrapped in `Arc`.
///
/// # Example
///
/// ```rust,ignore
/// use orpc_axum::OptionalBetterAuthSession;
///
/// async fn handler(
///     OptionalBetterAuthSession(session): OptionalBetterAuthSession<AppAuthSchema>,
/// ) -> Result<Output, OrpcError> {
///     let email = session
///         .0
///         .as_ref()
///         .map(|s| s.user.email().to_string())
///         .unwrap_or_else(|| "anonymous".to_string());
///     Ok(Output { email })
/// }
/// ```
pub struct OptionalBetterAuthSession<Schema: AuthSchema>(pub Arc<OptionalSession<Schema>>);

#[async_trait]
impl<Ctx, Schema> FromOrpcRequest<Ctx> for OptionalBetterAuthSession<Schema>
where
    Ctx: Send + Sync + 'static,
    Schema: AuthSchema + 'static,
{
    async fn from_request(
        ctx: Ctx,
        input: serde_json::Value,
        extensions: Option<&Arc<dyn std::any::Any + Send + Sync>>,
    ) -> Result<(Self, Ctx, serde_json::Value), OrpcError> {
        let extensions = extensions.ok_or_else(|| {
            OrpcError::internal("Extensions not available (not using Axum transport?)")
        })?;

        // Downcast to Axum Extensions
        let axum_extensions = extensions
            .downcast_ref::<axum::http::Extensions>()
            .ok_or_else(|| {
                OrpcError::internal("Failed to downcast extensions to Axum Extensions")
            })?;

        // Extract OptionalSession from extensions
        let optional_session = axum_extensions
            .get::<Arc<OptionalSession<Schema>>>()
            .ok_or_else(|| {
                OrpcError::internal(
                    "Better-Auth session not found in extensions. Did you call .with_better_auth()?",
                )
            })?;

        // Return the optional session (clone the Arc)
        Ok((
            OptionalBetterAuthSession(Arc::clone(optional_session)),
            ctx,
            input,
        ))
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
