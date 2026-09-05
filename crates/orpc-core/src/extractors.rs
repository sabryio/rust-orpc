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

// Macro to generate FromOrpcRequest implementations for tuples
macro_rules! impl_from_orpc_request_tuple {
    // Base case: empty tuple
    () => {
        #[async_trait]
        impl<Ctx> FromOrpcRequest<Ctx> for ()
        where
            Ctx: Send + 'static,
        {
            async fn from_request(ctx: Ctx, input: Value) -> Result<(Self, Ctx, Value), OrpcError> {
                Ok(((), ctx, input))
            }
        }
    };

    // Recursive case: 1+ extractors in tuple
    ($($E:ident),+) => {
        #[async_trait]
        impl<Ctx, $($E),+> FromOrpcRequest<Ctx> for ($($E,)+)
        where
            Ctx: Send + 'static,
            $($E: FromOrpcRequest<Ctx>,)+
        {
            #[allow(non_snake_case)]
            async fn from_request(ctx: Ctx, input: Value) -> Result<(Self, Ctx, Value), OrpcError> {
                impl_from_orpc_request_tuple!(@extract ctx, input => $($E),+);
                Ok((($($E,)+), ctx, input))
            }
        }
    };

    // Helper: extract tuple elements sequentially
    (@extract $ctx:ident, $input:ident => $E1:ident) => {
        let ($E1, $ctx, $input) = $E1::from_request($ctx, $input).await?;
    };
    (@extract $ctx:ident, $input:ident => $E1:ident, $($E:ident),+) => {
        let ($E1, $ctx, $input) = $E1::from_request($ctx, $input).await?;
        impl_from_orpc_request_tuple!(@extract $ctx, $input => $($E),+);
    };
}

// Generate implementations for 0-8 element tuples
impl_from_orpc_request_tuple!();
impl_from_orpc_request_tuple!(E1);
impl_from_orpc_request_tuple!(E1, E2);
impl_from_orpc_request_tuple!(E1, E2, E3);
impl_from_orpc_request_tuple!(E1, E2, E3, E4);
impl_from_orpc_request_tuple!(E1, E2, E3, E4, E5);
impl_from_orpc_request_tuple!(E1, E2, E3, E4, E5, E6);
impl_from_orpc_request_tuple!(E1, E2, E3, E4, E5, E6, E7);
impl_from_orpc_request_tuple!(E1, E2, E3, E4, E5, E6, E7, E8);

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

// Macro to generate Handler implementations for different arities
macro_rules! impl_handler {
    // Base case: 0 extractors
    () => {
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
    };

    // Recursive case: 1+ extractors
    ($($E:ident),+) => {
        #[async_trait]
        impl<Ctx, In, Out, $($E,)+ F, Fut> Handler<Ctx, In, Out, ($($E,)+)> for F
        where
            Ctx: Clone + Send + 'static,
            In: Serialize + Send + 'static,
            Out: Send + 'static,
            $($E: FromOrpcRequest<Ctx> + Send,)+
            F: Fn($($E),+) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Result<Out, OrpcError>> + Send,
        {
            #[allow(non_snake_case)]
            async fn call(&self, ctx: Ctx, input: In) -> Result<Out, OrpcError> {
                let input_value = serde_json::to_value(&input)
                    .map_err(|e| OrpcError::internal(format!("Failed to serialize input: {}", e)))?;

                impl_handler!(@extract ctx, input_value => $($E),+);

                self($($E),+).await
            }
        }
    };

    // Helper: extract extractors sequentially
    (@extract $ctx:ident, $input:ident => $E1:ident) => {
        let ($E1, _, _) = $E1::from_request($ctx, $input).await?;
    };
    (@extract $ctx:ident, $input:ident => $E1:ident, $($E:ident),+) => {
        let ($E1, $ctx, $input) = $E1::from_request($ctx, $input).await?;
        impl_handler!(@extract $ctx, $input => $($E),+);
    };
}

// Generate implementations for 0-8 extractors
impl_handler!();
impl_handler!(E1);
impl_handler!(E1, E2);
impl_handler!(E1, E2, E3);
impl_handler!(E1, E2, E3, E4);
impl_handler!(E1, E2, E3, E4, E5);
impl_handler!(E1, E2, E3, E4, E5, E6);
impl_handler!(E1, E2, E3, E4, E5, E6, E7);
impl_handler!(E1, E2, E3, E4, E5, E6, E7, E8);
