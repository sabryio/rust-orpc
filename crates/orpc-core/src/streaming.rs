//! Streaming procedure support for Server-Sent Events (SSE).

use crate::route::RouteMetadata;
use crate::OrpcError;
use async_trait::async_trait;
use futures_core::Stream;
use serde_json::Value;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;

/// Marker type for streaming output — used as `.output::<AsyncIterator<T>>()`.
///
/// This enables the type system to distinguish between:
/// - `.output::<T>()` → single value handler
/// - `.output::<AsyncIterator<T>>()` → streaming handler
pub struct AsyncIterator<T>(PhantomData<T>);

type StreamHandlerFn<Ctx, In, T> = Arc<
    dyn Fn(
            Ctx,
            In,
        ) -> Pin<
            Box<
                dyn Future<Output = Result<Pin<Box<dyn Stream<Item = T> + Send>>, OrpcError>>
                    + Send,
            >,
        > + Send
        + Sync,
>;

/// A streaming RPC procedure that outputs a stream of values.
pub struct StreamingProcedure<Ctx, In, T> {
    pub(crate) handler: StreamHandlerFn<Ctx, In, T>,
    /// Route metadata declared via `.route()` — read by transport adapters.
    pub route: RouteMetadata,
}

impl<Ctx, In, T> StreamingProcedure<Ctx, In, T> {
    pub(crate) fn new(handler: StreamHandlerFn<Ctx, In, T>, route: RouteMetadata) -> Self {
        Self { handler, route }
    }
}

impl<Ctx, In, T> Clone for StreamingProcedure<Ctx, In, T> {
    fn clone(&self) -> Self {
        Self {
            handler: Arc::clone(&self.handler),
            route: self.route.clone(),
        }
    }
}

#[async_trait]
impl<Ctx, In, T> crate::ProcedureHandler<Ctx> for StreamingProcedure<Ctx, In, T>
where
    Ctx: Clone + Send + 'static,
    In: serde::de::DeserializeOwned + Send + 'static,
    T: serde::Serialize + Send + 'static,
{
    async fn call(&self, ctx: Ctx, input: Value) -> Result<crate::OutputKind, OrpcError> {
        let typed_input: In = serde_json::from_value(input)
            .map_err(|e| OrpcError::bad_request(format!("Failed to deserialize input: {}", e)))?;

        let stream = (self.handler)(ctx, typed_input).await?;

        // Convert Stream<Item = T> to Stream<Item = Value>
        use tokio_stream::StreamExt;

        let json_stream = stream.map(|item| {
            serde_json::to_value(&item).unwrap_or_else(|e| {
                serde_json::json!({
                    "error": format!("Failed to serialize stream item: {}", e)
                })
            })
        });

        Ok(crate::OutputKind::Stream(Box::pin(json_stream)))
    }

    fn route_metadata(&self) -> &crate::RouteMetadata {
        &self.route
    }
}
