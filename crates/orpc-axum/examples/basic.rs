//! Basic example: define a type-safe Axum API with orpc using the router! macro.
//!
//! Clean, declarative syntax that mirrors TypeScript oRPC.

use orpc_axum::AxumRouter;
use orpc_core::{os, router, OrpcError};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
struct AppContext {
    greeting: String,
}

#[derive(Deserialize)]
struct GreetInput {
    name: String,
}

#[derive(Serialize)]
struct GreetOutput {
    message: String,
}

#[derive(Deserialize)]
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
            .output::<String>()
            .handler(|_ctx, _: ()| async { Ok("pong".to_string()) }),

        greet: os()
            .context::<AppContext>()
            .input::<GreetInput>()
            .output::<GreetOutput>()
            .handler(|ctx, input: GreetInput| async move {
                if input.name.is_empty() {
                    return Err(OrpcError::bad_request("Name cannot be empty"));
                }
                Ok(GreetOutput {
                    message: format!("{}, {}!", ctx.greeting, input.name),
                })
            }),

        planet: {
            list: os()
                .context::<AppContext>()
                .output::<Vec<Planet>>()
                .handler(|_ctx, _: ()| async move {
                    Ok(vec![
                        Planet { id: 1, name: "Earth".to_string() },
                        Planet { id: 2, name: "Mars".to_string() },
                    ])
                }),

            find: os()
                .context::<AppContext>()
                .input::<FindInput>()
                .output::<Planet>()
                .handler(|_ctx, input: FindInput| async move {
                    match input.id {
                        1 => Ok(Planet { id: 1, name: "Earth".to_string() }),
                        _ => Err(OrpcError::not_found(format!("Planet {} not found", input.id))),
                    }
                })
        }
    }
    .into_axum_router(AppContext {
        greeting: "Hello".to_string(),
    });

    println!("🚀 Server running on http://127.0.0.1:3000");
    println!("   POST /ping");
    println!("   POST /greet");
    println!("   POST /planet/list");
    println!("   POST /planet/find");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
