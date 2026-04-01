//! Model layer — data access abstraction.
//!
//! Port of `services/hapihub/src/utils/drizzle/pg-service.ts`.
//! `PostgresModel` provides table-driven CRUD with:
//! - MongoDB-style query translation
//! - `_data` JSONB overflow for schemaless fields
//! - snake_case ↔ camelCase column mapping
//! - PG / SQLite dialect abstraction

pub mod dialect;
pub mod overflow;
pub mod query;

use dialect::Dialect;
use overflow::{collect_overflow, from_row, to_snake};
use query::{extract_meta_params, translate_query, QueryOpts};
use sea_orm::{ConnectionTrait, DatabaseConnection};
use sea_orm::sea_query::{Alias, Condition, Expr, Query as SeaQuery};
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use std::sync::Arc;

use crate::error::ApiError;

/// Paginated result returned by `find()`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PaginatedResult {
    pub data: Vec<Value>,
    pub total: i64,
    pub limit: i64,
    pub skip: i64,
}

/// Configuration for creating a PostgresModel.
pub struct ModelConfig {
    pub table_name: String,
    pub known_columns: Vec<String>,
    pub id_alias: Option<String>,
    pub json_array_columns: Vec<String>,
    pub scalar_array_columns: Vec<String>,
    /// Columns that are JSONB type (for proper query casting)
    pub jsonb_columns: Vec<String>,
    /// Default values to set on create if not provided.
    pub defaults: Vec<(String, Value)>,
    /// Columns that are timestamp type (from information_schema)
    pub timestamp_columns: Vec<String>,
    /// Reverse column mapping (unused for now)
    pub column_name_map: Vec<(String, String)>,
}

/// Generic data-access model for a database table.
///
/// Provides CRUD operations with query translation, `_data` overflow,
/// and dialect-aware SQL generation.
#[derive(Clone)]
pub struct PostgresModel {
    pub table_name: String,
    pub(crate) known_columns: HashSet<String>,
    id_alias: Option<String>,
    json_array_columns: HashSet<String>,
    defaults: Vec<(String, Value)>,
    scalar_array_columns: HashSet<String>,
    jsonb_columns: HashSet<String>,
    timestamp_columns: HashSet<String>,
    pub db: DatabaseConnection,
    pub dialect: Dialect,
}

impl PostgresModel {
    pub fn new(db: DatabaseConnection, dialect: Dialect, config: ModelConfig) -> Self {
        Self {
            table_name: config.table_name,
            known_columns: config.known_columns.into_iter().collect(),
            id_alias: config.id_alias,
            json_array_columns: config.json_array_columns.into_iter().collect(),
            scalar_array_columns: config.scalar_array_columns.into_iter().collect(),
            jsonb_columns: config.jsonb_columns.into_iter().collect(),
            timestamp_columns: config.timestamp_columns.into_iter().collect(),
            defaults: config.defaults,
            db,
            dialect,
        }
    }

    fn query_opts(&self) -> QueryOpts {
        QueryOpts {
            known_columns: self.known_columns.clone(),
            json_array_columns: self.json_array_columns.clone(),
            scalar_array_columns: self.scalar_array_columns.clone(),
            jsonb_columns: self.jsonb_columns.clone(),
            dialect: self.dialect,
        }
    }

    // ── FIND ─────────────────────────────────────────────────────────

    /// List records with MongoDB-style query, pagination, sort, field selection.
    pub async fn find(&self, query_value: Value) -> Result<PaginatedResult, ApiError> {
        let query_map = query_value.as_object().cloned().unwrap_or_default();
        let (meta, filter) = extract_meta_params(&query_map);
        let condition = translate_query(&filter, &self.query_opts());

        let limit = meta.limit.unwrap_or(100); // TypeScript default: 100
        let skip = meta.skip.unwrap_or(0);
        let order_by = meta.order_by_clause();
        let table = Alias::new(&self.table_name);

        // Build count query using sea_query
        // (still using raw SQL for row_to_json since sea_query doesn't support it)
        let count_sql = format!(
            "SELECT COUNT(*) as count FROM \"{}\" t",
            self.table_name
        );
        // Build data query
        let data_sql = format!(
            "SELECT row_to_json(t.*) as doc FROM \"{}\" t",
            self.table_name
        );

        // Build the WHERE clause from the condition
        let (mut where_clause, mut where_params) = self.condition_to_sql(&condition);

        // Add $search support — ILIKE on name-like columns and _data->>'name'
        if let Some(ref search_text) = meta.search {
            let param_idx = where_params.len() + 1;
            let like_pattern = format!("%{}%", search_text);
            // Search across name, generic_name, brand_name columns (if they exist)
            // and _data->>'name' for overflow fields
            let mut search_parts = Vec::new();
            for col in &["name", "generic_name", "brand_name", "title", "description"] {
                if self.known_columns.contains(*col) {
                    search_parts.push(format!("t.\"{}\" ILIKE ${}", col, param_idx));
                }
            }
            // Also search _data->>'name' for overflow
            search_parts.push(format!("t.\"_data\"->>'name' ILIKE ${}", param_idx));

            if !search_parts.is_empty() {
                let search_condition = format!("({})", search_parts.join(" OR "));
                where_params.push(json!(like_pattern));
                if where_clause.is_empty() {
                    where_clause = search_condition;
                } else {
                    where_clause = format!("({}) AND {}", where_clause, search_condition);
                }
            }
        }

        let count_full = if where_clause.is_empty() {
            count_sql
        } else {
            format!("{} WHERE {}", count_sql, where_clause)
        };

        let data_full = if where_clause.is_empty() {
            format!("{} ORDER BY {} LIMIT {} OFFSET {}", data_sql, order_by, limit, skip)
        } else {
            format!("{} WHERE {} ORDER BY {} LIMIT {} OFFSET {}", data_sql, where_clause, order_by, limit, skip)
        };

        let count_rows = self.execute_query(&count_full, &where_params).await?;
        let total: i64 = count_rows
            .first()
            .and_then(|r| {
                use sea_orm::TryGetable;
                r.try_get::<i64>("", "count").ok()
            })
            .unwrap_or(0);

        let data_rows = self.execute_query(&data_full, &where_params).await?;
        let data = self.rows_to_values(&data_rows);

        Ok(PaginatedResult { data, total, limit, skip })
    }

    // ── GET ──────────────────────────────────────────────────────────

    /// Get a single record by ID.
    pub async fn get(&self, id: &str) -> Result<Option<Value>, ApiError> {
        let sql = format!(
            "SELECT row_to_json(t.*) as doc FROM \"{}\" t WHERE t.id = $1 LIMIT 1",
            self.table_name
        );
        let rows = self.execute_query(&sql, &[json!(id)]).await?;
        Ok(rows.first().map(|r| self.row_to_value(r)))
    }

    // ── CREATE ───────────────────────────────────────────────────────

    /// Insert a new record (or batch of records). Unknown fields go to `_data` overflow.
    pub async fn create(&self, data: Value) -> Result<Value, ApiError> {
        // Support batch create: if data is an array, create each item
        if let Some(arr) = data.as_array() {
            let mut results = Vec::new();
            for item in arr {
                results.push(self.create_single(item.clone()).await?);
            }
            return Ok(serde_json::json!(results));
        }
        self.create_single(data).await
    }

    async fn create_single(&self, data: Value) -> Result<Value, ApiError> {
        let mut data = data;
        // Apply defaults for missing fields
        if let Some(obj) = data.as_object_mut() {
            for (key, default_val) in &self.defaults {
                if !obj.contains_key(key) {
                    obj.insert(key.clone(), default_val.clone());
                }
            }
        }
        let obj = data.as_object().ok_or_else(|| ApiError::BadRequest("Body must be JSON object".into()))?;

        // Generate ID if not provided
        let id = obj.get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| nanoid::nanoid!());

        let (known, overflow) = collect_overflow(obj, &self.known_columns);

        let mut columns = vec!["\"id\"".to_string()];
        let mut placeholders = vec!["$1".to_string()];
        let mut params: Vec<Value> = vec![json!(id)];
        let mut param_idx = 2;

        // Add known fields (skip timestamps — they're added separately)
        for (key, value) in &known {
            if key == "id" || key == "_data" { continue; }
            let snake = to_snake(key);
            // Skip timestamp columns — handled below with NOW()
            if snake == "created_at" || snake == "updated_at" { continue; }
            columns.push(format!("\"{}\"", snake));
            if value.is_object() || value.is_array() {
                placeholders.push(format!("${}{}", param_idx, self.dialect.json_cast()));
                params.push(json!(value.to_string()));
            } else if self.jsonb_columns.contains(&snake) && value.is_string() {
                // JSONB column with string value — cast to jsonb
                placeholders.push(format!("${}{}", param_idx, self.dialect.json_cast()));
                params.push(json!(format!("\"{}\"", value.as_str().unwrap())));
            } else if self.timestamp_columns.contains(&snake) || is_likely_timestamp_column(&snake) {
                // Timestamp column — handle epoch ms (number) and ISO strings
                if let Some(ms) = value.as_i64() {
                    let secs = ms / 1000;
                    let nanos = ((ms % 1000) * 1_000_000) as u32;
                    if let Some(dt) = chrono::DateTime::from_timestamp(secs, nanos) {
                        placeholders.push(format!("${}::timestamptz", param_idx));
                        params.push(json!(dt.to_rfc3339()));
                    } else {
                        placeholders.push(format!("${}::timestamptz", param_idx));
                        params.push(value.clone());
                    }
                } else if value.is_string() {
                    placeholders.push(format!("${}::timestamptz", param_idx));
                    params.push(value.clone());
                } else {
                    placeholders.push(format!("${}", param_idx));
                    params.push(value.clone());
                }
            } else {
                placeholders.push(format!("${}", param_idx));
                params.push(value.clone());
            }
            param_idx += 1;
        }

        // Add _data overflow
        if !overflow.is_empty() {
            columns.push("\"_data\"".to_string());
            placeholders.push(format!("${}{}", param_idx, self.dialect.json_cast()));
            params.push(json!(Value::Object(overflow).to_string()));
            param_idx += 1;
        }

        // Add timestamps
        let now = self.dialect.now_expr();
        columns.push("\"created_at\"".to_string());
        placeholders.push(now.to_string());
        columns.push("\"updated_at\"".to_string());
        placeholders.push(now.to_string());

        let sql = format!(
            "WITH inserted AS (INSERT INTO \"{}\" ({}) VALUES ({}) RETURNING *) \
             SELECT row_to_json(inserted.*) as doc FROM inserted",
            self.table_name,
            columns.join(", "),
            placeholders.join(", ")
        );

        let rows = self.execute_query(&sql, &params).await.map_err(|e| {
            let msg = e.to_string();
            if msg.contains("unique") || msg.contains("duplicate") {
                ApiError::Conflict("Record already exists".into())
            } else {
                e
            }
        })?;

        rows.first()
            .map(|r| self.row_to_value(r))
            .ok_or_else(|| ApiError::Internal("Insert returned no rows".into()))
    }

    // ── PATCH ────────────────────────────────────────────────────────

    /// Update a record by ID.
    ///
    /// Supports:
    /// - Direct field assignment: `{ name: "new" }`
    /// - `$inc`: `{ $inc: { value: 1 } }` → `col = col + 1`
    /// - `$unset`: `{ $unset: { field: 1 } }` → `col = NULL`
    /// - `$addToSet`: `{ $addToSet: { arr: item } }` → JSONB array append (dedup)
    /// - `$push`: `{ $push: { arr: item } }` → JSONB array append
    /// - Dot-notation: `{ "config.key": val }` → `jsonb_set(col, '{key}', val)`
    /// - Overflow: unknown fields → `_data` JSONB merge
    pub async fn patch(&self, id: &str, data: Value) -> Result<Value, ApiError> {
        let obj = data.as_object().ok_or_else(|| ApiError::BadRequest("Body must be JSON object".into()))?;

        let mut sets = Vec::new();
        let mut params: Vec<Value> = Vec::new();
        let mut param_idx = 1;
        let jc = self.dialect.json_cast();

        // --- Handle $inc ---
        if let Some(inc) = obj.get("$inc").and_then(|v| v.as_object()) {
            for (key, value) in inc {
                let snake = to_snake(key);
                if self.known_columns.contains(&snake) {
                    sets.push(format!("\"{}\" = COALESCE(\"{}\", 0) + ${}", snake, snake, param_idx));
                    // Coerce float-with-no-fraction to integer to avoid PG type mismatch
                    let coerced = if let Some(f) = value.as_f64() {
                        if f.fract() == 0.0 { json!(f as i64) } else { value.clone() }
                    } else {
                        value.clone()
                    };
                    params.push(coerced);
                    param_idx += 1;
                }
            }
        }

        // --- Handle $unset ---
        if let Some(unset) = obj.get("$unset").and_then(|v| v.as_object()) {
            for (key, _) in unset {
                let snake = to_snake(key);
                if self.known_columns.contains(&snake) {
                    sets.push(format!("\"{}\" = NULL", snake));
                } else {
                    // Unset from _data overflow
                    sets.push(format!("\"_data\" = \"_data\" - '{}'", key));
                }
            }
        }

        // --- Handle $addToSet ---
        if let Some(add) = obj.get("$addToSet").and_then(|v| v.as_object()) {
            for (key, value) in add {
                let snake = to_snake(key);
                if self.known_columns.contains(&snake) {
                    // Append to JSONB array (with coalesce for NULL columns)
                    sets.push(format!(
                        "\"{}\" = COALESCE(\"{}\", '[]'{}) || ${}{}",
                        snake, snake, jc, param_idx, jc
                    ));
                    // Wrap single value in array
                    let arr_val = if value.is_array() {
                        value.to_string()
                    } else {
                        format!("[{}]", value)
                    };
                    params.push(json!(arr_val));
                    param_idx += 1;
                }
            }
        }

        // --- Handle $push ---
        if let Some(push) = obj.get("$push").and_then(|v| v.as_object()) {
            for (key, value) in push {
                let snake = to_snake(key);
                if self.known_columns.contains(&snake) {
                    sets.push(format!(
                        "\"{}\" = COALESCE(\"{}\", '[]'{}) || ${}{}",
                        snake, snake, jc, param_idx, jc
                    ));
                    let arr_val = if value.is_array() {
                        value.to_string()
                    } else {
                        format!("[{}]", value)
                    };
                    params.push(json!(arr_val));
                    param_idx += 1;
                }
            }
        }

        // --- Handle $pull (remove from JSONB array) ---
        if let Some(pull) = obj.get("$pull").and_then(|v| v.as_object()) {
            for (key, value) in pull {
                let snake = to_snake(key);
                if self.known_columns.contains(&snake) {
                    // Remove matching elements from JSONB array
                    // PostgreSQL: col = (SELECT jsonb_agg(elem) FROM jsonb_array_elements(col) elem WHERE elem != value::jsonb)
                    sets.push(format!(
                        "\"{}\" = COALESCE((SELECT jsonb_agg(elem) FROM jsonb_array_elements(\"{}\") elem WHERE elem != ${}{}), '[]'{})",
                        snake, snake, param_idx, jc, jc
                    ));
                    params.push(json!(value.to_string()));
                    param_idx += 1;
                }
            }
        }

        // --- Handle regular fields + dot-notation + overflow ---
        let mut data_updates: Map<String, Value> = Map::new();

        for (key, value) in obj {
            if key.starts_with('$') || key == "id" { continue; }

            // Dot-notation: "configField.nested" → jsonb_set on the column
            if key.contains('.') {
                let parts: Vec<&str> = key.splitn(2, '.').collect();
                let base = parts[0];
                let nested = parts[1];
                let snake_base = to_snake(base);

                if self.known_columns.contains(&snake_base) {
                    // jsonb_set(col, '{nested}', value::jsonb, true)
                    sets.push(format!(
                        "\"{}\" = jsonb_set(COALESCE(\"{}\", '{{}}'{jc}), '{{{}}}', ${}{jc}, true)",
                        snake_base, snake_base, nested, param_idx, jc = jc
                    ));
                    params.push(json!(value.to_string()));
                    param_idx += 1;
                } else {
                    // Unknown base → update _data overflow with dot path
                    data_updates.insert(key.clone(), value.clone());
                }
                continue;
            }

            let snake = to_snake(key);
            if self.known_columns.contains(&snake) {
                if value.is_object() && !value.is_array() {
                    // JSONB object: MERGE (shallow) instead of replace
                    sets.push(format!(
                        "\"{}\" = COALESCE(\"{}\", '{{}}'{}) || ${}{}",
                        snake, snake, jc, param_idx, jc
                    ));
                    params.push(json!(value.to_string()));
                } else if value.is_array() {
                    // Array: replace entirely
                    sets.push(format!("\"{}\" = ${}{}", snake, param_idx, jc));
                    params.push(json!(value.to_string()));
                } else if self.jsonb_columns.contains(&snake) && value.is_string() {
                    // JSONB column with string value — cast to jsonb
                    sets.push(format!("\"{}\" = ${}{}", snake, param_idx, jc));
                    params.push(json!(format!("\"{}\"", value.as_str().unwrap())));
                } else if self.timestamp_columns.contains(&snake) || is_likely_timestamp_column(&snake) {
                    // Timestamp column — handle epoch ms (number) and ISO strings
                    if let Some(ms) = value.as_i64() {
                        let secs = ms / 1000;
                        let nanos = ((ms % 1000) * 1_000_000) as u32;
                        if let Some(dt) = chrono::DateTime::from_timestamp(secs, nanos) {
                            sets.push(format!("\"{}\" = ${}::timestamptz", snake, param_idx));
                            params.push(json!(dt.to_rfc3339()));
                        } else {
                            sets.push(format!("\"{}\" = ${}::timestamptz", snake, param_idx));
                            params.push(value.clone());
                        }
                    } else if value.is_string() {
                        sets.push(format!("\"{}\" = ${}::timestamptz", snake, param_idx));
                        params.push(value.clone());
                    } else {
                        sets.push(format!("\"{}\" = ${}", snake, param_idx));
                        params.push(value.clone());
                    }
                } else {
                    sets.push(format!("\"{}\" = ${}", snake, param_idx));
                    params.push(value.clone());
                }
                param_idx += 1;
            } else {
                // Unknown field → goes to _data overflow
                data_updates.insert(key.clone(), value.clone());
            }
        }

        // Merge overflow updates into _data
        if !data_updates.is_empty() {
            sets.push(format!(
                "\"_data\" = COALESCE(\"_data\", '{{}}'{jc}) || ${}{jc}",
                param_idx, jc = jc
            ));
            params.push(json!(Value::Object(data_updates).to_string()));
            param_idx += 1;
        }

        // Always update updated_at
        sets.push(format!("\"updated_at\" = {}", self.dialect.now_expr()));

        if sets.is_empty() {
            // Legacy behavior: return null instead of 404 for non-existent items
            return Ok(self.get(id).await?.unwrap_or(Value::Null));
        }

        params.push(json!(id));
        let sql = format!(
            "WITH updated AS (UPDATE \"{}\" SET {} WHERE id = ${} RETURNING *) \
             SELECT row_to_json(updated.*) as doc FROM updated",
            self.table_name,
            sets.join(", "),
            param_idx
        );

        let rows = self.execute_query(&sql, &params).await?;
        // Legacy behavior: return null instead of 404 for non-existent items
        Ok(rows.first()
            .map(|r| self.row_to_value(r))
            .unwrap_or(Value::Null))
    }

    // ── REMOVE ───────────────────────────────────────────────────────

    /// Delete a record by ID.
    pub async fn remove(&self, id: &str) -> Result<Value, ApiError> {
        let sql = format!(
            "WITH deleted AS (DELETE FROM \"{}\" WHERE id = $1 RETURNING *) \
             SELECT row_to_json(deleted.*) as doc FROM deleted",
            self.table_name
        );
        let rows = self.execute_query(&sql, &[json!(id)]).await?;
        // Legacy behavior: return null instead of 404 for non-existent items
        Ok(rows.first()
            .map(|r| self.row_to_value(r))
            .unwrap_or(Value::Null))
    }

    /// Delete records matching a query (bulk delete).
    pub async fn remove_by_query(&self, query_value: Value) -> Result<Vec<Value>, ApiError> {
        let query_map = query_value.as_object().cloned().unwrap_or_default();
        let condition = translate_query(&query_map, &self.query_opts());
        let (where_sql, where_params) = self.condition_to_sql(&condition);

        if where_sql.is_empty() {
            return Err(ApiError::BadRequest("Query required for bulk delete".into()));
        }

        let sql = format!(
            "WITH deleted AS (DELETE FROM \"{}\" WHERE {} RETURNING *) \
             SELECT row_to_json(deleted.*) as doc FROM deleted",
            self.table_name, where_sql
        );
        let rows = self.execute_query(&sql, &where_params).await?;
        Ok(self.rows_to_values(&rows))
    }

    // ── Helpers ──────────────────────────────────────────────────────

    /// Convert a sea_query Condition into SQL WHERE clause string + params.
    /// Uses the appropriate backend (Postgres/Sqlite) for SQL generation.
    fn condition_to_sql(&self, condition: &Condition) -> (String, Vec<Value>) {
        use sea_orm::sea_query::{PostgresQueryBuilder, SqliteQueryBuilder};

        // Build a dummy SELECT to extract the WHERE clause
        let mut select = SeaQuery::select();
        select
            .column(Alias::new("id"))
            .from(Alias::new(&self.table_name))
            .cond_where(condition.clone());

        let (sql, values) = match self.dialect {
            Dialect::Postgres => select.build(PostgresQueryBuilder),
            Dialect::Sqlite => select.build(SqliteQueryBuilder),
        };

        // Extract WHERE clause from the full SQL
        // The full SQL looks like: SELECT "id" FROM "table" WHERE ...
        let where_start = sql.find(" WHERE ");
        if let Some(idx) = where_start {
            let where_sql = &sql[idx + 7..]; // Skip " WHERE "
            let params: Vec<Value> = values.0.iter().map(sea_value_to_json).collect();
            (where_sql.to_string(), params)
        } else {
            (String::new(), Vec::new())
        }
    }

    async fn execute_query(
        &self,
        sql: &str,
        params: &[Value],
    ) -> Result<Vec<sea_orm::QueryResult>, ApiError> {
        let sea_params: Vec<sea_orm::Value> = params.iter().map(json_to_sea_value).collect();
        self.db
            .query_all(sea_orm::Statement::from_sql_and_values(
                self.dialect.sea_orm_backend(),
                sql,
                sea_params,
            ))
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))
    }

    fn row_to_value(&self, row: &sea_orm::QueryResult) -> Value {
        use sea_orm::TryGetable;
        let doc: Value = row.try_get("", "doc").unwrap_or(Value::Null);
        from_row(doc, self.id_alias.as_deref())
    }

    fn rows_to_values(&self, rows: &[sea_orm::QueryResult]) -> Vec<Value> {
        rows.iter().map(|r| self.row_to_value(r)).collect()
    }
}

/// Convert a sea_query::Value back to serde_json::Value.
fn sea_value_to_json(v: &sea_query::Value) -> Value {
    match v {
        sea_query::Value::String(Some(s)) => json!(s.as_ref()),
        sea_query::Value::Int(Some(i)) => json!(i),
        sea_query::Value::BigInt(Some(i)) => json!(i),
        sea_query::Value::Float(Some(f)) => json!(f),
        sea_query::Value::Double(Some(f)) => json!(f),
        sea_query::Value::Bool(Some(b)) => json!(b),
        _ => Value::Null,
    }
}

/// Check if a column name likely represents a timestamp.
fn is_likely_timestamp_column(snake_col: &str) -> bool {
    snake_col.ends_with("_at")
        || snake_col == "date_of_birth"
        || snake_col.contains("date")
        || snake_col.ends_with("_since")
        || snake_col.ends_with("_exp")
        || snake_col.starts_with("effective_")
        || snake_col == "start" || snake_col == "end"
        || snake_col == "due_date"
        || snake_col.ends_with("_from") || snake_col.ends_with("_to")
        || snake_col.ends_with("_start") || snake_col.ends_with("_end")
}

/// Convert a serde_json::Value to a sea_orm::Value for parameterized queries.
fn json_to_sea_value(v: &Value) -> sea_orm::Value {
    match v {
        Value::Null => sea_orm::Value::String(None),
        Value::Bool(b) => (*b).into(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() { i.into() }
            else if let Some(f) = n.as_f64() { f.into() }
            else { n.to_string().into() }
        }
        Value::String(s) => s.clone().into(),
        Value::Array(_) | Value::Object(_) => v.to_string().into(),
    }
}
