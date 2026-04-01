//! In-process hapihub runtime for embedded mode.
//! NO Axum, NO HTTP — direct function calls via IPC adapter.

use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection};
use crate::transport::ipc::{IpcRequest, IpcResponse};

/// Embedded hapihub instance for Tauri apps.
/// Uses IPC adapter directly — no Axum, no HTTP, no network.
pub struct HapihubEmbedded {
    db: DatabaseConnection,
}

impl HapihubEmbedded {
    /// Create a new embedded hapihub instance with SQLite backend.
    pub async fn new(db_path: &str) -> Result<Self, anyhow::Error> {
        let db_url = format!("sqlite:{}?mode=rwc", db_path);

        // Single connection for embedded SQLite
        let mut opt = ConnectOptions::new(db_url);
        opt.max_connections(1)
            .sqlx_logging(false);

        let db = Database::connect(opt).await?;

        // Enable WAL mode with mobile-safe settings
        db.execute_unprepared("PRAGMA journal_mode=WAL").await?;
        db.execute_unprepared("PRAGMA wal_autocheckpoint=1000").await?;
        db.execute_unprepared("PRAGMA synchronous=NORMAL").await?;

        // Create auth tables
        crate::auth::session::ensure_auth_tables(&db).await?;

        Ok(Self { db })
    }

    /// Force WAL checkpoint — call from iOS lifecycle events.
    pub async fn checkpoint(&self) -> Result<(), anyhow::Error> {
        self.db.execute_unprepared("PRAGMA wal_checkpoint(TRUNCATE)").await?;
        Ok(())
    }

    /// Handle IPC request — NO HTTP, NO NETWORK.
    /// Routes to handlers based on method + path.
    pub async fn request(&self, request: IpcRequest) -> IpcResponse {
        // For now, simple path-based routing to internal handlers
        match (request.method.as_str(), request.path.as_str()) {
            ("GET", "/livez") => {
                IpcResponse::success(serde_json::json!({"status": "pass"}))
            }
            ("GET", "/readyz") => {
                IpcResponse::success(serde_json::json!({"status": "pass"}))
            }
            _ => IpcResponse::not_found(),
        }
    }
}
