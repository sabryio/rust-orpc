//! Integration test: End-to-end procedure call with context injection

use orpc_core::{OrpcError, OutputKind, Procedure, ProcedureHandler, os};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
struct DatabaseContext {
    data: Arc<Vec<Planet>>,
    user_id: Option<i32>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
struct Planet {
    id: i32,
    name: String,
    discovered_by: Option<i32>,
}

#[derive(Deserialize)]
struct FindPlanetInput {
    id: i32,
}

#[derive(Deserialize)]
struct CreatePlanetInput {
    name: String,
}

#[tokio::test]
async fn test_end_to_end_procedure_with_context() {
    // Setup context with data
    let ctx = DatabaseContext {
        data: Arc::new(vec![
            Planet {
                id: 1,
                name: "Earth".to_string(),
                discovered_by: None,
            },
            Planet {
                id: 2,
                name: "Mars".to_string(),
                discovered_by: Some(42),
            },
        ]),
        user_id: Some(100),
    };

    // Create procedure that uses context
    let find_planet = os()
        .context::<DatabaseContext>()
        .input::<FindPlanetInput>()
        .output::<Planet>()
        .handler(|ctx: DatabaseContext, input: FindPlanetInput| async move {
            ctx.data
                .iter()
                .find(|p| p.id == input.id)
                .cloned()
                .ok_or_else(|| OrpcError::not_found(format!("Planet {} not found", input.id)))
        });

    // Call procedure
    let input = serde_json::json!({ "id": 1 });
    let result = find_planet.call(ctx.clone(), input).await;

    assert!(result.is_ok());
    match result.unwrap() {
        OutputKind::Single(value) => {
            let planet: Planet = serde_json::from_value(value).unwrap();
            assert_eq!(planet.id, 1);
            assert_eq!(planet.name, "Earth");
        }
        OutputKind::Stream(_) => panic!("Expected Single output"),
    }
}

#[tokio::test]
async fn test_context_shared_across_procedures() {
    let ctx = DatabaseContext {
        data: Arc::new(vec![
            Planet {
                id: 1,
                name: "Venus".to_string(),
                discovered_by: None,
            },
        ]),
        user_id: Some(200),
    };

    // Multiple procedures share the same context type
    let list_planets = os()
        .context::<DatabaseContext>()
        .output::<Vec<Planet>>()
        .handler(|ctx: DatabaseContext, _: ()| async move { Ok(ctx.data.to_vec()) });

    let count_planets = os()
        .context::<DatabaseContext>()
        .output::<usize>()
        .handler(|ctx: DatabaseContext, _: ()| async move { Ok(ctx.data.len()) });

    // Both procedures can access the same context
    let list_result = list_planets.call(ctx.clone(), serde_json::Value::Null).await;
    assert!(list_result.is_ok());

    let count_result = count_planets.call(ctx, serde_json::Value::Null).await;
    assert!(count_result.is_ok());

    match count_result.unwrap() {
        OutputKind::Single(value) => {
            let count: usize = serde_json::from_value(value).unwrap();
            assert_eq!(count, 1);
        }
        OutputKind::Stream(_) => panic!("Expected Single output"),
    }
}

#[tokio::test]
async fn test_procedure_error_handling() {
    let ctx = DatabaseContext {
        data: Arc::new(vec![]),
        user_id: None,
    };

    let create_planet = os()
        .context::<DatabaseContext>()
        .input::<CreatePlanetInput>()
        .output::<Planet>()
        .handler(|ctx: DatabaseContext, input: CreatePlanetInput| async move {
            // Require authenticated user
            let user_id = ctx
                .user_id
                .ok_or_else(|| OrpcError::custom("UNAUTHORIZED", "Authentication required"))?;

            if input.name.is_empty() {
                return Err(OrpcError::bad_request("Planet name cannot be empty"));
            }

            Ok(Planet {
                id: 999,
                name: input.name,
                discovered_by: Some(user_id),
            })
        });

    // Should fail due to missing user_id
    let input = serde_json::json!({ "name": "NewPlanet" });
    let result = create_planet.call(ctx, input).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, "UNAUTHORIZED");
}

#[tokio::test]
async fn test_procedure_type_safety() {
    #[derive(Clone)]
    struct TypedContext {
        value: i32,
    }

    #[derive(Deserialize)]
    struct TypedInput {
        multiplier: i32,
    }

    let proc: Procedure<TypedContext, TypedInput, i32> = os()
        .context::<TypedContext>()
        .input::<TypedInput>()
        .output::<i32>()
        .handler(|ctx: TypedContext, input: TypedInput| async move {
            Ok(ctx.value * input.multiplier)
        });

    let ctx = TypedContext { value: 7 };
    let input = serde_json::json!({ "multiplier": 6 });
    let result = proc.call(ctx, input).await;

    assert!(result.is_ok());
    match result.unwrap() {
        OutputKind::Single(value) => {
            let product: i32 = serde_json::from_value(value).unwrap();
            assert_eq!(product, 42);
        }
        OutputKind::Stream(_) => panic!("Expected Single output"),
    }
}
