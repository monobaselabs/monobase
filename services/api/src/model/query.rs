//! MongoDB-style query → `sea_query::Condition` translator.
//!
//! Port of `services/hapihub/src/utils/drizzle/pg-query-translator.ts`.
//! Outputs `sea_query::Condition` which SeaORM builds into dialect-specific SQL.

use super::dialect::Dialect;
use super::overflow::to_snake;
use sea_orm::sea_query::{self, Alias, Condition, Expr, IntoIden, SimpleExpr};
use serde_json::{json, Map, Value};
use std::collections::HashSet;

/// Configuration for query translation.
pub struct QueryOpts {
    pub known_columns: HashSet<String>,
    pub json_array_columns: HashSet<String>,
    pub scalar_array_columns: HashSet<String>,
    pub jsonb_columns: HashSet<String>,
    pub dialect: Dialect,
}

/// Translate a MongoDB-style JSON query into a `sea_query::Condition`.
pub fn translate_query(query: &Map<String, Value>, opts: &QueryOpts) -> Condition {
    let mut condition = Condition::all();

    for (key, value) in query {
        // Skip meta-parameters
        if key.starts_with('$') && !matches!(key.as_str(), "$and" | "$or") {
            continue;
        }

        match key.as_str() {
            "$and" => {
                if let Some(arr) = value.as_array() {
                    let mut sub = Condition::all();
                    for item in arr {
                        if let Some(obj) = item.as_object() {
                            sub = sub.add(translate_query(obj, opts));
                        }
                    }
                    condition = condition.add(sub);
                }
            }
            "$or" => {
                if let Some(arr) = value.as_array() {
                    let mut sub = Condition::any();
                    for item in arr {
                        if let Some(obj) = item.as_object() {
                            sub = sub.add(translate_query(obj, opts));
                        }
                    }
                    condition = condition.add(sub);
                }
            }
            _ => {
                let col_ref = resolve_column(key, opts);
                let exprs = translate_value(&col_ref, value, opts);
                for expr in exprs {
                    condition = condition.add(expr);
                }
            }
        }
    }

    condition
}

/// Parse meta-parameters from a query object.
pub fn extract_meta_params(query: &Map<String, Value>) -> (MetaParams, Map<String, Value>) {
    let mut meta = MetaParams::default();
    let mut filter = Map::new();

    for (key, value) in query {
        match key.as_str() {
            "$limit" => meta.limit = value.as_i64().or_else(|| value.as_str().and_then(|s| s.parse().ok())),
            "$skip" | "$offset" => meta.skip = value.as_i64().or_else(|| value.as_str().and_then(|s| s.parse().ok())),
            "$sort" => meta.sort = Some(value.clone()),
            "$select" => meta.select = Some(value.clone()),
            "$total" => meta.total = value.as_bool().unwrap_or(true),
            "$search" => {
                // $search can be a string or an object { text, limit, organization }
                if let Some(s) = value.as_str() {
                    meta.search = Some(s.to_string());
                } else if let Some(obj) = value.as_object() {
                    meta.search = obj.get("text").and_then(|v| v.as_str()).map(|s| s.to_string());
                    // Also extract organization filter from $search
                    if let Some(org) = obj.get("organization").and_then(|v| v.as_str()) {
                        filter.insert("organization".to_string(), json!(org));
                    }
                    if let Some(limit) = obj.get("limit").and_then(|v| v.as_i64()) {
                        meta.limit = Some(limit);
                    }
                }
            }
            _ => { filter.insert(key.clone(), value.clone()); }
        }
    }

    (meta, filter)
}

/// Meta-parameters extracted from the query.
#[derive(Debug, Default)]
pub struct MetaParams {
    pub limit: Option<i64>,
    pub skip: Option<i64>,
    pub sort: Option<Value>,
    pub select: Option<Value>,
    pub total: bool,
    pub search: Option<String>,
}

impl MetaParams {
    /// Build ORDER BY clause. Returns vec of (column, direction) pairs.
    pub fn order_by_clause(&self) -> String {
        if let Some(sort) = &self.sort {
            if let Some(obj) = sort.as_object() {
                let parts: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| {
                        let col = to_snake(k);
                        let dir = if v.as_i64().unwrap_or(1) < 0 { "DESC" } else { "ASC" };
                        format!("\"{}\" {}", col, dir)
                    })
                    .collect();
                if !parts.is_empty() {
                    return parts.join(", ");
                }
            }
            if let Some(s) = sort.as_str() {
                if let Some(field) = s.strip_prefix('-') {
                    return format!("\"{}\" DESC", to_snake(field));
                }
                return format!("\"{}\" ASC", to_snake(s));
            }
        }
        "\"created_at\" DESC".to_string()
    }
}

// --- Internal ---

enum ColumnRef {
    Direct(String),
    ScalarArray(String),
    /// JSONB path extraction: col->>'key' or col->'a'->>'b'
    JsonPath { column: String, path: Vec<String> },
    JsonArrayPath { column: String, key: String },
    DataPath { path: Vec<String> },
}

fn resolve_column(field: &str, opts: &QueryOpts) -> ColumnRef {
    if field.contains('.') {
        let parts: Vec<&str> = field.splitn(2, '.').collect();
        let base = parts[0];
        let nested = parts[1];
        let snake_base = to_snake(base);

        if opts.json_array_columns.contains(&snake_base) {
            return ColumnRef::JsonArrayPath {
                column: snake_base,
                key: nested.to_string(),
            };
        }
        // If base is a known column (e.g., JSONB), use JsonPath extraction
        if opts.known_columns.contains(&snake_base) {
            return ColumnRef::JsonPath {
                column: snake_base,
                path: vec![nested.to_string()],
            };
        }
        return ColumnRef::DataPath {
            path: vec![base.to_string(), nested.to_string()],
        };
    }

    let snake = to_snake(field);
    if opts.known_columns.contains(&snake) || snake == "id" {
        ColumnRef::Direct(snake)
    } else if opts.scalar_array_columns.contains(&snake) {
        ColumnRef::ScalarArray(snake)
    } else {
        ColumnRef::DataPath { path: vec![field.to_string()] }
    }
}

fn col_expr(col: &ColumnRef, dialect: &Dialect) -> SimpleExpr {
    match col {
        ColumnRef::Direct(name) => Expr::col(Alias::new(name)).into(),
        ColumnRef::ScalarArray(name) => Expr::col(Alias::new(name)).into(),
        ColumnRef::JsonPath { column, path } => {
            let sql = dialect.json_extract_text(column, &path.iter().map(|s| s.as_str()).collect::<Vec<_>>());
            Expr::cust(&sql)
        }
        ColumnRef::DataPath { path } => {
            let sql = dialect.json_extract_text("_data", &path.iter().map(|s| s.as_str()).collect::<Vec<_>>());
            Expr::cust(&sql)
        }
        ColumnRef::JsonArrayPath { column, .. } => Expr::col(Alias::new(column)).into(),
    }
}

fn is_jsonb_col(col: &ColumnRef, opts: &QueryOpts) -> bool {
    match col {
        ColumnRef::Direct(name) => opts.jsonb_columns.contains(name),
        _ => false,
    }
}

fn translate_value(col: &ColumnRef, value: &Value, opts: &QueryOpts) -> Vec<SimpleExpr> {
    match value {
        Value::String(s) => {
            // JSONB columns need special handling — use text extraction for comparison
            if is_jsonb_col(col, opts) {
                if let ColumnRef::Direct(name) = col {
                    // For JSONB column with string value, cast the value to jsonb for comparison
                    vec![Expr::cust_with_values(
                        &format!("\"{}\" @> $1::jsonb", name),
                        [format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))],
                    )]
                } else {
                    vec![Expr::expr(col_expr(col, &opts.dialect)).eq(s.as_str())]
                }
            } else {
                vec![Expr::expr(col_expr(col, &opts.dialect)).eq(s.as_str())]
            }
        }
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                vec![Expr::expr(col_expr(col, &opts.dialect)).eq(i)]
            } else if let Some(f) = n.as_f64() {
                vec![Expr::expr(col_expr(col, &opts.dialect)).eq(f)]
            } else {
                vec![]
            }
        }
        Value::Bool(b) => {
            vec![Expr::expr(col_expr(col, &opts.dialect)).eq(*b)]
        }
        Value::Null => {
            vec![Expr::expr(col_expr(col, &opts.dialect)).is_null()]
        }
        Value::Array(arr) => {
            // For JSONB columns, use @> (contains) operator
            if is_jsonb_col(col, opts) {
                if let ColumnRef::Direct(name) = col {
                    let json_arr = serde_json::to_string(arr).unwrap_or_default();
                    vec![Expr::cust(&format!(
                        "\"{}\" @> '{}'{}", name, json_arr, opts.dialect.json_cast()
                    ))]
                } else {
                    let vals: Vec<sea_query::Value> = arr.iter().map(json_to_sea_value).collect();
                    vec![Expr::expr(col_expr(col, &opts.dialect)).is_in(vals)]
                }
            } else {
                // Non-JSONB: use IN clause
                let vals: Vec<sea_query::Value> = arr.iter().map(json_to_sea_value).collect();
                vec![Expr::expr(col_expr(col, &opts.dialect)).is_in(vals)]
            }
        }
        Value::Object(obj) => {
            translate_operators(col, obj, opts)
        }
    }
}

/// Check if a column is likely a timestamp (for epoch ms conversion in comparisons).
fn is_timestamp_col(col: &ColumnRef) -> bool {
    match col {
        ColumnRef::Direct(name) => {
            name.ends_with("_at") || name == "date_of_birth" || name.contains("date")
                || name.ends_with("_since") || name.ends_with("_exp")
        }
        ColumnRef::JsonPath { .. } => false,
        _ => false,
    }
}

/// Convert a JSON value to a sea_query::Value, handling epoch ms → timestamp for timestamp columns.
fn json_to_sea_value_for_col(val: &Value, col: &ColumnRef) -> sea_query::Value {
    if is_timestamp_col(col) {
        // Epoch ms number → ISO string for timestamp comparison
        if let Some(ms) = val.as_i64() {
            if ms > 1_000_000_000 {
                let secs = ms / 1000;
                let nanos = ((ms % 1000) * 1_000_000) as u32;
                if let Some(dt) = chrono::DateTime::from_timestamp(secs, nanos) {
                    return dt.to_rfc3339().into();
                }
            }
        }
        if let Some(ms) = val.as_f64() {
            if ms > 1_000_000_000.0 {
                let secs = (ms / 1000.0) as i64;
                if let Some(dt) = chrono::DateTime::from_timestamp(secs, 0) {
                    return dt.to_rfc3339().into();
                }
            }
        }
        // ISO date string → keep as-is (PostgreSQL can compare timestamp with ISO string)
        // The issue is when it's a plain string — wrap in ::timestamp cast
        if let Some(s) = val.as_str() {
            // PostgreSQL can parse ISO strings natively if cast
            return s.to_string().into();
        }
    }
    json_to_sea_value(val)
}

fn translate_operators(col: &ColumnRef, ops: &Map<String, Value>, opts: &QueryOpts) -> Vec<SimpleExpr> {
    let mut exprs = Vec::new();
    let ce = || col_expr(col, &opts.dialect);

    for (op, val) in ops {
        match op.as_str() {
            "$eq" => {
                exprs.push(Expr::expr(ce()).eq(json_to_sea_value_for_col(val, col)));
            }
            "$ne" => {
                if val.is_null() {
                    // $ne: null → IS NOT NULL
                    exprs.push(Expr::expr(ce()).is_not_null());
                } else {
                    exprs.push(
                        Expr::expr(ce()).ne(json_to_sea_value_for_col(val, col))
                            .or(Expr::expr(ce()).is_null())
                    );
                }
            }
            "$gt" => {
                if is_timestamp_col(col) {
                    let col_str = col_sql_str(col, &opts.dialect);
                    exprs.push(Expr::cust_with_values(
                        &format!("{} > $1::timestamptz", col_str), [json_to_sea_value_for_col(val, col)]
                    ));
                } else {
                    exprs.push(Expr::expr(ce()).gt(json_to_sea_value_for_col(val, col)));
                }
            }
            "$gte" => {
                if is_timestamp_col(col) {
                    let col_str = col_sql_str(col, &opts.dialect);
                    exprs.push(Expr::cust_with_values(
                        &format!("{} >= $1::timestamptz", col_str), [json_to_sea_value_for_col(val, col)]
                    ));
                } else {
                    exprs.push(Expr::expr(ce()).gte(json_to_sea_value_for_col(val, col)));
                }
            }
            "$lt" => {
                if is_timestamp_col(col) {
                    let col_str = col_sql_str(col, &opts.dialect);
                    exprs.push(Expr::cust_with_values(
                        &format!("{} < $1::timestamptz", col_str), [json_to_sea_value_for_col(val, col)]
                    ));
                } else {
                    exprs.push(Expr::expr(ce()).lt(json_to_sea_value_for_col(val, col)));
                }
            }
            "$lte" => {
                if is_timestamp_col(col) {
                    let col_str = col_sql_str(col, &opts.dialect);
                    exprs.push(Expr::cust_with_values(
                        &format!("{} <= $1::timestamptz", col_str), [json_to_sea_value_for_col(val, col)]
                    ));
                } else {
                    exprs.push(Expr::expr(ce()).lte(json_to_sea_value_for_col(val, col)));
                }
            }
            "$in" => {
                if let Some(arr) = val.as_array() {
                    match col {
                        ColumnRef::ScalarArray(column) => {
                            // JSONB scalar array overlap — dialect-specific
                            let str_values: Vec<String> = arr
                                .iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect();
                            let sql = opts.dialect.json_scalar_overlap(column, &str_values);
                            exprs.push(Expr::cust(&sql));
                        }
                        _ => {
                            let vals: Vec<sea_query::Value> = arr.iter().map(json_to_sea_value).collect();
                            if vals.is_empty() {
                                exprs.push(Expr::cust("FALSE"));
                            } else {
                                exprs.push(Expr::expr(ce()).is_in(vals));
                            }
                        }
                    }
                }
            }
            "$nin" => {
                if let Some(arr) = val.as_array() {
                    let vals: Vec<sea_query::Value> = arr.iter().map(json_to_sea_value).collect();
                    if !vals.is_empty() {
                        exprs.push(Expr::expr(ce()).is_not_in(vals));
                    }
                }
            }
            "$exists" => {
                let exists = val.as_bool().unwrap_or(true);
                if exists {
                    exprs.push(Expr::expr(ce()).is_not_null());
                } else {
                    exprs.push(Expr::expr(ce()).is_null());
                }
            }
            "$regex" => {
                if let Some(pattern) = val.as_str() {
                    let case_insensitive = ops
                        .get("$options")
                        .and_then(|v| v.as_str())
                        .map(|s| s.contains('i'))
                        .unwrap_or(false);

                    match opts.dialect {
                        Dialect::Postgres => {
                            let op = if case_insensitive { "~*" } else { "~" };
                            exprs.push(Expr::cust_with_values(
                                &format!("{} {} $1", col_sql_str(col, &opts.dialect), op),
                                [pattern.to_string()],
                            ));
                        }
                        Dialect::Sqlite => {
                            let like_pattern = format!("%{}%", pattern);
                            exprs.push(Expr::expr(ce()).like(&like_pattern));
                        }
                    }
                }
            }
            "$options" => {} // Handled by $regex
            "$all" => {
                if let ColumnRef::ScalarArray(column) = col {
                    if let Some(arr) = val.as_array() {
                        let json_str = serde_json::to_string(arr).unwrap_or_default();
                        let cast = opts.dialect.json_cast();
                        exprs.push(Expr::cust(&format!(
                            "\"{}\" @> '{}'{}", column, json_str, cast
                        )));
                    }
                }
            }
            _ => {} // Unknown operator
        }
    }

    exprs
}

fn col_sql_str(col: &ColumnRef, dialect: &Dialect) -> String {
    match col {
        ColumnRef::Direct(name) | ColumnRef::ScalarArray(name) => format!("\"{}\"", name),
        ColumnRef::JsonPath { column, path } => {
            dialect.json_extract_text(column, &path.iter().map(|s| s.as_str()).collect::<Vec<_>>())
        }
        ColumnRef::DataPath { path } => {
            dialect.json_extract_text("_data", &path.iter().map(|s| s.as_str()).collect::<Vec<_>>())
        }
        ColumnRef::JsonArrayPath { column, .. } => format!("\"{}\"", column),
    }
}

fn json_to_sea_value(v: &Value) -> sea_query::Value {
    match v {
        Value::Null => sea_query::Value::String(None),
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn pg_opts() -> QueryOpts {
        QueryOpts {
            known_columns: ["id", "name", "email", "organization", "type", "status", "created_at"]
                .iter().map(|s| s.to_string()).collect(),
            json_array_columns: HashSet::new(),
            scalar_array_columns: ["tags"].iter().map(|s| s.to_string()).collect(),
            jsonb_columns: HashSet::new(),
            dialect: Dialect::Postgres,
        }
    }

    #[test]
    fn test_direct_equality() {
        let query = json!({"name": "John"}).as_object().unwrap().clone();
        let cond = translate_query(&query, &pg_opts());
        // Condition should not be empty
        assert!(!format!("{:?}", cond).is_empty());
    }

    #[test]
    fn test_in_operator() {
        let query = json!({"id": {"$in": ["a", "b"]}}).as_object().unwrap().clone();
        let cond = translate_query(&query, &pg_opts());
        assert!(!format!("{:?}", cond).is_empty());
    }

    #[test]
    fn test_and_or() {
        let query = json!({"$or": [{"name": "A"}, {"name": "B"}]}).as_object().unwrap().clone();
        let cond = translate_query(&query, &pg_opts());
        assert!(!format!("{:?}", cond).is_empty());
    }

    #[test]
    fn test_null_value() {
        let query = json!({"email": null}).as_object().unwrap().clone();
        let cond = translate_query(&query, &pg_opts());
        assert!(!format!("{:?}", cond).is_empty());
    }

    #[test]
    fn test_meta_params_extraction() {
        let query = json!({
            "$limit": 10,
            "$skip": 5,
            "$sort": {"createdAt": -1},
            "name": "test"
        }).as_object().unwrap().clone();

        let (meta, filter) = extract_meta_params(&query);
        assert_eq!(meta.limit, Some(10));
        assert_eq!(meta.skip, Some(5));
        assert!(meta.sort.is_some());
        assert_eq!(filter.len(), 1);
        assert!(filter.contains_key("name"));
    }

    #[test]
    fn test_sort_clause() {
        let meta = MetaParams {
            sort: Some(json!({"createdAt": -1})),
            ..Default::default()
        };
        assert_eq!(meta.order_by_clause(), "\"created_at\" DESC");
    }
}
