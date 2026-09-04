//! Procedure definitions and type-erased dispatch.

use crate::OrpcError;
use async_trait::async_trait;
use futures_core::Stream;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Output kind for procedure results — either a single value or a stream.
///
/// Supports both regular procedures that return a single value and streaming
/// procedures that return an async stream of values.
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
///
/// Allows heterogeneous procedures to be stored in a registry and called
/// through a uniform interface, converting JSON input to typed input and
/// typed output back to JSON.
///
/// # DIP: Interface for runtime dispatch — concrete Procedure<Ctx, In, Out> depends on this
#[async_trait]
pub trait ProcedureHandler<Ctx>: Send + Sync {
    /// Executes the procedure with JSON input, returns JSON output or error.
    ///
    /// Handles deserialization of input and serialization of output internally.
    async fn call(&self, ctx: Ctx, input: Value) -> Result<OutputKind, OrpcError>;
}

type HandlerFn<Ctx, In, Out> = Arc<
    dyn Fn(Ctx, In) -> Pin<Box<dyn Future<Output = Result<Out, OrpcError>> + Send>> + Send + Sync,
>;

/// A typed RPC procedure with compile-time guarantees.
///
/// Generic over context, input, and output types. The type system enforces
/// that handlers match the declared signature.
///
/// # SRP: Represents a single RPC procedure — input validation, handler execution, output serialization
pub struct Procedure<Ctx, In, Out> {
    pub(crate) handler: HandlerFn<Ctx, In, Out>,
}

impl<Ctx, In, Out> Procedure<Ctx, In, Out> {
    pub(crate) fn new(handler: HandlerFn<Ctx, In, Out>) -> Self {
        Self { handler }
    }
}

impl<Ctx, In, Out> Clone for Procedure<Ctx, In, Out> {
    fn clone(&self) -> Self {
        Self {
            handler: Arc::clone(&self.handler),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::os;
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
        let proc = os().context::<TestContext>().output::<String>().handler(
            |ctx: TestContext, _: ()| async move { Ok(format!("multiplier: {}", ctx.multiplier)) },
        );

        let result = proc.call(ctx, Value::Null).await;

        assert!(result.is_ok());
        match result.unwrap() {
            OutputKind::Single(v) => {
                assert_eq!(v, serde_json::json!("multiplier: 5"));
            }
            OutputKind::Stream(_) => panic!("Expected Single, got Stream"),
        }
    }

    #[tokio::test]
    async fn test_procedure_context_usage() {
        #[derive(Clone)]
        struct DbContext {
            data: Vec<String>,
        }

        let ctx = DbContext {
            data: vec!["item1".to_string(), "item2".to_string()],
        };

        let proc = os()
            .context::<DbContext>()
            .output::<Vec<String>>()
            .handler(|ctx: DbContext, _: ()| async move { Ok(ctx.data) });

        let result = proc.call(ctx, Value::Null).await;

        assert!(result.is_ok());
        match result.unwrap() {
            OutputKind::Single(v) => {
                let items: Vec<String> = serde_json::from_value(v).unwrap();
                assert_eq!(items, vec!["item1", "item2"]);
            }
            OutputKind::Stream(_) => panic!("Expected Single, got Stream"),
        }
    }
}
