//! Procedure registry for runtime dispatch.

use crate::{OrpcError, OutputKind, Procedure, ProcedureHandler};
use serde_json::Value;
use std::collections::HashMap;

/// Registry that holds type-erased procedures for O(1) runtime dispatch.
///
/// Flattens nested router structures into a HashMap at initialization time,
/// enabling efficient path-based lookup during request handling.
///
/// # SRP: Manages procedure storage and dispatch only
pub struct ProcedureRegistry<Ctx> {
    procedures: HashMap<String, Box<dyn ProcedureHandler<Ctx>>>,
}

impl<Ctx> ProcedureRegistry<Ctx>
where
    Ctx: Clone + Send + 'static,
{
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self {
            procedures: HashMap::new(),
        }
    }

    /// Inserts a procedure at the given path.
    ///
    /// # Arguments
    ///
    /// * `path` - Full procedure path (e.g., "planet/find")
    /// * `procedure` - Procedure to register
    pub fn insert<In, Out>(&mut self, path: impl Into<String>, procedure: &Procedure<Ctx, In, Out>)
    where
        In: serde::de::DeserializeOwned + Send + 'static,
        Out: serde::Serialize + Send + 'static,
    {
        let path = path.into();
        let cloned = procedure.clone();
        self.procedures.insert(path, Box::new(cloned));
    }

    /// Calls a procedure by path with JSON input.
    ///
    /// Returns the JSON output or an error if the procedure doesn't exist
    /// or execution fails.
    ///
    /// # Arguments
    ///
    /// * `path` - Procedure path to call
    /// * `ctx` - Context to pass to the handler
    /// * `input` - JSON input value
    pub async fn call(&self, path: &str, ctx: Ctx, input: Value) -> Result<OutputKind, OrpcError> {
        let procedure = self
            .procedures
            .get(path)
            .ok_or_else(|| OrpcError::not_found(format!("No procedure at path: {}", path)))?;

        procedure.call(ctx, input).await
    }

    /// Checks if a procedure exists at the given path.
    pub fn has(&self, path: &str) -> bool {
        self.procedures.contains_key(path)
    }

    /// Returns the number of registered procedures.
    pub fn len(&self) -> usize {
        self.procedures.len()
    }

    /// Returns true if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.procedures.is_empty()
    }

    /// Returns an iterator over all registered procedure paths.
    pub fn paths(&self) -> impl Iterator<Item = &String> {
        self.procedures.keys()
    }
}

impl<Ctx> Default for ProcedureRegistry<Ctx>
where
    Ctx: Clone + Send + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::os;
    use serde::{Deserialize, Serialize};

    #[derive(Clone)]
    struct TestContext {
        value: i32,
    }

    #[derive(Deserialize, Serialize)]
    struct Input {
        x: i32,
    }

    #[tokio::test]
    async fn test_registry_insert_and_call() {
        let mut registry = ProcedureRegistry::<TestContext>::new();
        let proc = os()
            .context::<TestContext>()
            .input::<Input>()
            .output::<i32>()
            .handler(|ctx: TestContext, input: Input| async move { Ok(ctx.value + input.x) });

        registry.insert("add", &proc);

        assert!(registry.has("add"));
        assert_eq!(registry.len(), 1);

        let ctx = TestContext { value: 10 };
        let input = serde_json::json!({ "x": 32 });
        let result = registry.call("add", ctx, input).await;

        assert!(result.is_ok());
        match result.unwrap() {
            OutputKind::Single(v) => assert_eq!(v, 42),
            OutputKind::Stream(_) => panic!("Expected Single"),
        }
    }

    #[tokio::test]
    async fn test_registry_not_found() {
        let registry = ProcedureRegistry::<TestContext>::new();
        let ctx = TestContext { value: 0 };

        let result = registry.call("nonexistent", ctx, Value::Null).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, "NOT_FOUND");
        assert!(err.message.contains("No procedure at path"));
    }

    #[tokio::test]
    async fn test_registry_multiple_procedures() {
        let mut registry = ProcedureRegistry::<TestContext>::new();

        let ping = os()
            .context::<TestContext>()
            .output::<String>()
            .handler(|_ctx: TestContext, _: ()| async { Ok("pong".to_string()) });

        let double = os()
            .context::<TestContext>()
            .input::<Input>()
            .output::<i32>()
            .handler(|_ctx: TestContext, input: Input| async move { Ok(input.x * 2) });

        registry.insert("ping", &ping);
        registry.insert("math/double", &double);

        assert_eq!(registry.len(), 2);
        assert!(registry.has("ping"));
        assert!(registry.has("math/double"));

        let ctx = TestContext { value: 0 };

        let ping_result = registry.call("ping", ctx.clone(), Value::Null).await;
        assert!(ping_result.is_ok());

        let double_result = registry
            .call("math/double", ctx, serde_json::json!({ "x": 21 }))
            .await;
        assert!(double_result.is_ok());
        match double_result.unwrap() {
            OutputKind::Single(v) => assert_eq!(v, 42),
            OutputKind::Stream(_) => panic!("Expected Single"),
        }
    }

    #[tokio::test]
    async fn test_registry_is_empty() {
        let mut registry = ProcedureRegistry::<TestContext>::new();
        assert!(registry.is_empty());

        let proc = os()
            .context::<TestContext>()
            .output::<String>()
            .handler(|_ctx: TestContext, _: ()| async { Ok("test".to_string()) });

        registry.insert("test", &proc);
        assert!(!registry.is_empty());
    }

    #[tokio::test]
    async fn test_registry_procedure_error() {
        let mut registry = ProcedureRegistry::<TestContext>::new();
        let proc = os()
            .context::<TestContext>()
            .input::<Input>()
            .output::<i32>()
            .handler(|_ctx: TestContext, _input: Input| async {
                Err(OrpcError::custom("VALIDATION_ERROR", "Invalid value"))
            });

        registry.insert("validate", &proc);

        let ctx = TestContext { value: 0 };
        let result = registry
            .call("validate", ctx, serde_json::json!({ "x": 5 }))
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, "VALIDATION_ERROR");
    }
}
