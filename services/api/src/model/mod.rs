pub mod dialect;
pub mod query;

use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, FromQueryResult, JsonValue, Statement,
};
use serde::Serialize;

use crate::error::ApiError;
use dialect::Dialect;

/// Paginated result returned by list queries.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResult<T: Serialize> {
    pub data: Vec<T>,
    pub pagination: PaginationMeta,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationMeta {
    pub offset: u64,
    pub limit: u64,
    pub count: u64,
    pub total_count: u64,
    pub total_pages: u64,
    pub current_page: u64,
    pub has_next_page: bool,
    pub has_previous_page: bool,
}

impl PaginationMeta {
    pub fn new(offset: u64, limit: u64, count: u64, total_count: u64) -> Self {
        let total_pages = if limit > 0 {
            (total_count + limit - 1) / limit
        } else {
            0
        };
        let current_page = if limit > 0 { offset / limit + 1 } else { 1 };

        Self {
            offset,
            limit,
            count,
            total_count,
            total_pages,
            current_page,
            has_next_page: offset + count < total_count,
            has_previous_page: offset > 0,
        }
    }
}

/// Configuration for a model — defines table structure for generic CRUD.
#[derive(Debug, Clone)]
pub struct ModelConfig {
    pub table_name: String,
    pub known_columns: Vec<String>,
    pub json_columns: Vec<String>,
    pub timestamp_columns: Vec<String>,
}

/// Generic model for table CRUD operations.
/// Dialect-aware, works with both PostgreSQL and SQLite.
pub struct BaseModel {
    pub config: ModelConfig,
    pub dialect: Dialect,
}

impl BaseModel {
    pub fn new(config: ModelConfig, dialect: Dialect) -> Self {
        Self { config, dialect }
    }

    /// Find a record by ID.
    pub async fn find_by_id(
        &self,
        db: &DatabaseConnection,
        id: &str,
    ) -> Result<Option<JsonValue>, ApiError> {
        let sql = format!(
            "SELECT * FROM \"{}\" WHERE id = $1",
            self.config.table_name
        );
        let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
            self.dialect.sea_orm_backend(),
            &sql,
            vec![id.into()],
        ))
        .one(db)
        .await
        .map_err(ApiError::Database)?;

        Ok(result)
    }

    /// Count records matching optional WHERE clause.
    pub async fn count(
        &self,
        db: &DatabaseConnection,
        where_clause: &str,
        params: Vec<sea_orm::Value>,
    ) -> Result<u64, ApiError> {
        let sql = if where_clause.is_empty() {
            format!("SELECT COUNT(*) as count FROM \"{}\"", self.config.table_name)
        } else {
            format!(
                "SELECT COUNT(*) as count FROM \"{}\" WHERE {}",
                self.config.table_name, where_clause
            )
        };

        let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
            self.dialect.sea_orm_backend(),
            &sql,
            params,
        ))
        .one(db)
        .await
        .map_err(ApiError::Database)?;

        let count = result
            .and_then(|v| v.get("count").and_then(|c| c.as_u64()))
            .unwrap_or(0);

        Ok(count)
    }

    /// Delete a record by ID.
    pub async fn delete_by_id(
        &self,
        db: &DatabaseConnection,
        id: &str,
    ) -> Result<(), ApiError> {
        let sql = format!("DELETE FROM \"{}\" WHERE id = $1", self.config.table_name);
        db.execute(Statement::from_sql_and_values(
            self.dialect.sea_orm_backend(),
            &sql,
            vec![id.into()],
        ))
        .await
        .map_err(ApiError::Database)?;

        Ok(())
    }
}
