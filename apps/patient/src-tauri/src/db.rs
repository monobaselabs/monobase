use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryResult {
    pub changes: u64,
    pub last_insert_id: i64,
}

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(db_path: &str) -> Result<Self, String> {
        let conn = Connection::open(db_path)
            .map_err(|e| format!("Failed to open database: {}", e))?;

        // Enable WAL mode for better concurrency
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| format!("Failed to set WAL mode: {}", e))?;

        // Create better-auth tables
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS \"user\" (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                email TEXT NOT NULL UNIQUE,
                email_verified INTEGER NOT NULL DEFAULT 0,
                image TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                role TEXT,
                banned INTEGER DEFAULT 0,
                ban_reason TEXT,
                ban_expires INTEGER
            );
            CREATE TABLE IF NOT EXISTS session (
                id TEXT PRIMARY KEY,
                expires_at INTEGER NOT NULL,
                token TEXT NOT NULL UNIQUE,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                ip_address TEXT,
                user_agent TEXT,
                user_id TEXT NOT NULL REFERENCES \"user\"(id) ON DELETE CASCADE,
                active_organization_id TEXT
            );
            CREATE TABLE IF NOT EXISTS account (
                id TEXT PRIMARY KEY,
                account_id TEXT NOT NULL,
                provider_id TEXT NOT NULL,
                user_id TEXT NOT NULL REFERENCES \"user\"(id) ON DELETE CASCADE,
                access_token TEXT,
                refresh_token TEXT,
                id_token TEXT,
                access_token_expires_at INTEGER,
                refresh_token_expires_at INTEGER,
                scope TEXT,
                password TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS verification (
                id TEXT PRIMARY KEY,
                identifier TEXT NOT NULL,
                value TEXT NOT NULL,
                expires_at INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );"
        ).map_err(|e| format!("Failed to create auth schema: {}", e))?;

        log::info!("Database initialized at: {}", db_path);
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }

    pub fn execute(&self, sql: &str, params: Vec<serde_json::Value>) -> Result<QueryResult, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let rusqlite_params: Vec<Box<dyn rusqlite::ToSql>> = params.iter().map(json_to_sql).collect();
        let refs: Vec<&dyn rusqlite::ToSql> = rusqlite_params.iter().map(|b| b.as_ref()).collect();
        let changes = conn.execute(sql, refs.as_slice())
            .map_err(|e| format!("SQL execute error: {}", e))?;
        Ok(QueryResult {
            changes: changes as u64,
            last_insert_id: conn.last_insert_rowid(),
        })
    }

    /// Execute a SELECT (or INSERT...RETURNING) and return rows as arrays
    /// with values in SQL column order. This is critical for Drizzle's
    /// sqlite-proxy which maps by position, not by name.
    pub fn select(&self, sql: &str, params: Vec<serde_json::Value>) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(sql).map_err(|e| format!("SQL prepare error: {}", e))?;
        let col_count = stmt.column_count();
        let rusqlite_params: Vec<Box<dyn rusqlite::ToSql>> = params.iter().map(json_to_sql).collect();
        let refs: Vec<&dyn rusqlite::ToSql> = rusqlite_params.iter().map(|b| b.as_ref()).collect();
        let rows = stmt.query_map(refs.as_slice(), |row| {
            let mut arr = Vec::with_capacity(col_count);
            for i in 0..col_count {
                arr.push(row_value_to_json(row, i));
            }
            Ok(serde_json::Value::Array(arr))
        }).map_err(|e| format!("SQL query error: {}", e))?
          .collect::<rusqlite::Result<Vec<_>>>()
          .map_err(|e| format!("Row error: {}", e))?;
        Ok(rows)
    }
}

fn json_to_sql(value: &serde_json::Value) -> Box<dyn rusqlite::ToSql> {
    match value {
        serde_json::Value::Null => Box::new(Option::<String>::None),
        serde_json::Value::Bool(b) => Box::new(*b as i64),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() { Box::new(i) }
            else if let Some(f) = n.as_f64() { Box::new(f) }
            else { Box::new(n.to_string()) }
        }
        serde_json::Value::String(s) => Box::new(s.clone()),
        _ => Box::new(value.to_string()),
    }
}

fn row_value_to_json(row: &rusqlite::Row, idx: usize) -> serde_json::Value {
    if let Ok(v) = row.get::<_, i64>(idx) { return serde_json::Value::Number(v.into()); }
    if let Ok(v) = row.get::<_, f64>(idx) { return serde_json::json!(v); }
    if let Ok(v) = row.get::<_, String>(idx) {
        if (v.starts_with('{') && v.ends_with('}')) || (v.starts_with('[') && v.ends_with(']')) {
            if let Ok(json_val) = serde_json::from_str(&v) { return json_val; }
        }
        return serde_json::Value::String(v);
    }
    if let Ok(v) = row.get::<_, Option<String>>(idx) {
        return v.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null);
    }
    serde_json::Value::Null
}
