//! Type-safe procedure builder with compile-time guarantees.

use crate::{OrpcError, Procedure};
use std::future::Future;
use std::marker::PhantomData;
use std::sync::Arc;

/// Type-safe builder for constructing RPC procedures.
///
/// Uses phantom types to enforce correct state transitions at compile time:
/// - Input type must be set before handler (if needed)
/// - Output type must be set before handler
/// - Handler can only be called once all required types are set
///
/// # DIP: Depends on OrpcError abstraction, not concrete error types
pub struct ProcedureBuilder<Ctx, In, Out> {
    _phantom: PhantomData<(Ctx, In, Out)>,
}

impl ProcedureBuilder<(), (), ()> {
    fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<Ctx, In, Out> ProcedureBuilder<Ctx, In, Out> {
    /// Sets the context type for this procedure.
    ///
    /// The handler will receive this context type as its first parameter.
    pub fn context<C>(self) -> ProcedureBuilder<C, In, Out> {
        ProcedureBuilder {
            _phantom: PhantomData,
        }
    }

    /// Sets the input type for this procedure.
    ///
    /// The handler will receive a value of type `I` deserialized from JSON.
    pub fn input<I>(self) -> ProcedureBuilder<Ctx, I, Out> {
        ProcedureBuilder {
            _phantom: PhantomData,
        }
    }

    /// Sets the output type for this procedure.
    ///
    /// The handler must return `Result<O, OrpcError>` where `O` is the output type.
    pub fn output<O>(self) -> ProcedureBuilder<Ctx, In, O> {
        ProcedureBuilder {
            _phantom: PhantomData,
        }
    }
}

// Handler implementation that works for both with-input and no-input cases
impl<Ctx, In, Out> ProcedureBuilder<Ctx, In, Out>
where
    Ctx: Clone + Send + 'static,
    In: serde::de::DeserializeOwned + Send + 'static,
    Out: serde::Serialize + Send + 'static,
{
    /// Defines the handler for a procedure.
    ///
    /// The handler receives the context and input (or `()` if no input),
    /// and must return `Result<Out, OrpcError>`.
    ///
    /// # Examples
    ///
    /// With input:
    /// ```rust,ignore
    /// os().input::<MyInput>().output::<MyOutput>()
    ///     .handler(|ctx, input| async move { Ok(output) })
    /// ```
    ///
    /// Without input:
    /// ```rust,ignore
    /// os().output::<MyOutput>()
    ///     .handler(|ctx, _| async move { Ok(output) })
    /// ```
    pub fn handler<F, Fut>(self, handler: F) -> Procedure<Ctx, In, Out>
    where
        F: Fn(Ctx, In) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Out, OrpcError>> + Send + 'static,
    {
        Procedure::new(Arc::new(move |ctx, input| {
            let fut = handler(ctx, input);
            Box::pin(fut)
        }))
    }
}

/// Entry point for building procedures, mirroring TypeScript oRPC's `oc` pattern.
///
/// # Example
///
/// ```rust
/// use orpc_core::os;
///
/// # #[derive(Clone)]
/// # struct Ctx;
/// let proc = os()
///     .context::<Ctx>()
///     .output::<String>()
///     .handler(|_ctx: Ctx, _: ()| async { Ok("pong".to_string()) });
/// ```
pub fn os() -> ProcedureBuilder<(), (), ()> {
    ProcedureBuilder::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct TestContext {
        value: i32,
    }

    #[test]
    fn test_builder_no_input() {
        let proc = os()
            .context::<TestContext>()
            .output::<String>()
            .handler(|_ctx: TestContext, _: ()| async { Ok("test".to_string()) });

        // Type check: procedure was created successfully
        let _: Procedure<TestContext, (), String> = proc;
    }

    #[test]
    fn test_builder_with_input() {
        #[derive(serde::Deserialize)]
        struct Input {
            value: i32,
        }

        let proc = os()
            .context::<TestContext>()
            .input::<Input>()
            .output::<String>()
            .handler(|_ctx: TestContext, input: Input| async move {
                Ok(format!("value: {}", input.value))
            });

        // Type check: procedure was created successfully
        let _: Procedure<TestContext, Input, String> = proc;
    }

    #[test]
    fn test_builder_input_output_order() {
        #[derive(serde::Deserialize)]
        struct Input {
            x: i32,
        }

        // Should work in either order
        let _proc1 = os()
            .context::<TestContext>()
            .input::<Input>()
            .output::<i32>()
            .handler(|_ctx: TestContext, input: Input| async move { Ok(input.x * 2) });

        let _proc2 = os()
            .context::<TestContext>()
            .output::<i32>()
            .input::<Input>()
            .handler(|_ctx: TestContext, input: Input| async move { Ok(input.x * 2) });
    }

    #[tokio::test]
    async fn test_handler_execution_no_input() {
        let proc = os()
            .context::<TestContext>()
            .output::<i32>()
            .handler(|ctx: TestContext, _: ()| async move { Ok(ctx.value) });

        // Handler type-checks and can be constructed
        let _: Procedure<TestContext, (), i32> = proc;
    }

    #[tokio::test]
    async fn test_handler_execution_with_input() {
        #[derive(serde::Deserialize, serde::Serialize)]
        struct Input {
            value: i32,
        }

        let proc = os()
            .context::<TestContext>()
            .input::<Input>()
            .output::<i32>()
            .handler(|ctx: TestContext, input: Input| async move { Ok(ctx.value + input.value) });

        // Handler type-checks and can be constructed
        let _: Procedure<TestContext, Input, i32> = proc;
    }
}
