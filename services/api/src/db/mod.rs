use anyhow::Result;
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection};

pub mod entities;

/// Connect to database based on URI scheme
pub async fn connect(database_url: &str) -> Result<DatabaseConnection> {
    let mut opt = ConnectOptions::new(database_url.to_string());

    if database_url.starts_with("sqlite") {
        // SQLite: single connection, WAL mode
        opt.max_connections(1)
            .sqlx_logging(false);
    } else {
        // PostgreSQL: connection pool
        opt.max_connections(10)
            .min_connections(2)
            .sqlx_logging(false);
    }

    let db = Database::connect(opt).await?;

    // Enable WAL mode for SQLite
    if database_url.starts_with("sqlite") {
        db.execute_unprepared("PRAGMA journal_mode=WAL").await?;
        db.execute_unprepared("PRAGMA synchronous=NORMAL").await?;
    }

    tracing::info!("Database connected: {}", if database_url.starts_with("sqlite") { "SQLite" } else { "PostgreSQL" });

    Ok(db)
}
