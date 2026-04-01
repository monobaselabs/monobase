pub mod audit;
pub mod auth;
pub mod billing;
pub mod booking;
pub mod comms;
pub mod email;
pub mod emr;
pub mod notifs;
pub mod patient;
pub mod person;
pub mod provider;
pub mod reviews;
pub mod storage;

use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::config::Config;
use crate::model::dialect::Dialect;
use crate::service::ws::WebSocketService;

/// Shared application state — injected into all handlers via `State<Arc<AppState>>`.
#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub config: Config,
    pub dialect: Dialect,
    pub ws: Arc<WebSocketService>,
}

impl AppState {
    pub fn new(db: DatabaseConnection, config: Config) -> Self {
        let dialect = Dialect::from_url(&config.database_url);
        Self {
            db,
            config,
            dialect,
            ws: Arc::new(WebSocketService::new()),
        }
    }
}
