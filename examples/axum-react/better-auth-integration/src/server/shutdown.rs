use tokio::signal;

/// Handles graceful shutdown signals (Ctrl+C or SIGTERM).
/// SRP: Isolated signal handling logic.
pub async fn shutdown_signal() {
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
        _ = ctrl_c => println!("\n🛑 Received Ctrl+C, shutting down gracefully..."),
        _ = terminate => println!("\n🛑 Received termination signal, shutting down gracefully..."),
    }
}
