//! Procedure definitions and type-erased dispatch.

use crate::route::RouteMetadata;
use crate::OrpcError;
use async_trait::async_trait;
use futures_core::Stream;
use serde_json::Value;
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
    async fn call(&self, ctx: Ctx, input: Value) -> Result<OutputKind, OrpcError>;
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
}

impl<Ctx, In, Out> Procedure<Ctx, In, Out> {
    pub(crate) fn new(handler: HandlerFn<Ctx, In, Out>, route: RouteMetadata) -> Self {
        Self { handler, route }
    }
}

impl<Ctx, In, Out> Clone for Procedure<Ctx, In, Out> {
    fn clone(&self) -> Self {
        Self {
            handler: Arc::clone(&self.handler),
            route: self.route.clone(),
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
    async fn call(&self, ctx: Ctx, input: Value) -> Result<OutputKind, OrpcError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::route::HttpMethod;
    use crate::{os, OrpcError};
    use serde::{Deserialize, Serialize};

    #[derive(Clone)]
    struct TestContext {
        multiplier: i32,
    }

    #[derive(Deserialize, Serialize)]
    struct Input {
        value: i32,
    }

    #[tokio::test]
    async fn test_procedure_call_success() {
        let ctx = TestContext { multiplier: 2 };
        let proc = os()
            .context::<TestContext>()
            .route(HttpMethod::Post, "/multiply")
            .input::<Input>()
            .output::<i32>()
            .handler(
                |ctx: TestContext, input: Input| async move { Ok(input.value * ctx.multiplier) },
            );

        let input_json = serde_json::json!({ "value": 21 });
        let result = proc.call(ctx, input_json).await;

        assert!(result.is_ok());
        match result.unwrap() {
            OutputKind::Single(v) => assert_eq!(v, 42),
            OutputKind::Stream(_) => panic!("Expected Single, got Stream"),
        }
    }

    #[tokio::test]
    async fn test_procedure_call_error() {
        let ctx = TestContext { multiplier: 2 };
        let proc = os()
            .context::<TestContext>()
            .route(HttpMethod::Post, "/fail")
            .input::<Input>()
            .output::<i32>()
            .handler(|_ctx: TestContext, _input: Input| async move {
                Err(OrpcError::not_found("Resource not found"))
            });

        let input_json = serde_json::json!({ "value": 10 });
        let result = proc.call(ctx, input_json).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, "NOT_FOUND");
    }

    #[tokio::test]
    async fn test_procedure_invalid_input() {
        let ctx = TestContext { multiplier: 2 };
        let proc = os()
            .context::<TestContext>()
            .route(HttpMethod::Post, "/multiply")
            .input::<Input>()
            .output::<i32>()
            .handler(
                |ctx: TestContext, input: Input| async move { Ok(input.value * ctx.multiplier) },
            );

        let invalid_json = serde_json::json!({ "wrong_field": "not_a_number" });
        let result = proc.call(ctx, invalid_json).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, "BAD_REQUEST");
        assert!(err.message.contains("Failed to deserialize input"));
    }

    #[tokio::test]
    async fn test_procedure_no_input() {
        let ctx = TestContext { multiplier: 5 };
        let proc = os()
            .context::<TestContext>()
            .route(HttpMethod::Get, "/info")
            .output::<String>()
            .handler(|ctx: TestContext, _: ()| async move {
                Ok(format!("multiplier: {}", ctx.multiplier))
            });

        let result = proc.call(ctx, Value::Null).await;

        assert!(result.is_ok());
        match result.unwrap() {
            OutputKind::Single(v) => assert_eq!(v, serde_json::json!("multiplier: 5")),
            OutputKind::Stream(_) => panic!("Expected Single, got Stream"),
        }
    }

    #[tokio::test]
    async fn test_procedure_carries_route_metadata() {
        let proc = os()
            .context::<TestContext>()
            .route(HttpMethod::Delete, "/items/{id}")
            .output::<String>()
            .handler(|_ctx: TestContext, _: ()| async { Ok("deleted".to_string()) });

        assert_eq!(proc.route.method, HttpMethod::Delete);
        assert_eq!(proc.route.path, "/items/{id}");
    }
}
