use anyhow::Result;
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection};

use crate::model::dialect::Dialect;

/// Connect to database based on URL scheme.
/// Supports PostgreSQL (`postgres://`) and SQLite (`sqlite://`).
pub async fn connect(database_url: &str) -> Result<DatabaseConnection> {
    let dialect = Dialect::from_url(database_url);
    let mut opt = ConnectOptions::new(database_url.to_string());

    match dialect {
        Dialect::Sqlite => {
            opt.max_connections(1).sqlx_logging(false);
        }
        Dialect::Postgres => {
            opt.max_connections(20)
                .min_connections(2)
                .sqlx_logging(false);
        }
    }

    let db = Database::connect(opt).await?;

    // SQLite: enable WAL mode for concurrent reads
    if dialect == Dialect::Sqlite {
        db.execute_unprepared("PRAGMA journal_mode=WAL").await?;
        db.execute_unprepared("PRAGMA synchronous=NORMAL").await?;
    }

    tracing::info!(
        dialect = ?dialect,
        "Database connected"
    );

    Ok(db)
}

/// Detect dialect from the current database connection URL.
pub fn dialect_from_url(url: &str) -> Dialect {
    Dialect::from_url(url)
}
