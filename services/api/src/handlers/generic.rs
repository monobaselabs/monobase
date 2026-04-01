//! Generic CRUD handlers that accept a service name in the path.
//! Provides dynamic, table-agnostic CRUD routing via information_schema discovery.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::ApiError;
use crate::model::{ModelConfig, PostgresModel};
use crate::service::{CrudService, ServiceContext};
use super::AppState;

/// Get enum validation rules for a table.
/// Override this in your application to add domain-specific enum constraints.
fn get_enum_validations(_table: &str) -> Vec<(&'static str, Vec<&'static str>)> {
    vec![]
}

/// Get required fields for a table on create.
/// Override this in your application to add domain-specific required field constraints.
pub fn get_required_fields(_table: &str) -> Vec<&'static str> {
    vec![]
}

/// Get or create a CrudService for a table on-the-fly.
pub fn get_service(state: &AppState, table: &str) -> CrudService {
    let table_name = table.replace('-', "_");
    let known_columns = get_table_columns_cached(state, &table_name);

    let model = PostgresModel::new(
        state.db.clone(),
        state.dialect,
        ModelConfig {
            table_name: table_name.clone(),
            known_columns,
            id_alias: None,
            json_array_columns: vec![],
            scalar_array_columns: vec![],
            defaults: get_table_defaults(&table_name),
            column_name_map: vec![],
            jsonb_columns: get_jsonb_columns(&table_name),
            timestamp_columns: get_timestamp_columns(&table_name),
        },
    );
    CrudService::new(model)
}

/// Cache of table -> columns, populated lazily from information_schema.
static TABLE_COLUMNS: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<String, Vec<String>>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

/// Cache of table -> JSONB column names (for proper query handling).
static JSONB_COLUMNS: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<String, std::collections::HashSet<String>>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

/// Cache of table -> timestamp column names.
static TIMESTAMP_COLUMNS: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<String, std::collections::HashSet<String>>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

/// Get known columns for a table from the discovery cache.
fn get_table_columns_cached(_state: &AppState, table: &str) -> Vec<String> {
    if let Some(cols) = TABLE_COLUMNS.lock().unwrap().get(table) {
        return cols.clone();
    }
    get_table_columns(table)
}

/// Populate column cache from database (call at startup).
pub async fn discover_table_columns(db: &sea_orm::DatabaseConnection, dialect: crate::model::dialect::Dialect) {
    use sea_orm::{ConnectionTrait, TryGetable};

    let sql = "SELECT table_name, column_name, data_type FROM information_schema.columns WHERE table_schema = 'public' ORDER BY table_name, ordinal_position";
    if let Ok(rows) = db.query_all(
        sea_orm::Statement::from_sql_and_values(dialect.sea_orm_backend(), sql, vec![])
    ).await {
        let mut col_cache = TABLE_COLUMNS.lock().unwrap();
        let mut jsonb_cache = JSONB_COLUMNS.lock().unwrap();
        let mut ts_cache = TIMESTAMP_COLUMNS.lock().unwrap();
        for row in &rows {
            let table: String = row.try_get("", "table_name").unwrap_or_default();
            let col: String = row.try_get("", "column_name").unwrap_or_default();
            let dtype: String = row.try_get("", "data_type").unwrap_or_default();
            col_cache.entry(table.clone()).or_insert_with(Vec::new).push(col.clone());
            if dtype == "jsonb" || dtype == "json" {
                jsonb_cache.entry(table.clone()).or_insert_with(std::collections::HashSet::new).insert(col.clone());
            }
            if dtype.contains("timestamp") {
                ts_cache.entry(table).or_insert_with(std::collections::HashSet::new).insert(col);
            }
        }
        tracing::info!("Discovered columns for {} tables", col_cache.len());
    }
}

/// Static fallback: minimal columns when information_schema hasn't been queried yet.
fn get_table_columns(table: &str) -> Vec<String> {
    if let Some(cols) = TABLE_COLUMNS.lock().unwrap().get(table) {
        return cols.clone();
    }
    vec![
        "id".to_string(),
        "created_at".to_string(),
        "updated_at".to_string(),
        "_data".to_string(),
    ]
}

/// Get default values for a table on create.
/// Override this in your application to add domain-specific defaults.
fn get_table_defaults(_table: &str) -> Vec<(String, Value)> {
    vec![]
}

/// Get JSONB columns for a table from the discovery cache.
pub fn get_jsonb_columns(table: &str) -> Vec<String> {
    JSONB_COLUMNS.lock().unwrap()
        .get(table)
        .map(|s| s.iter().cloned().collect())
        .unwrap_or_default()
}

/// Get timestamp columns for a table from discovery cache.
pub fn get_timestamp_columns(table: &str) -> Vec<String> {
    TIMESTAMP_COLUMNS.lock().unwrap()
        .get(table)
        .map(|s| s.iter().cloned().collect())
        .unwrap_or_default()
}

/// Check if a table requires authentication for all operations.
/// Override this in your application to add auth requirements.
pub fn table_requires_auth(_table: &str) -> bool {
    false
}

/// Enforce auth if required for this table.
pub fn enforce_auth(table: &str, user: &Option<crate::auth::backend::AuthUser>) -> Result<(), ApiError> {
    if table_requires_auth(table) && user.is_none() {
        return Err(ApiError::Unauthorized);
    }
    Ok(())
}

pub fn build_ctx(raw_query: &str, user: Option<crate::auth::backend::AuthUser>) -> ServiceContext {
    let query = parse_qs_to_json(raw_query);
    ServiceContext {
        user,
        query,
        params: HashMap::new(),
        provider: "rest".to_string(),
    }
}

/// Parse qs-style query string into nested JSON.
/// Handles: key=value, key[nested]=value, key[$op]=value, key[$op][0]=value (arrays)
pub fn parse_qs_to_json(raw: &str) -> Value {
    let mut result = serde_json::Map::new();

    for pair in raw.split('&') {
        if pair.is_empty() { continue; }
        let (key_part, value) = match pair.split_once('=') {
            Some((k, v)) => (
                urlencoding::decode(k).unwrap_or_default().to_string(),
                urlencoding::decode(v).unwrap_or_default().to_string(),
            ),
            None => continue,
        };

        let parsed_value = parse_value(&value);
        let segments = parse_qs_key(&key_part);

        if segments.len() == 1 {
            result.insert(segments[0].clone(), parsed_value);
        } else {
            set_nested(&mut result, &segments, parsed_value);
        }
    }

    convert_numeric_objects_to_arrays(&mut result);
    Value::Object(result)
}

fn parse_qs_key(key: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut in_bracket = false;

    for ch in key.chars() {
        if ch == '[' {
            if !current.is_empty() {
                segments.push(current.clone());
                current.clear();
            }
            in_bracket = true;
        } else if ch == ']' {
            if in_bracket {
                segments.push(current.clone());
                current.clear();
                in_bracket = false;
            }
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        segments.push(current);
    }
    if segments.is_empty() {
        segments.push(key.to_string());
    }
    segments
}

fn set_nested(map: &mut serde_json::Map<String, Value>, segments: &[String], value: Value) {
    if segments.is_empty() { return; }
    if segments.len() == 1 {
        map.insert(segments[0].clone(), value);
        return;
    }

    let key = &segments[0];
    if !map.contains_key(key) {
        map.insert(key.clone(), Value::Object(serde_json::Map::new()));
    }
    if let Some(obj) = map.get_mut(key).and_then(|v| v.as_object_mut()) {
        set_nested(obj, &segments[1..], value);
    }
}

fn convert_numeric_objects_to_arrays(map: &mut serde_json::Map<String, Value>) {
    let keys: Vec<String> = map.keys().cloned().collect();
    for key in keys {
        if let Some(val) = map.get_mut(&key) {
            if let Some(obj) = val.as_object_mut() {
                convert_numeric_objects_to_arrays(obj);
                let all_numeric = !obj.is_empty() && obj.keys().all(|k| k.parse::<usize>().is_ok());
                if all_numeric {
                    let mut entries: Vec<(usize, Value)> = obj
                        .iter()
                        .filter_map(|(k, v)| k.parse::<usize>().ok().map(|i| (i, v.clone())))
                        .collect();
                    entries.sort_by_key(|(i, _)| *i);
                    let arr: Vec<Value> = entries.into_iter().map(|(_, v)| v).collect();
                    map.insert(key, Value::Array(arr));
                }
            }
        }
    }
}

fn parse_value(s: &str) -> Value {
    if s == "true" { return Value::Bool(true); }
    if s == "false" { return Value::Bool(false); }
    if s == "null" { return Value::Null; }
    if let Ok(n) = s.parse::<i64>() { return json!(n); }
    if let Ok(n) = s.parse::<f64>() { return json!(n); }
    if (s.starts_with('{') && s.ends_with('}')) || (s.starts_with('[') && s.ends_with(']')) {
        if let Ok(v) = serde_json::from_str::<Value>(s) {
            return v;
        }
    }
    json!(s)
}

// === CRUD Handlers ===

pub async fn generic_list(
    State(state): State<Arc<AppState>>,
    Path(table): Path<String>,
    headers: HeaderMap,
    raw_query: axum::extract::RawQuery,
) -> Result<Json<Value>, ApiError> {
    let user = crate::middleware::auth::extract_auth(&state, &headers).await;
    let table_name = table.replace('-', "_");
    enforce_auth(&table_name, &user)?;
    let svc = get_service(&state, &table);
    let ctx = build_ctx(raw_query.0.as_deref().unwrap_or(""), user);
    let result = svc.find(&ctx).await?;
    Ok(Json(result))
}

pub async fn generic_get(
    State(state): State<Arc<AppState>>,
    Path((table, id)): Path<(String, String)>,
    headers: HeaderMap,
    raw_query: axum::extract::RawQuery,
) -> Result<Json<Value>, ApiError> {
    let user = crate::middleware::auth::extract_auth(&state, &headers).await;
    let table_name = table.replace('-', "_");
    enforce_auth(&table_name, &user)?;
    let svc = get_service(&state, &table);
    let ctx = build_ctx(raw_query.0.as_deref().unwrap_or(""), user);
    let result = svc.get(&id, &ctx).await?;
    Ok(Json(result))
}

/// Check if a table name exists in the discovery cache.
fn is_known_table(name: &str) -> bool {
    TABLE_COLUMNS.lock().unwrap().contains_key(name)
}

/// Smart GET handler for 2-segment paths: GET /{table}/{id_or_subtable}
/// If "{table}_{id_or_subtable}" is a known table, treat as nested list.
/// Otherwise, treat as get-by-id on "{table}".
pub async fn generic_smart_get(
    State(state): State<Arc<AppState>>,
    Path((table, id_or_sub)): Path<(String, String)>,
    headers: HeaderMap,
    raw_query: axum::extract::RawQuery,
) -> Result<Json<Value>, ApiError> {
    let nested_table = format!("{}_{}", table, id_or_sub).replace('-', "_");
    if is_known_table(&nested_table) {
        // Nested list: e.g., GET /billing/invoices -> list billing_invoices
        let user = crate::middleware::auth::extract_auth(&state, &headers).await;
        let svc = get_service(&state, &nested_table);
        let ctx = build_ctx(raw_query.0.as_deref().unwrap_or(""), user);
        let result = svc.find(&ctx).await?;
        Ok(Json(result))
    } else {
        // Get by ID: e.g., GET /organizations/abc123
        let user = crate::middleware::auth::extract_auth(&state, &headers).await;
        let table_name = table.replace('-', "_");
        enforce_auth(&table_name, &user)?;
        let svc = get_service(&state, &table);
        let ctx = build_ctx(raw_query.0.as_deref().unwrap_or(""), user);
        let result = svc.get(&id_or_sub, &ctx).await?;
        Ok(Json(result))
    }
}

pub async fn generic_create(
    State(state): State<Arc<AppState>>,
    Path(table): Path<String>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let table_name = table.replace('-', "_");

    let required = get_required_fields(&table_name);
    if !required.is_empty() {
        if let Some(obj) = body.as_object() {
            for field in &required {
                if !obj.contains_key(*field) || obj.get(*field).map_or(true, |v| v.is_null()) {
                    return Err(ApiError::BadRequest(format!("Validation failed: {} is required", field)));
                }
            }
        }
    }

    let enums = get_enum_validations(&table_name);
    if !enums.is_empty() {
        if let Some(obj) = body.as_object() {
            for (field, valid_values) in &enums {
                if let Some(val) = obj.get(*field).and_then(|v| v.as_str()) {
                    if !valid_values.contains(&val) {
                        return Err(ApiError::BadRequest(format!(
                            "Validation failed: invalid {}", field
                        )));
                    }
                }
            }
        }
    }

    let user = crate::middleware::auth::extract_auth(&state, &headers).await;
    enforce_auth(&table_name, &user)?;
    let svc = get_service(&state, &table);
    let ctx = build_ctx("", user);
    let result = svc.create(body, &ctx).await?;
    Ok((StatusCode::CREATED, Json(result)))
}

pub async fn generic_patch(
    State(state): State<Arc<AppState>>,
    Path((table, id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let user = crate::middleware::auth::extract_auth(&state, &headers).await;
    let table_name = table.replace('-', "_");
    enforce_auth(&table_name, &user)?;
    let svc = get_service(&state, &table);
    let ctx = build_ctx("", user);
    let result = svc.patch(&id, body, &ctx).await?;
    Ok(Json(result))
}

pub async fn generic_delete(
    State(state): State<Arc<AppState>>,
    Path((table, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let user = crate::middleware::auth::extract_auth(&state, &headers).await;
    let table_name = table.replace('-', "_");
    enforce_auth(&table_name, &user)?;
    let svc = get_service(&state, &table);
    let ctx = build_ctx("", user);
    let result = svc.remove(&id, &ctx).await?;
    Ok(Json(result))
}

// === Nested path handlers (e.g., /bir/readings -> table "bir_readings") ===

pub async fn generic_nested_list(
    State(state): State<Arc<AppState>>,
    Path((prefix, subtable)): Path<(String, String)>,
    headers: HeaderMap,
    raw_query: axum::extract::RawQuery,
) -> Result<Json<Value>, ApiError> {
    let table = format!("{}_{}", prefix, subtable).replace('-', "_");
    let user = crate::middleware::auth::extract_auth(&state, &headers).await;
    let svc = get_service(&state, &table);
    let ctx = build_ctx(raw_query.0.as_deref().unwrap_or(""), user);
    let result = svc.find(&ctx).await?;
    Ok(Json(result))
}

pub async fn generic_nested_get(
    State(state): State<Arc<AppState>>,
    Path((prefix, subtable, id)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let table = format!("{}_{}", prefix, subtable).replace('-', "_");
    let user = crate::middleware::auth::extract_auth(&state, &headers).await;
    let svc = get_service(&state, &table);
    let ctx = build_ctx("", user);
    let result = svc.get(&id, &ctx).await?;
    Ok(Json(result))
}

pub async fn generic_nested_create(
    State(state): State<Arc<AppState>>,
    Path((prefix, subtable)): Path<(String, String)>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let table = format!("{}_{}", prefix, subtable).replace('-', "_");
    let user = crate::middleware::auth::extract_auth(&state, &headers).await;
    let svc = get_service(&state, &table);
    let ctx = build_ctx("", user);
    let result = svc.create(body, &ctx).await?;
    Ok((StatusCode::CREATED, Json(result)))
}

pub async fn generic_nested_patch(
    State(state): State<Arc<AppState>>,
    Path((prefix, subtable, id)): Path<(String, String, String)>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let table = format!("{}_{}", prefix, subtable).replace('-', "_");
    let user = crate::middleware::auth::extract_auth(&state, &headers).await;
    let svc = get_service(&state, &table);
    let ctx = build_ctx("", user);
    let result = svc.patch(&id, body, &ctx).await?;
    Ok(Json(result))
}

pub async fn generic_nested_delete(
    State(state): State<Arc<AppState>>,
    Path((prefix, subtable, id)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let table = format!("{}_{}", prefix, subtable).replace('-', "_");
    let user = crate::middleware::auth::extract_auth(&state, &headers).await;
    let svc = get_service(&state, &table);
    let ctx = build_ctx("", user);
    let result = svc.remove(&id, &ctx).await?;
    Ok(Json(result))
}

pub async fn generic_bulk_delete(
    State(state): State<Arc<AppState>>,
    Path(table): Path<String>,
    raw_query: axum::extract::RawQuery,
) -> Result<Json<Value>, ApiError> {
    let query_str = raw_query.0.unwrap_or_default();
    let query = parse_qs_to_json(&query_str);
    let svc = get_service(&state, &table);
    let deleted = svc.remove_by_query(query).await?;
    Ok(Json(serde_json::json!({
        "data": deleted,
        "total": deleted.len(),
    })))
}
