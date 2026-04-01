use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod auth;
mod config;
mod context;
mod db;
mod error;
mod generated;
mod handlers;
mod middleware;
mod model;
mod service;
mod transport;

#[cfg(feature = "embedded")]
mod embedded;

/// monobase-api: Generic CRUD API framework
#[derive(Parser, Debug)]
#[command(name = "monobase-api", version, about)]
struct Cli {
    /// Port to listen on
    #[arg(short, long, env = "PORT", default_value = "7600")]
    port: u16,

    /// Database URL
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,

    /// Log level
    #[arg(long, env = "RUST_LOG", default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new(&cli.log_level)
        }))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = config::Config::new(cli.port, cli.database_url.clone());
    let dialect = model::dialect::Dialect::from_url(&config.database_url);
    let db = db::connect(&config.database_url).await?;

    // Ensure auth tables exist
    auth::session::ensure_auth_tables(&db).await?;

    // Discover table columns from information_schema (for generic CRUD handler)
    handlers::generic::discover_table_columns(&db, dialect).await;

    // Build application state
    let state = handlers::AppState::new(db, config.clone(), dialect);

    // Build HTTP router and start server
    let app = transport::http::create_router(state);
    let addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("monobase-api listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
