use clap::Parser;
use tracing::info;

mod config;
mod error;
mod db;
mod model;
mod auth;
mod context;
mod service;
mod handlers;
mod middleware;
mod transport;
mod generated;

#[cfg(feature = "embedded")]
mod embedded;

use config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file (ignore if missing)
    dotenvy::dotenv().ok();

    // Parse CLI args + env vars
    let config = Config::parse();

    // Initialize tracing
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| config.log_level.clone().into());

    if config.log_pretty {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .pretty()
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .json()
            .flatten_event(true)
            .init();
    }

    info!("Starting Monobase API (Rust)");

    // Connect to database
    let db = db::connect(&config.database_url).await?;
    info!("Database connected");

    // Build app state
    let state = handlers::AppState::new(db, config.clone());

    // Create and start HTTP server
    let router = transport::http::create_router(state);
    let addr = format!("{}:{}", config.server_host, config.server_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Listening on {}", addr);

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Received Ctrl+C, shutting down"),
        _ = terminate => info!("Received SIGTERM, shutting down"),
    }
}
