//! Service layer -- business logic.
//!
//! `CrudService` provides default CRUD by delegating to `PostgresModel`.
//! Domain services can extend with hooks by wrapping CrudService.

use crate::auth::backend::AuthUser;
use crate::error::ApiError;
use crate::model::PostgresModel;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

/// Context passed to every service method.
/// Built from the Axum request by handlers.
pub struct ServiceContext {
    pub user: Option<AuthUser>,
    pub query: Value,
    pub params: HashMap<String, String>,
    pub provider: String, // "rest" or "ipc"
}

impl ServiceContext {
    pub fn user_id(&self) -> Option<&str> {
        self.user.as_ref().map(|u| u.id.as_str())
    }
}

/// Default CRUD service -- delegates directly to PostgresModel.
#[derive(Clone)]
pub struct CrudService {
    pub model: Arc<PostgresModel>,
}

impl CrudService {
    pub fn new(model: PostgresModel) -> Self {
        Self {
            model: Arc::new(model),
        }
    }

    pub async fn find(&self, ctx: &ServiceContext) -> Result<Value, ApiError> {
        let result = self.model.find(ctx.query.clone()).await?;
        let mut data = result.data;

        // Apply $expand, expand, or $populate if present
        let expand = ctx.query.get("$expand")
            .or_else(|| ctx.query.get("expand"))
            .or_else(|| ctx.query.get("$populate"));
        if let Some(expand) = expand {
            let expand_fields = parse_expand_param(expand);
            for item in &mut data {
                apply_expand(item, &expand_fields, &self.model.db, self.model.dialect).await;
            }
        }

        // Strip sensitive fields
        for item in &mut data {
            strip_sensitive_fields(item, &self.model.table_name);
        }

        Ok(json!({
            "data": data,
            "total": result.total,
            "limit": result.limit,
            "skip": result.skip,
        }))
    }

    pub async fn get(&self, id: &str, ctx: &ServiceContext) -> Result<Value, ApiError> {
        let result = self.model.get(id).await?;

        match result {
            Some(mut item) => {
                let expand = ctx.query.get("$expand")
                    .or_else(|| ctx.query.get("expand"))
                    .or_else(|| ctx.query.get("$populate"));
                if let Some(expand) = expand {
                    let expand_fields = parse_expand_param(expand);
                    apply_expand(&mut item, &expand_fields, &self.model.db, self.model.dialect).await;
                }
                strip_sensitive_fields(&mut item, &self.model.table_name);
                Ok(item)
            }
            None => Ok(Value::Null),
        }
    }

    pub async fn create(&self, mut data: Value, ctx: &ServiceContext) -> Result<Value, ApiError> {
        // Inject createdBy from auth context
        if let (Some(uid), Some(obj)) = (ctx.user_id(), data.as_object_mut()) {
            obj.insert("createdBy".to_string(), json!(uid));
        }

        // Validate required fields
        let required = crate::handlers::generic::get_required_fields(&self.model.table_name);
        if !required.is_empty() {
            if let Some(obj) = data.as_object() {
                for field in &required {
                    if !obj.contains_key(*field) || obj.get(*field).map_or(true, |v| v.is_null()) {
                        return Err(ApiError::BadRequest(format!("Validation failed: {} is required", field)));
                    }
                }
            }
        }

        self.model.create(data).await
    }

    pub async fn patch(&self, id: &str, data: Value, _ctx: &ServiceContext) -> Result<Value, ApiError> {
        self.model.patch(id, data).await
    }

    pub async fn remove(&self, id: &str, _ctx: &ServiceContext) -> Result<Value, ApiError> {
        self.model.remove(id).await
    }

    pub async fn remove_by_query(&self, query: Value) -> Result<Vec<Value>, ApiError> {
        self.model.remove_by_query(query).await
    }
}

/// Parse $expand parameter into list of field names.
fn parse_expand_param(expand: &Value) -> Vec<String> {
    match expand {
        Value::String(s) => s.split(',').map(|f| f.trim().to_string()).collect(),
        Value::Array(arr) => arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect(),
        Value::Object(obj) => obj.keys().cloned().collect(),
        _ => vec![],
    }
}

/// Map a field name to the table it references.
/// Override this for your application's foreign key relationships.
fn expand_field_to_table(_field: &str) -> Option<&'static str> {
    None
}

/// Apply $expand to a single record by resolving foreign key references.
async fn apply_expand(
    record: &mut Value,
    fields: &[String],
    db: &sea_orm::DatabaseConnection,
    dialect: crate::model::dialect::Dialect,
) {
    use sea_orm::{ConnectionTrait, TryGetable};

    for field in fields {
        let (base_field, nested_field) = if field.contains('.') {
            let parts: Vec<&str> = field.splitn(2, '.').collect();
            (parts[0].to_string(), Some(parts[1].to_string()))
        } else {
            (field.clone(), None)
        };

        let table = match expand_field_to_table(&base_field) {
            Some(t) => t,
            None => continue,
        };

        let id = match record.get(&base_field).and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => continue,
        };

        let sql = format!(
            "SELECT row_to_json(t.*) as doc FROM \"{}\" t WHERE t.id = $1 LIMIT 1",
            table
        );
        if let Ok(rows) = db.query_all(
            sea_orm::Statement::from_sql_and_values(dialect.sea_orm_backend(), &sql, vec![id.clone().into()])
        ).await {
            if let Some(row) = rows.first() {
                let doc: Value = row.try_get("", "doc").unwrap_or(Value::Null);
                if !doc.is_null() {
                    let mut expanded = crate::model::overflow::from_row(doc, None);
                    if let Some(ref nested) = nested_field {
                        let nested_fields = vec![nested.clone()];
                        Box::pin(apply_expand(&mut expanded, &nested_fields, db, dialect)).await;
                    }
                    if let Some(obj) = record.as_object_mut() {
                        obj.insert(base_field.clone(), expanded);
                    }
                } else if let Some(obj) = record.as_object_mut() {
                    obj.insert(base_field.clone(), json!({"id": id}));
                }
            } else if let Some(obj) = record.as_object_mut() {
                obj.insert(base_field.clone(), json!({"id": id}));
            }
        }
    }
}

/// Strip sensitive fields from response based on table name.
/// Add your application's sensitive field rules here.
fn strip_sensitive_fields(record: &mut Value, table_name: &str) {
    if let Some(obj) = record.as_object_mut() {
        // Always strip passwords from auth-related tables
        if table_name == "accounts" || table_name == "user" || table_name == "account" {
            obj.remove("password");
            obj.remove("mfa");
            obj.remove("credentials");
        }
    }
}
