use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::OrpcError;

/// A future that returns a transformed context or an error.
pub type MiddlewareFuture<NewCtx> = Pin<Box<dyn Future<Output = Result<NewCtx, OrpcError>> + Send>>;

/// The continuation that represents the rest of the middleware chain.
///
/// Call `next.run(new_context)` to pass control to the next middleware or handler.
/// The middleware must construct the new context and pass it to `next.run()`.
pub struct Next<NewCtx> {
    inner: Arc<dyn Fn(NewCtx) -> MiddlewareFuture<NewCtx> + Send + Sync>,
}

impl<NewCtx> Next<NewCtx> {
    /// Create a new `Next` from a closure that accepts the transformed context.
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(NewCtx) -> MiddlewareFuture<NewCtx> + Send + Sync + 'static,
    {
        Self { inner: Arc::new(f) }
    }

    /// Execute the rest of the middleware chain with the provided context.
    ///
    /// The context passed here is the transformed context that the middleware
    /// has created. It will be passed through the rest of the chain.
    pub async fn run(self, ctx: NewCtx) -> Result<NewCtx, OrpcError> {
        (self.inner)(ctx).await
    }
}

impl<NewCtx> Clone for Next<NewCtx> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Type alias for a middleware function.
///
/// A middleware function receives:
/// - `Ctx`: The current context
/// - `Next<NewCtx>`: The continuation representing the rest of the chain
///
/// It returns a future that produces the transformed context or an error.
pub type MiddlewareFn<Ctx, NewCtx> = Arc<
    dyn Fn(Ctx, Next<NewCtx>) -> Pin<Box<dyn Future<Output = Result<NewCtx, OrpcError>> + Send>>
        + Send
        + Sync,
>;

/// A typed middleware that transforms context from `Ctx` to `NewCtx`.
///
/// # Examples
///
/// ```rust,ignore
/// use orpc_core::{Middleware, Next, OrpcError};
///
/// async fn require_auth(ctx: BaseContext, next: Next<AuthContext>) -> Result<AuthContext, OrpcError> {
///     let user = get_user(&ctx.db).await?;
///     next.run(AuthContext { db: ctx.db, user }).await
/// }
///
/// let middleware = Middleware::new(require_auth);
/// ```
pub struct Middleware<Ctx, NewCtx> {
    func: MiddlewareFn<Ctx, NewCtx>,
}

impl<Ctx, NewCtx> Middleware<Ctx, NewCtx> {
    /// Create a new middleware from a function.
    pub fn new<F, Fut>(f: F) -> Self
    where
        F: Fn(Ctx, Next<NewCtx>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<NewCtx, OrpcError>> + Send + 'static,
    {
        Self {
            func: Arc::new(move |ctx, next| Box::pin(f(ctx, next))),
        }
    }

    /// Get the underlying middleware function.
    pub(crate) fn func(&self) -> &MiddlewareFn<Ctx, NewCtx> {
        &self.func
    }
}

impl<Ctx, NewCtx> Clone for Middleware<Ctx, NewCtx> {
    fn clone(&self) -> Self {
        Self {
            func: Arc::clone(&self.func),
        }
    }
}

/// Type alias for a composed middleware stack function.
///
/// This represents the entire accumulated chain of middleware, from `OuterCtx` to `HandlerCtx`.
pub type MiddlewareStackFn<OuterCtx, HandlerCtx> =
    Arc<dyn Fn(OuterCtx) -> MiddlewareFuture<HandlerCtx> + Send + Sync>;

/// Trait for types that can be converted into middleware.
///
/// This is implemented for:
/// - Async functions with signature `Fn(Ctx, Next<NewCtx>) -> Future<Output = Result<NewCtx, OrpcError>>`
/// - `Middleware<Ctx, NewCtx>` itself
/// - Closures with the same signature
pub trait IntoMiddleware<Ctx, NewCtx> {
    /// Convert this value into a middleware.
    fn into_middleware(self) -> Middleware<Ctx, NewCtx>;
}

// Identity conversion for Middleware itself
impl<Ctx, NewCtx> IntoMiddleware<Ctx, NewCtx> for Middleware<Ctx, NewCtx> {
    fn into_middleware(self) -> Middleware<Ctx, NewCtx> {
        self
    }
}

// Blanket impl for 2-arg async functions: context-only middleware
impl<Ctx, NewCtx, F, Fut> IntoMiddleware<Ctx, NewCtx> for F
where
    F: Fn(Ctx, Next<NewCtx>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<NewCtx, OrpcError>> + Send + 'static,
    Ctx: 'static,
    NewCtx: 'static,
{
    fn into_middleware(self) -> Middleware<Ctx, NewCtx> {
        Middleware::new(self)
    }
}

impl<Ctx, NewCtx> Middleware<Ctx, NewCtx> {
    /// Adapt this middleware to work with input-aware logic.
    ///
    /// The adapter function `f` receives a reference to the input and returns
    /// a value that the middleware can use for its logic (e.g., extracting an ID).
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// async fn can_edit(ctx: AuthContext, id: i32, next: Next<AuthContext>) -> Result<AuthContext, OrpcError> {
    ///     check_permission(&ctx.db, id).await?;
    ///     next.run(ctx).await
    /// }
    ///
    /// let middleware = Middleware::new(can_edit)
    ///     .adapt_input(|input: &UpdateInput| input.id);
    /// ```
    pub fn adapt_input<In, MappedIn, F>(
        self,
        adapter: F,
    ) -> AdaptedMiddleware<Ctx, NewCtx, In, MappedIn>
    where
        F: Fn(&In) -> MappedIn + Send + Sync + 'static,
        In: 'static,
        MappedIn: 'static,
    {
        AdaptedMiddleware {
            middleware: self,
            adapter: Arc::new(adapter),
        }
    }
}

/// A middleware that has been adapted to work with input.
///
/// This wraps a middleware and an adapter function that extracts a value from the input.
pub struct AdaptedMiddleware<Ctx, NewCtx, In, MappedIn> {
    middleware: Middleware<Ctx, NewCtx>,
    adapter: Arc<dyn Fn(&In) -> MappedIn + Send + Sync>,
}

impl<Ctx, NewCtx, In, MappedIn> Clone for AdaptedMiddleware<Ctx, NewCtx, In, MappedIn> {
    fn clone(&self) -> Self {
        Self {
            middleware: self.middleware.clone(),
            adapter: Arc::clone(&self.adapter),
        }
    }
}

// Note: We'll implement IntoMiddleware for AdaptedMiddleware in T006 when we wire it into the builder,
// since it needs to know about the input type at use_middleware() call time.

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct BaseContext {
        value: i32,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct AuthContext {
        value: i32,
        user_id: String,
    }

    #[tokio::test]
    async fn test_middleware_new_with_closure() {
        async fn add_user(
            ctx: BaseContext,
            next: Next<AuthContext>,
        ) -> Result<AuthContext, OrpcError> {
            let auth_ctx = AuthContext {
                value: ctx.value,
                user_id: "user123".to_string(),
            };
            next.run(auth_ctx).await
        }

        let middleware = Middleware::new(add_user);

        // Create a dummy Next that just returns the context
        let next = Next::new(|auth_ctx| Box::pin(async move { Ok(auth_ctx) }));

        let result = (middleware.func())(BaseContext { value: 42 }, next).await;

        assert!(result.is_ok());
        let auth_ctx = result.unwrap();
        assert_eq!(auth_ctx.value, 42);
        assert_eq!(auth_ctx.user_id, "user123");
    }

    #[tokio::test]
    async fn test_into_middleware_for_bare_fn() {
        async fn my_middleware(
            ctx: BaseContext,
            next: Next<AuthContext>,
        ) -> Result<AuthContext, OrpcError> {
            let auth_ctx = AuthContext {
                value: ctx.value * 2,
                user_id: "test_user".to_string(),
            };
            next.run(auth_ctx).await
        }

        // Test that IntoMiddleware works for bare async fn
        let _middleware: Middleware<BaseContext, AuthContext> = my_middleware.into_middleware();
    }

    #[tokio::test]
    async fn test_next_calls_continuation() {
        let next = Next::new(|auth_ctx| Box::pin(async move { Ok(auth_ctx) }));

        let result = next
            .run(AuthContext {
                value: 50,
                user_id: "initial".to_string(),
            })
            .await;

        assert!(result.is_ok());
        let ctx = result.unwrap();
        // The Next just returns what was passed to it (identity)
        assert_eq!(ctx.value, 50);
        assert_eq!(ctx.user_id, "initial");
    }

    #[tokio::test]
    async fn test_middleware_error_propagation() {
        async fn failing_middleware(
            _ctx: BaseContext,
            _next: Next<AuthContext>,
        ) -> Result<AuthContext, OrpcError> {
            Err(OrpcError::unauthorized("Authentication failed"))
        }

        let middleware = Middleware::new(failing_middleware);

        let next = Next::new(|auth_ctx| Box::pin(async move { Ok(auth_ctx) }));

        let result = (middleware.func())(BaseContext { value: 10 }, next).await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Authentication failed"));
    }
}
