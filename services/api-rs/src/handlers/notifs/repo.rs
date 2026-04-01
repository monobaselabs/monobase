use sea_orm::{ConnectionTrait, DatabaseConnection, FromQueryResult, JsonValue, Statement};

use crate::error::ApiError;

use super::{CreateNotificationBody, ListNotificationsQuery};

/// Create a new notification record.
pub async fn create_notification(
    db: &DatabaseConnection,
    created_by: &str,
    body: &CreateNotificationBody,
) -> Result<serde_json::Value, ApiError> {
    let id = uuid::Uuid::new_v4().to_string();

    let sql = r#"
        INSERT INTO "notification" (
            id, recipient_id, type, channel, title, message,
            scheduled_at, related_entity_type, related_entity_id,
            status, consent_validated,
            created_by, updated_by, created_at, updated_at, version
        )
        VALUES (
            $1, $2, $3, $4, $5, $6,
            $7, $8, $9,
            'queued', $10,
            $11, $11, NOW(), NOW(), 1
        )
        RETURNING *
    "#;

    let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![
            id.into(),
            body.recipient_id.clone().into(),
            body.r#type.clone().into(),
            body.channel.clone().into(),
            body.title.clone().into(),
            body.message.clone().into(),
            body.scheduled_at.clone().unwrap_or_default().into(),
            body.related_entity_type.clone().unwrap_or_default().into(),
            body.related_entity_id.clone().unwrap_or_default().into(),
            body.consent_validated.into(),
            created_by.into(),
        ],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::internal("Failed to create notification"))?;

    Ok(result)
}

/// Get a single notification by ID.
pub async fn get_notification(
    db: &DatabaseConnection,
    id: &str,
) -> Result<Option<serde_json::Value>, ApiError> {
    let sql = r#"SELECT * FROM "notification" WHERE id = $1"#;

    let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![id.into()],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?;

    Ok(result)
}

/// List notifications with pagination and optional filters.
pub async fn list_notifications(
    db: &DatabaseConnection,
    offset: u64,
    limit: u64,
    query: &ListNotificationsQuery,
    // When Some, restrict results to this recipient (non-admin path).
    recipient_scope: Option<&str>,
) -> Result<(Vec<serde_json::Value>, u64), ApiError> {
    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<sea_orm::Value> = Vec::new();
    let mut idx = 1u32;

    // Scope to a specific recipient when the caller is not an admin.
    if let Some(recipient_id) = recipient_scope {
        conditions.push(format!("recipient_id = ${}", idx));
        params.push(recipient_id.into());
        idx += 1;
    }

    if let Some(ref t) = query.r#type {
        conditions.push(format!("type = ${}", idx));
        params.push(t.clone().into());
        idx += 1;
    }

    if let Some(ref ch) = query.channel {
        conditions.push(format!("channel = ${}", idx));
        params.push(ch.clone().into());
        idx += 1;
    }

    if let Some(ref st) = query.status {
        conditions.push(format!("status = ${}", idx));
        params.push(st.clone().into());
        idx += 1;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Count
    let count_sql = format!(
        r#"SELECT COUNT(*) as count FROM "notification" {}"#,
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

    // Data
    let data_sql = format!(
        r#"SELECT * FROM "notification" {} ORDER BY created_at DESC LIMIT ${} OFFSET ${}"#,
        where_clause,
        idx,
        idx + 1,
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

/// Mark a single notification as read (status='read', read_at=NOW()).
/// Scoped to a specific recipient to prevent cross-user writes.
pub async fn mark_notification_as_read(
    db: &DatabaseConnection,
    id: &str,
    recipient_id: &str,
    updated_by: &str,
) -> Result<serde_json::Value, ApiError> {
    let sql = r#"
        UPDATE "notification"
        SET status = 'read', read_at = NOW(), updated_at = NOW(),
            updated_by = $3, version = version + 1
        WHERE id = $1 AND recipient_id = $2
        RETURNING *
    "#;

    let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![id.into(), recipient_id.into(), updated_by.into()],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::not_found("Notification not found or access denied"))?;

    Ok(result)
}

/// Mark all unread notifications for a recipient as read.
/// Returns the count of updated rows.
pub async fn mark_all_notifications_as_read(
    db: &DatabaseConnection,
    recipient_id: &str,
    updated_by: &str,
) -> Result<u64, ApiError> {
    let sql = r#"
        UPDATE "notification"
        SET status = 'read', read_at = NOW(), updated_at = NOW(),
            updated_by = $2, version = version + 1
        WHERE recipient_id = $1
          AND status NOT IN ('read', 'failed', 'expired')
    "#;

    let result = db
        .execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            sql,
            vec![recipient_id.into(), updated_by.into()],
        ))
        .await
        .map_err(ApiError::Database)?;

    Ok(result.rows_affected())
}
