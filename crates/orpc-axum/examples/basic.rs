//! Basic example: define a type-safe Axum API with orpc using the router! macro.
//!
//! Clean, declarative syntax that mirrors TypeScript oRPC.
//! Each procedure declares its HTTP method and absolute path via .route().

use orpc_axum::AxumRouter;
use orpc_core::{os, router, HttpMethod, OrpcContext, OrpcError, OrpcJson};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
struct AppContext {
    greeting: String,
}

#[derive(Deserialize, Serialize)]
struct GreetInput {
    name: String,
}

#[derive(Serialize)]
struct GreetOutput {
    message: String,
}

#[derive(Deserialize, Serialize)]
struct FindInput {
    id: i32,
}

#[derive(Serialize)]
struct Planet {
    id: i32,
    name: String,
}

#[tokio::main]
async fn main() {
    let app = router! {
        ping: os()
            .context::<AppContext>()
            .route(HttpMethod::Get, "/ping")
            .output::<String>()
            .handler(async || Ok("pong".to_string()) ),

        greet: os()
            .context::<AppContext>()
            .route(HttpMethod::Post, "/greet")
            .input::<GreetInput>()
            .output::<GreetOutput>()
            .handler(greet),

        planet: {
            list: os()
                .context::<AppContext>()
                .route(HttpMethod::Get, "/planet")
                .output::<Vec<Planet>>()
                .handler(list_planets),

            find: os()
                .context::<AppContext>()
                .route(HttpMethod::Post, "/planet/find")
                .input::<FindInput>()
                .output::<Planet>()
                .handler(|OrpcContext(_ctx): OrpcContext<AppContext>, OrpcJson(input): OrpcJson<FindInput>| async move {
                    match input.id {
                        1 => Ok(Planet {
                            id: 1,
                            name: "Earth".to_string(),
                        }),
                        _ => Err(OrpcError::not_found(format!(
                            "Planet {} not found",
                            input.id
                        ))),
                    }
                })
        }
    }
    .into_axum_router(AppContext {
        greeting: "Hello".to_string(),
    });

    println!("🚀 Server running on http://127.0.0.1:3000");
    println!("   GET  /ping");
    println!("   POST /greet");
    println!("   GET  /planet");
    println!("   POST /planet/find");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn greet(
    OrpcContext(ctx): OrpcContext<AppContext>,
    OrpcJson(input): OrpcJson<GreetInput>,
) -> Result<GreetOutput, OrpcError> {
    if input.name.is_empty() {
        return Err(OrpcError::bad_request("Name cannot be empty"));
    }
    Ok(GreetOutput {
        message: format!("{}, {}!", ctx.greeting, input.name),
    })
}

async fn list_planets(
    OrpcContext(_ctx): OrpcContext<AppContext>,
) -> Result<Vec<Planet>, OrpcError> {
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
}
