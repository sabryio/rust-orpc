//! Basic example of using orpc-axum to create a type-safe Axum API

use orpc_axum::AxumRouter;
use orpc_core::{os, OrpcError, Procedure, ProcedureRegistry, Router};
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

// Define your API as nested structs matching your contract shape
struct ApiRouter {
    ping: Procedure<AppContext, (), String>,
    greet: Procedure<AppContext, GreetInput, GreetOutput>,
}

// Implement the Router trait to register procedures
impl Router<AppContext> for ApiRouter {
    fn register_procedures(&self, prefix: &str, registry: &mut ProcedureRegistry<AppContext>) {
        let ping_path = if prefix.is_empty() {
            "ping".to_string()
        } else {
            format!("{}/ping", prefix)
        };
        registry.insert(ping_path, &self.ping);

        let greet_path = if prefix.is_empty() {
            "greet".to_string()
        } else {
            format!("{}/greet", prefix)
        };
        registry.insert(greet_path, &self.greet);
    }
}

#[tokio::main]
async fn main() {
    // Create your router with type-safe procedures
    let router = ApiRouter {
        ping: os()
            .context::<AppContext>()
            .output::<String>()
            .handler(|_ctx: AppContext, _: ()| async move { Ok("pong".to_string()) }),
        
        greet: os()
            .context::<AppContext>()
            .input::<GreetInput>()
            .output::<GreetOutput>()
            .handler(|ctx: AppContext, input: GreetInput| async move {
                if input.name.is_empty() {
                    return Err(OrpcError::bad_request("Name cannot be empty"));
                }
                Ok(GreetOutput {
                    message: format!("{}, {}!", ctx.greeting, input.name),
                })
            }),
    };

    // Create context
    let ctx = AppContext {
        greeting: "Hello".to_string(),
    };

    // Convert to Axum router - automatically registers all procedures
    let app = router.into_axum_router(ctx);

    // Start server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    
    println!("🚀 Server running on http://127.0.0.1:3000");
    println!("   POST /ping");
    println!("   POST /greet");

    axum::serve(listener, app).await.unwrap();
}
