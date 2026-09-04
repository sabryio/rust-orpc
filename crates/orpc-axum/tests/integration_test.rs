//! Integration test: orpc router to Axum router conversion using RouterBuilder

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use orpc_axum::AxumRouter;
use orpc_core::{os, r, OrpcError};
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

#[tokio::test]
async fn test_into_axum_router() {
    let planet = r()
        .add(
            "list",
            os().context::<AppContext>()
                .output::<Vec<Planet>>()
                .handler(|_ctx: AppContext, _: ()| async move {
                    Ok(vec![
                        Planet {
                            id: 1,
                            name: "Earth".to_string(),
                        },
                        Planet {
                            id: 2,
                            name: "Mars".to_string(),
                        },
                    ])
                }),
        )
        .add(
            "find",
            os().context::<AppContext>()
                .input::<FindInput>()
                .output::<Planet>()
                .handler(|_ctx: AppContext, input: FindInput| async move {
                    match input.id {
                        1 => Ok(Planet {
                            id: 1,
                            name: "Earth".to_string(),
                        }),
                        2 => Ok(Planet {
                            id: 2,
                            name: "Mars".to_string(),
                        }),
                        _ => Err(OrpcError::not_found(format!(
                            "Planet {} not found",
                            input.id
                        ))),
                    }
                }),
        );

    let api = r()
        .add(
            "ping",
            os().context::<AppContext>().output::<String>().handler(
                |ctx: AppContext, _: ()| async move { Ok(format!("{} pong", ctx.prefix)) },
            ),
        )
        .nest("planet", planet);

    let app = api.into_axum_router(AppContext {
        prefix: "test".to_string(),
    });

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
    let app = r()
        .add(
            "ping",
            os().context::<AppContext>()
                .output::<String>()
                .handler(|_ctx: AppContext, _: ()| async { Ok("pong".to_string()) }),
        )
        .nest(
            "planet",
            r().add(
                "list",
                os().context::<AppContext>()
                    .output::<Vec<Planet>>()
                    .handler(|_ctx: AppContext, _: ()| async { Ok(vec![]) }),
            )
            .add(
                "find",
                os().context::<AppContext>()
                    .input::<FindInput>()
                    .output::<Planet>()
                    .handler(|_ctx: AppContext, input: FindInput| async move {
                        Err(OrpcError::not_found(format!(
                            "Planet {} not found",
                            input.id
                        )))
                    }),
            ),
        )
        .into_axum_router(AppContext {
            prefix: "test".to_string(),
        });

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
    assert!(error["message"]
        .as_str()
        .unwrap()
        .contains("Planet 999 not found"));
}

#[tokio::test]
async fn test_deep_nesting() {
    let app = r()
        .nest(
            "api",
            r().nest(
                "v1",
                r().add(
                    "status",
                    os().context::<AppContext>()
                        .output::<String>()
                        .handler(|_ctx: AppContext, _: ()| async { Ok("ok".to_string()) }),
                ),
            ),
        )
        .into_axum_router(AppContext {
            prefix: "".to_string(),
        });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/status")
                .header("content-type", "application/json")
                .body(Body::from("null"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
