use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::config::Config;
use crate::model::dialect::Dialect;

pub mod auth;
pub mod generic;
pub mod internal;
#[macro_use]
pub mod macros;

/// Shared application state -- holds DB connection, config, and dialect.
#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub config: Config,
    pub dialect: Dialect,
}

impl AppState {
    pub fn new(db: DatabaseConnection, config: Config, dialect: Dialect) -> Arc<Self> {
        Arc::new(Self { db, config, dialect })
    }
}
