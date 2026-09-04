//! Request Inspector Server
//!
//! A debugging server that logs all incoming requests regardless of method or path.
//! Useful for debugging client-server communication, auth flows, and API calls.
//!
//! Run with:
//!   cargo run --bin inspector
//!
//! Then send any request:
//!   curl -X POST http://localhost:3002/any/path -H "Content-Type: application/json" -d '{"test": 123}'
//!   curl -X GET http://localhost:3002/api/test?foo=bar

use axum::{
    body::Bytes,
    extract::{Path, Query},
    http::{HeaderMap, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use colored::*;
use serde_json::Value;
use std::collections::HashMap;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() {
    println!("{}", "🔍 Request Inspector Server".bright_cyan().bold());
    println!("{}", "============================".cyan());
    println!();
    println!(
        "{} {}",
        "Listening on:".green(),
        "http://localhost:3002".bright_white().underline()
    );
    println!("{}", "Logs ALL requests with full details".yellow());
    println!();
    println!("{}", "Examples:".bright_white());
    println!("  {}", "curl http://localhost:3002/test".bright_black());
    println!("  {}", "curl -X POST http://localhost:3002/api/users -H 'Content-Type: application/json' -d '{\"name\":\"test\"}'".bright_black());
    println!(
        "  {}",
        "curl http://localhost:3002/anything?foo=bar&baz=qux".bright_black()
    );
    println!();
    println!("{}", "Press Ctrl+C to stop".dimmed());
    println!();
    println!(
        "{}",
        "─────────────────────────────────────────────────────────".dimmed()
    );
    println!();

    let app = Router::new()
        // Catch all paths with all methods
        .route("/{*path}", any(inspect_handler))
        // Fallback for root path
        .fallback(inspect_root)
        // CORS for development - allow all origins
        .layer(
            CorsLayer::new()
                .allow_origin([
                    "http://localhost:3000"
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                    "http://127.0.0.1:3000"
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                    "http://localhost:5173"
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                    "http://127.0.0.1:5173"
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                ])
                .allow_methods([
                    axum::http::Method::GET,
                    axum::http::Method::POST,
                    axum::http::Method::PUT,
                    axum::http::Method::PATCH,
                    axum::http::Method::DELETE,
                    axum::http::Method::OPTIONS,
                ])
                .allow_headers([
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::AUTHORIZATION,
                    axum::http::header::ACCEPT,
                ])
                .allow_credentials(true),
        );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3002")
        .await
        .unwrap();

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    println!("\n{}", "✨ Server shutdown complete".green());
}

async fn inspect_handler(
    method: Method,
    Path(path): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    log_request(&method, &format!("/{}", path), &query, &headers, &body);
    build_response(&method, &format!("/{}", path))
}

async fn inspect_root(
    method: Method,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    log_request(&method, "/", &query, &headers, &body);
    build_response(&method, "/")
}

fn log_request(
    method: &Method,
    path: &str,
    query: &HashMap<String, String>,
    headers: &HeaderMap,
    body: &Bytes,
) {
    // Method color based on HTTP verb
    let method_colored = match method.as_str() {
        "GET" => method.as_str().bright_green(),
        "POST" => method.as_str().bright_blue(),
        "PUT" => method.as_str().bright_yellow(),
        "PATCH" => method.as_str().bright_magenta(),
        "DELETE" => method.as_str().bright_red(),
        _ => method.as_str().white(),
    };

    println!(
        "{}",
        "╔═══════════════════════════════════════════════════════════".bright_black()
    );
    println!(
        "{} {} {}",
        "║".bright_black(),
        method_colored.bold(),
        path.bright_white().bold()
    );
    println!(
        "{}",
        "╠═══════════════════════════════════════════════════════════".bright_black()
    );

    // Query parameters
    if !query.is_empty() {
        println!("{}", "║".bright_black());
        println!(
            "{} {}",
            "║".bright_black(),
            "Query Parameters:".cyan().bold()
        );
        for (key, value) in query {
            println!(
                "{} {} {} {}",
                "║".bright_black(),
                "  ".normal(),
                key.yellow(),
                format!("= {}", value).white()
            );
        }
    }

    // Headers
    println!("{}", "║".bright_black());
    println!("{} {}", "║".bright_black(), "Headers:".cyan().bold());
    let mut header_vec: Vec<_> = headers.iter().collect();
    header_vec.sort_by_key(|(name, _)| name.as_str());

    for (name, value) in header_vec {
        let value_str = value.to_str().unwrap_or("<non-utf8>");
        println!(
            "{} {} {}: {}",
            "║".bright_black(),
            "  ".normal(),
            name.as_str().bright_magenta(),
            value_str.white()
        );
    }

    // Body
    if !body.is_empty() {
        println!("{}", "║".bright_black());
        println!(
            "{} {} {}",
            "║".bright_black(),
            "Body".cyan().bold(),
            format!("({} bytes):", body.len()).dimmed()
        );

        // Try to parse as JSON
        if let Ok(json) = serde_json::from_slice::<Value>(body) {
            let pretty = serde_json::to_string_pretty(&json).unwrap();
            for line in pretty.lines() {
                println!(
                    "{} {} {}",
                    "║".bright_black(),
                    "  ".normal(),
                    line.bright_white()
                );
            }
        } else {
            // Try as UTF-8 text
            match std::str::from_utf8(body) {
                Ok(text) => {
                    for line in text.lines() {
                        println!("{} {} {}", "║".bright_black(), "  ".normal(), line.white());
                    }
                }
                Err(_) => {
                    println!(
                        "{} {} {}",
                        "║".bright_black(),
                        "  ".normal(),
                        format!("<binary data: {} bytes>", body.len()).red()
                    );
                    println!(
                        "{} {} {}: {}",
                        "║".bright_black(),
                        "  ".normal(),
                        "Hex".dimmed(),
                        hex_preview(body, 64).bright_black()
                    );
                }
            }
        }
    } else {
        println!("{}", "║".bright_black());
        println!(
            "{} {} {}",
            "║".bright_black(),
            "Body:".cyan().bold(),
            "<empty>".dimmed()
        );
    }

    println!(
        "{}",
        "╚═══════════════════════════════════════════════════════════".bright_black()
    );
    println!();
}

fn build_response(method: &Method, path: &str) -> Response {
    let response_body = serde_json::json!({
        "inspector": "ok",
        "method": method.as_str(),
        "path": path,
        "message": "Request inspected successfully"
    });

    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        serde_json::to_string_pretty(&response_body).unwrap(),
    )
        .into_response()
}

fn hex_preview(bytes: &[u8], max_len: usize) -> String {
    let preview_bytes = &bytes[..bytes.len().min(max_len)];
    let hex: Vec<String> = preview_bytes.iter().map(|b| format!("{:02x}", b)).collect();
    let result = hex.join(" ");

    if bytes.len() > max_len {
        format!("{} ... ({} more bytes)", result, bytes.len() - max_len)
    } else {
        result
    }
}

async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            println!("\n{} {}", "🛑".red(), "Received Ctrl+C, shutting down gracefully...".yellow());
        },
        _ = terminate => {
            println!("\n{} {}", "🛑".red(), "Received termination signal, shutting down gracefully...".yellow());
        },
    }
}
