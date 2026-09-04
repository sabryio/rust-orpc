//! Integration test: orpc router to Axum router conversion

use axum::{body::Body, http::{Request, StatusCode}};
use orpc_axum::AxumRouter;
use orpc_core::{os, OrpcError, Procedure, ProcedureRegistry, Router};
use serde::{Deserialize, Serialize};
use tower::ServiceExt;

#[derive(Clone)]
struct AppContext {
    prefix: String,
}

#[derive(Deserialize)]
struct FindInput {
    id: i32,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Planet {
    id: i32,
    name: String,
}

struct PlanetRouter {
    list: Procedure<AppContext, (), Vec<Planet>>,
    find: Procedure<AppContext, FindInput, Planet>,
}

impl Router<AppContext> for PlanetRouter {
    fn register_procedures(&self, prefix: &str, registry: &mut ProcedureRegistry<AppContext>) {
        let list_path = if prefix.is_empty() {
            "list".to_string()
        } else {
            format!("{}/list", prefix)
        };
        registry.insert(list_path, &self.list);

        let find_path = if prefix.is_empty() {
            "find".to_string()
        } else {
            format!("{}/find", prefix)
        };
        registry.insert(find_path, &self.find);
    }
}

struct ApiRouter {
    ping: Procedure<AppContext, (), String>,
    planet: PlanetRouter,
}

impl Router<AppContext> for ApiRouter {
    fn register_procedures(&self, prefix: &str, registry: &mut ProcedureRegistry<AppContext>) {
        let ping_path = if prefix.is_empty() {
            "ping".to_string()
        } else {
            format!("{}/ping", prefix)
        };
        registry.insert(ping_path, &self.ping);

        let planet_prefix = if prefix.is_empty() {
            "planet".to_string()
        } else {
            format!("{}/planet", prefix)
        };
        self.planet.register_procedures(&planet_prefix, registry);
    }
}

#[tokio::test]
async fn test_into_axum_router() {
    let router = ApiRouter {
        ping: os()
            .context::<AppContext>()
            .output::<String>()
            .handler(|ctx: AppContext, _: ()| async move {
                Ok(format!("{} pong", ctx.prefix))
            }),
        planet: PlanetRouter {
            list: os()
                .context::<AppContext>()
                .output::<Vec<Planet>>()
                .handler(|_ctx: AppContext, _: ()| async move {
                    Ok(vec![
                        Planet { id: 1, name: "Earth".to_string() },
                        Planet { id: 2, name: "Mars".to_string() },
                    ])
                }),
            find: os()
                .context::<AppContext>()
                .input::<FindInput>()
                .output::<Planet>()
                .handler(|_ctx: AppContext, input: FindInput| async move {
                    match input.id {
                        1 => Ok(Planet { id: 1, name: "Earth".to_string() }),
                        2 => Ok(Planet { id: 2, name: "Mars".to_string() }),
                        _ => Err(OrpcError::not_found(format!("Planet {} not found", input.id))),
                    }
                }),
        },
    };

    let ctx = AppContext {
        prefix: "test".to_string(),
    };

    let app = router.into_axum_router(ctx);

    // Test ping endpoint
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/ping")
                .header("content-type", "application/json")
                .body(Body::from("null"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: String = serde_json::from_slice(&body).unwrap();
    assert_eq!(result, "test pong");

    // Test planet/list endpoint
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/planet/list")
                .header("content-type", "application/json")
                .body(Body::from("null"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: Vec<Planet> = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].name, "Earth");

    // Test planet/find endpoint
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/planet/find")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"id": 1}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: Planet = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.id, 1);
    assert_eq!(result.name, "Earth");
}

#[tokio::test]
async fn test_error_handling() {
    let router = ApiRouter {
        ping: os()
            .context::<AppContext>()
            .output::<String>()
            .handler(|_ctx: AppContext, _: ()| async move { Ok("pong".to_string()) }),
        planet: PlanetRouter {
            list: os()
                .context::<AppContext>()
                .output::<Vec<Planet>>()
                .handler(|_ctx: AppContext, _: ()| async move { Ok(vec![]) }),
            find: os()
                .context::<AppContext>()
                .input::<FindInput>()
                .output::<Planet>()
                .handler(|_ctx: AppContext, input: FindInput| async move {
                    Err(OrpcError::not_found(format!("Planet {} not found", input.id)))
                }),
        },
    };

    let ctx = AppContext {
        prefix: "test".to_string(),
    };

    let app = router.into_axum_router(ctx);

    // Test error response
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/planet/find")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"id": 999}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let error: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(error["code"], "NOT_FOUND");
    assert!(error["message"].as_str().unwrap().contains("Planet 999 not found"));
}
