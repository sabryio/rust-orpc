//! Procedure definitions and type-erased dispatch.

use crate::openapi::OpenApiMeta;
use crate::route::RouteMetadata;
use crate::OrpcError;
use async_trait::async_trait;
use futures_core::Stream;
use serde_json::Value;
use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Output kind for procedure results — either a single value or a stream.
pub enum OutputKind {
    /// Single JSON value
    Single(Value),
    /// Stream of JSON values
    Stream(Pin<Box<dyn Stream<Item = Value> + Send>>),
}

impl std::fmt::Debug for OutputKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputKind::Single(v) => f.debug_tuple("Single").field(v).finish(),
            OutputKind::Stream(_) => f.debug_tuple("Stream").field(&"<stream>").finish(),
        }
    }
}

/// Type-erased trait for runtime procedure dispatch.
#[async_trait]
pub trait ProcedureHandler<Ctx>: Send + Sync {
    async fn call(
        &self,
        ctx: Ctx,
        input: Value,
        extensions: Option<&Arc<dyn Any + Send + Sync>>,
    ) -> Result<OutputKind, OrpcError>;
    fn route_metadata(&self) -> &RouteMetadata;
}

type HandlerFn<Ctx, In, Out> = Arc<
    dyn Fn(Ctx, In) -> Pin<Box<dyn Future<Output = Result<Out, OrpcError>> + Send>> + Send + Sync,
>;

/// A typed RPC procedure with compile-time guarantees.
///
/// Carries both the handler and the route metadata declared via `.route()`.
/// The `route` field is public so transport adapters (Axum, Tauri) can read it.
pub struct Procedure<Ctx, In, Out> {
    pub(crate) handler: HandlerFn<Ctx, In, Out>,
    /// Route metadata declared via `.route()` — read by transport adapters.
    pub route: RouteMetadata,
    /// OpenAPI metadata for richer routing information.
    pub openapi_meta: OpenApiMeta,
}

impl<Ctx, In, Out> Procedure<Ctx, In, Out> {
    pub(crate) fn new(
        handler: HandlerFn<Ctx, In, Out>,
        route: RouteMetadata,
        openapi_meta: OpenApiMeta,
    ) -> Self {
        Self {
            handler,
            route,
            openapi_meta,
        }
    }
}

impl<Ctx, In, Out> Clone for Procedure<Ctx, In, Out> {
    fn clone(&self) -> Self {
        Self {
            handler: Arc::clone(&self.handler),
            route: self.route.clone(),
            openapi_meta: self.openapi_meta.clone(),
        }
    }
}

#[async_trait]
impl<Ctx, In, Out> ProcedureHandler<Ctx> for Procedure<Ctx, In, Out>
where
    Ctx: Clone + Send + 'static,
    In: serde::de::DeserializeOwned + Send + 'static,
    Out: serde::Serialize + Send + 'static,
{
    async fn call(
        &self,
        ctx: Ctx,
        input: Value,
        _extensions: Option<&Arc<dyn Any + Send + Sync>>,
    ) -> Result<OutputKind, OrpcError> {
        let typed_input: In = serde_json::from_value(input)
            .map_err(|e| OrpcError::bad_request(format!("Failed to deserialize input: {}", e)))?;

        let output = (self.handler)(ctx, typed_input).await?;

        let json_output =
            serde_json::to_value(&output).map_err(|e| OrpcError::internal(e.to_string()))?;

        Ok(OutputKind::Single(json_output))
    }

    fn route_metadata(&self) -> &RouteMetadata {
        &self.route
    }
}
