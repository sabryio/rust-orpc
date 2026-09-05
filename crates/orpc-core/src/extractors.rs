//! Extractor types for handler signatures (Axum-style pattern).
//!
//! Extractors allow handlers to use familiar pattern-matching syntax to pull
//! typed data from the orpc execution context:
//!
//! ```rust,ignore
//! pub async fn handler(
//!     OrpcContext(ctx): OrpcContext<AppContext>,
//!     OrpcJson(input): OrpcJson<CreateInput>,
//! ) -> Result<impl Serialize, OrpcError> {
//!     Ok(ctx.repo.create(input).await?)
//! }
//! ```

use std::future::Future;

use crate::OrpcError;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

/// Trait for types that can be extracted from an orpc request.
///
/// Similar to Axum's `FromRequest`, this allows handlers to use extractor
/// pattern-matching syntax. Extractors consume parts of the request context
/// and forward the remainder to the next extractor in the chain.
#[async_trait]
pub trait FromOrpcRequest<Ctx>: Sized + Send + Sync {
    /// Extract this type from the request context and input.
    ///
    /// Returns `(extracted_value, remaining_ctx, remaining_input)` to support
    /// chaining multiple extractors in a handler signature.
    async fn from_request(ctx: Ctx, input: Value) -> Result<(Self, Ctx, Value), OrpcError>;
}

/// Extract the orpc context.
///
/// Use this in handler signatures to access the request context:
///
/// ```rust,ignore
/// pub async fn handler(
///     OrpcContext(ctx): OrpcContext<AppContext>,
/// ) -> Result<impl Serialize, OrpcError> {
///     // Use ctx here
/// }
/// ```
pub struct OrpcContext<T>(pub T);

#[async_trait]
impl<Ctx> FromOrpcRequest<Ctx> for OrpcContext<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    async fn from_request(ctx: Ctx, input: Value) -> Result<(Self, Ctx, Value), OrpcError> {
        let extracted = OrpcContext(ctx.clone());
        Ok((extracted, ctx, input))
    }
}

/// Extract and deserialize JSON input.
///
/// Use this in handler signatures to access typed input data:
///
/// ```rust,ignore
/// pub async fn handler(
///     OrpcJson(input): OrpcJson<CreateInput>,
/// ) -> Result<impl Serialize, OrpcError> {
///     // Use input here
/// }
/// ```
pub struct OrpcJson<T>(pub T);

#[async_trait]
impl<Ctx, T> FromOrpcRequest<Ctx> for OrpcJson<T>
where
    Ctx: Send + Sync + 'static,
    T: DeserializeOwned + Send + Sync + 'static,
{
    async fn from_request(ctx: Ctx, input: Value) -> Result<(Self, Ctx, Value), OrpcError> {
        let typed_input: T = serde_json::from_value(input.clone())
            .map_err(|e| OrpcError::bad_request(format!("Failed to deserialize input: {}", e)))?;

        let extracted = OrpcJson(typed_input);
        Ok((extracted, ctx, input))
    }
}

// Tuple implementations for chaining extractors

#[async_trait]
impl<Ctx> FromOrpcRequest<Ctx> for ()
where
    Ctx: Send + 'static,
{
    async fn from_request(ctx: Ctx, input: Value) -> Result<(Self, Ctx, Value), OrpcError> {
        Ok(((), ctx, input))
    }
}

#[async_trait]
impl<Ctx, E1> FromOrpcRequest<Ctx> for (E1,)
where
    Ctx: Send + 'static,
    E1: FromOrpcRequest<Ctx>,
{
    async fn from_request(ctx: Ctx, input: Value) -> Result<(Self, Ctx, Value), OrpcError> {
        let (e1, ctx, input) = E1::from_request(ctx, input).await?;
        Ok(((e1,), ctx, input))
    }
}

#[async_trait]
impl<Ctx, E1, E2> FromOrpcRequest<Ctx> for (E1, E2)
where
    Ctx: Send + 'static,
    E1: FromOrpcRequest<Ctx>,
    E2: FromOrpcRequest<Ctx>,
{
    async fn from_request(ctx: Ctx, input: Value) -> Result<(Self, Ctx, Value), OrpcError> {
        let (e1, ctx, input) = E1::from_request(ctx, input).await?;
        let (e2, ctx, input) = E2::from_request(ctx, input).await?;
        Ok(((e1, e2), ctx, input))
    }
}

#[async_trait]
impl<Ctx, E1, E2, E3> FromOrpcRequest<Ctx> for (E1, E2, E3)
where
    Ctx: Send + 'static,
    E1: FromOrpcRequest<Ctx>,
    E2: FromOrpcRequest<Ctx>,
    E3: FromOrpcRequest<Ctx>,
{
    async fn from_request(ctx: Ctx, input: Value) -> Result<(Self, Ctx, Value), OrpcError> {
        let (e1, ctx, input) = E1::from_request(ctx, input).await?;
        let (e2, ctx, input) = E2::from_request(ctx, input).await?;
        let (e3, ctx, input) = E3::from_request(ctx, input).await?;
        Ok(((e1, e2, e3), ctx, input))
    }
}

#[async_trait]
impl<Ctx, E1, E2, E3, E4> FromOrpcRequest<Ctx> for (E1, E2, E3, E4)
where
    Ctx: Send + 'static,
    E1: FromOrpcRequest<Ctx>,
    E2: FromOrpcRequest<Ctx>,
    E3: FromOrpcRequest<Ctx>,
    E4: FromOrpcRequest<Ctx>,
{
    async fn from_request(ctx: Ctx, input: Value) -> Result<(Self, Ctx, Value), OrpcError> {
        let (e1, ctx, input) = E1::from_request(ctx, input).await?;
        let (e2, ctx, input) = E2::from_request(ctx, input).await?;
        let (e3, ctx, input) = E3::from_request(ctx, input).await?;
        let (e4, ctx, input) = E4::from_request(ctx, input).await?;
        Ok(((e1, e2, e3, e4), ctx, input))
    }
}

#[async_trait]
impl<Ctx, E1, E2, E3, E4, E5> FromOrpcRequest<Ctx> for (E1, E2, E3, E4, E5)
where
    Ctx: Send + 'static,
    E1: FromOrpcRequest<Ctx>,
    E2: FromOrpcRequest<Ctx>,
    E3: FromOrpcRequest<Ctx>,
    E4: FromOrpcRequest<Ctx>,
    E5: FromOrpcRequest<Ctx>,
{
    async fn from_request(ctx: Ctx, input: Value) -> Result<(Self, Ctx, Value), OrpcError> {
        let (e1, ctx, input) = E1::from_request(ctx, input).await?;
        let (e2, ctx, input) = E2::from_request(ctx, input).await?;
        let (e3, ctx, input) = E3::from_request(ctx, input).await?;
        let (e4, ctx, input) = E4::from_request(ctx, input).await?;
        let (e5, ctx, input) = E5::from_request(ctx, input).await?;
        Ok(((e1, e2, e3, e4, e5), ctx, input))
    }
}

#[async_trait]
impl<Ctx, E1, E2, E3, E4, E5, E6> FromOrpcRequest<Ctx> for (E1, E2, E3, E4, E5, E6)
where
    Ctx: Send + 'static,
    E1: FromOrpcRequest<Ctx>,
    E2: FromOrpcRequest<Ctx>,
    E3: FromOrpcRequest<Ctx>,
    E4: FromOrpcRequest<Ctx>,
    E5: FromOrpcRequest<Ctx>,
    E6: FromOrpcRequest<Ctx>,
{
    async fn from_request(ctx: Ctx, input: Value) -> Result<(Self, Ctx, Value), OrpcError> {
        let (e1, ctx, input) = E1::from_request(ctx, input).await?;
        let (e2, ctx, input) = E2::from_request(ctx, input).await?;
        let (e3, ctx, input) = E3::from_request(ctx, input).await?;
        let (e4, ctx, input) = E4::from_request(ctx, input).await?;
        let (e5, ctx, input) = E5::from_request(ctx, input).await?;
        let (e6, ctx, input) = E6::from_request(ctx, input).await?;
        Ok(((e1, e2, e3, e4, e5, e6), ctx, input))
    }
}

#[async_trait]
impl<Ctx, E1, E2, E3, E4, E5, E6, E7> FromOrpcRequest<Ctx> for (E1, E2, E3, E4, E5, E6, E7)
where
    Ctx: Send + 'static,
    E1: FromOrpcRequest<Ctx>,
    E2: FromOrpcRequest<Ctx>,
    E3: FromOrpcRequest<Ctx>,
    E4: FromOrpcRequest<Ctx>,
    E5: FromOrpcRequest<Ctx>,
    E6: FromOrpcRequest<Ctx>,
    E7: FromOrpcRequest<Ctx>,
{
    async fn from_request(ctx: Ctx, input: Value) -> Result<(Self, Ctx, Value), OrpcError> {
        let (e1, ctx, input) = E1::from_request(ctx, input).await?;
        let (e2, ctx, input) = E2::from_request(ctx, input).await?;
        let (e3, ctx, input) = E3::from_request(ctx, input).await?;
        let (e4, ctx, input) = E4::from_request(ctx, input).await?;
        let (e5, ctx, input) = E5::from_request(ctx, input).await?;
        let (e6, ctx, input) = E6::from_request(ctx, input).await?;
        let (e7, ctx, input) = E7::from_request(ctx, input).await?;
        Ok(((e1, e2, e3, e4, e5, e6, e7), ctx, input))
    }
}

#[async_trait]
impl<Ctx, E1, E2, E3, E4, E5, E6, E7, E8> FromOrpcRequest<Ctx> for (E1, E2, E3, E4, E5, E6, E7, E8)
where
    Ctx: Send + 'static,
    E1: FromOrpcRequest<Ctx>,
    E2: FromOrpcRequest<Ctx>,
    E3: FromOrpcRequest<Ctx>,
    E4: FromOrpcRequest<Ctx>,
    E5: FromOrpcRequest<Ctx>,
    E6: FromOrpcRequest<Ctx>,
    E7: FromOrpcRequest<Ctx>,
    E8: FromOrpcRequest<Ctx>,
{
    async fn from_request(ctx: Ctx, input: Value) -> Result<(Self, Ctx, Value), OrpcError> {
        let (e1, ctx, input) = E1::from_request(ctx, input).await?;
        let (e2, ctx, input) = E2::from_request(ctx, input).await?;
        let (e3, ctx, input) = E3::from_request(ctx, input).await?;
        let (e4, ctx, input) = E4::from_request(ctx, input).await?;
        let (e5, ctx, input) = E5::from_request(ctx, input).await?;
        let (e6, ctx, input) = E6::from_request(ctx, input).await?;
        let (e7, ctx, input) = E7::from_request(ctx, input).await?;
        let (e8, ctx, input) = E8::from_request(ctx, input).await?;
        Ok(((e1, e2, e3, e4, e5, e6, e7, e8), ctx, input))
    }
}

/// Trait for handler functions with extractors.
///
/// Implemented for functions with different numbers of extractor arguments.
/// This allows handlers to accept multiple separate extractor parameters:
///
/// ```rust,ignore
/// // One parameter
/// async fn handler1(OrpcContext(ctx): OrpcContext<Ctx>) -> Result<Out, OrpcError>
///
/// // Two parameters  
/// async fn handler2(
///     OrpcContext(ctx): OrpcContext<Ctx>,
///     OrpcJson(input): OrpcJson<Input>,
/// ) -> Result<Out, OrpcError>
/// ```
#[async_trait]
pub trait Handler<Ctx, In, Out, Extractors>: Send + Sync + 'static {
    /// Call the handler after extracting values from context and input.
    async fn call(&self, ctx: Ctx, input: In) -> Result<Out, OrpcError>;
}

// Implement Handler for functions with 0 extractors
#[async_trait]
impl<Ctx, In, Out, F, Fut> Handler<Ctx, In, Out, ()> for F
where
    Ctx: Send + 'static,
    In: Serialize + Send + 'static,
    Out: Send + 'static,
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Out, OrpcError>> + Send,
{
    async fn call(&self, _ctx: Ctx, _input: In) -> Result<Out, OrpcError> {
        self().await
    }
}

// Implement Handler for functions with 1 extractor
#[async_trait]
impl<Ctx, In, Out, E1, F, Fut> Handler<Ctx, In, Out, (E1,)> for F
where
    Ctx: Clone + Send + 'static,
    In: Serialize + Send + 'static,
    Out: Send + 'static,
    E1: FromOrpcRequest<Ctx> + Send,
    F: Fn(E1) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Out, OrpcError>> + Send,
{
    async fn call(&self, ctx: Ctx, input: In) -> Result<Out, OrpcError> {
        let input_value = serde_json::to_value(&input)
            .map_err(|e| OrpcError::internal(format!("Failed to serialize input: {}", e)))?;

        let (e1, _, _) = E1::from_request(ctx, input_value).await?;
        self(e1).await
    }
}

// Implement Handler for functions with 2 extractors
#[async_trait]
impl<Ctx, In, Out, E1, E2, F, Fut> Handler<Ctx, In, Out, (E1, E2)> for F
where
    Ctx: Clone + Send + 'static,
    In: Serialize + Send + 'static,
    Out: Send + 'static,
    E1: FromOrpcRequest<Ctx> + Send,
    E2: FromOrpcRequest<Ctx> + Send,
    F: Fn(E1, E2) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Out, OrpcError>> + Send,
{
    async fn call(&self, ctx: Ctx, input: In) -> Result<Out, OrpcError> {
        let input_value = serde_json::to_value(&input)
            .map_err(|e| OrpcError::internal(format!("Failed to serialize input: {}", e)))?;

        let (e1, ctx, input_value) = E1::from_request(ctx, input_value).await?;
        let (e2, _, _) = E2::from_request(ctx, input_value).await?;
        self(e1, e2).await
    }
}

// Implement Handler for functions with 3 extractors
#[async_trait]
impl<Ctx, In, Out, E1, E2, E3, F, Fut> Handler<Ctx, In, Out, (E1, E2, E3)> for F
where
    Ctx: Clone + Send + 'static,
    In: Serialize + Send + 'static,
    Out: Send + 'static,
    E1: FromOrpcRequest<Ctx> + Send,
    E2: FromOrpcRequest<Ctx> + Send,
    E3: FromOrpcRequest<Ctx> + Send,
    F: Fn(E1, E2, E3) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Out, OrpcError>> + Send,
{
    async fn call(&self, ctx: Ctx, input: In) -> Result<Out, OrpcError> {
        let input_value = serde_json::to_value(&input)
            .map_err(|e| OrpcError::internal(format!("Failed to serialize input: {}", e)))?;

        let (e1, ctx, input_value) = E1::from_request(ctx, input_value).await?;
        let (e2, ctx, input_value) = E2::from_request(ctx, input_value).await?;
        let (e3, _, _) = E3::from_request(ctx, input_value).await?;
        self(e1, e2, e3).await
    }
}

// Implement Handler for functions with 4 extractors
#[async_trait]
impl<Ctx, In, Out, E1, E2, E3, E4, F, Fut> Handler<Ctx, In, Out, (E1, E2, E3, E4)> for F
where
    Ctx: Clone + Send + 'static,
    In: Serialize + Send + 'static,
    Out: Send + 'static,
    E1: FromOrpcRequest<Ctx> + Send,
    E2: FromOrpcRequest<Ctx> + Send,
    E3: FromOrpcRequest<Ctx> + Send,
    E4: FromOrpcRequest<Ctx> + Send,
    F: Fn(E1, E2, E3, E4) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Out, OrpcError>> + Send,
{
    async fn call(&self, ctx: Ctx, input: In) -> Result<Out, OrpcError> {
        let input_value = serde_json::to_value(&input)
            .map_err(|e| OrpcError::internal(format!("Failed to serialize input: {}", e)))?;

        let (e1, ctx, input_value) = E1::from_request(ctx, input_value).await?;
        let (e2, ctx, input_value) = E2::from_request(ctx, input_value).await?;
        let (e3, ctx, input_value) = E3::from_request(ctx, input_value).await?;
        let (e4, _, _) = E4::from_request(ctx, input_value).await?;
        self(e1, e2, e3, e4).await
    }
}
