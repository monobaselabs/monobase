use sea_orm::{DatabaseConnection, FromQueryResult, JsonValue, Statement};

use crate::error::ApiError;

use super::ListAuditLogsQuery;

/// List audit log entries with pagination and optional filters.
pub async fn list_audit_logs(
    db: &DatabaseConnection,
    offset: u64,
    limit: u64,
    query: &ListAuditLogsQuery,
) -> Result<(Vec<serde_json::Value>, u64), ApiError> {
    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<sea_orm::Value> = Vec::new();
    let mut idx = 1u32;

    if let Some(ref event_type) = query.event_type {
        conditions.push(format!("event_type = ${}", idx));
        params.push(event_type.clone().into());
        idx += 1;
    }

    if let Some(ref category) = query.category {
        conditions.push(format!("category = ${}", idx));
        params.push(category.clone().into());
        idx += 1;
    }

    if let Some(ref action) = query.action {
        conditions.push(format!("action = ${}", idx));
        params.push(action.clone().into());
        idx += 1;
    }

    if let Some(ref outcome) = query.outcome {
        conditions.push(format!("outcome = ${}", idx));
        params.push(outcome.clone().into());
        idx += 1;
    }

    if let Some(ref user_id) = query.user_id {
        conditions.push(format!("user_id = ${}", idx));
        params.push(user_id.clone().into());
        idx += 1;
    }

    if let Some(ref resource_type) = query.resource_type {
        conditions.push(format!("resource_type = ${}", idx));
        params.push(resource_type.clone().into());
        idx += 1;
    }

    if let Some(ref date_from) = query.date_from {
        conditions.push(format!("created_at >= ${}::timestamptz", idx));
        params.push(date_from.clone().into());
        idx += 1;
    }

    if let Some(ref date_to) = query.date_to {
        conditions.push(format!("created_at <= ${}::timestamptz", idx));
        params.push(date_to.clone().into());
        idx += 1;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Count query
    let count_sql = format!(
        r#"SELECT COUNT(*) as count FROM "audit_log_entry" {}"#,
        where_clause
    );
    let count_result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        &count_sql,
        params.clone(),
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?;

    let total_count = count_result
        .and_then(|v| v.get("count").and_then(|c| c.as_u64()))
        .unwrap_or(0);

    // Data query
    let data_sql = format!(
        r#"SELECT * FROM "audit_log_entry" {} ORDER BY created_at DESC LIMIT ${} OFFSET ${}"#,
        where_clause, idx, idx + 1,
    );

    let mut data_params = params;
    data_params.push((limit as i64).into());
    data_params.push((offset as i64).into());

    let data = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        &data_sql,
        data_params,
    ))
    .all(db)
    .await
    .map_err(ApiError::Database)?;

    Ok((data, total_count))
}
