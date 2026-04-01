use sea_orm::{DatabaseConnection, FromQueryResult, JsonValue, Statement};

use crate::error::ApiError;

use super::{
    CancelEmailQueueItemBody, CreateEmailTemplateBody, ListEmailQueueQuery, ListEmailTemplatesQuery,
    UpdateEmailTemplateBody,
};

// ---------------------------------------------------------------------------
// Email template queries
// ---------------------------------------------------------------------------

/// Create a new email template.
pub async fn create_email_template(
    db: &DatabaseConnection,
    created_by: &str,
    body: &CreateEmailTemplateBody,
) -> Result<serde_json::Value, ApiError> {
    let tags = body
        .tags
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string());

    let variables = body.variables.to_string();

    let sql = r#"
        INSERT INTO "email_template" (
            id, name, description, subject,
            body_html, body_text, tags, variables,
            from_name, from_email, reply_to_email, reply_to_name,
            status, created_by, updated_by, created_at, updated_at, version
        )
        VALUES (
            gen_random_uuid(), $1, $2, $3,
            $4, $5, $6::jsonb, $7::jsonb,
            $8, $9, $10, $11,
            'draft', $12, $12, NOW(), NOW(), 1
        )
        RETURNING *
    "#;

    let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![
            body.name.clone().into(),
            body.description.clone().unwrap_or_default().into(),
            body.subject.clone().into(),
            body.body_html.clone().into(),
            body.body_text.clone().unwrap_or_default().into(),
            tags.into(),
            variables.into(),
            body.from_name.clone().unwrap_or_default().into(),
            body.from_email.clone().unwrap_or_default().into(),
            body.reply_to_email.clone().unwrap_or_default().into(),
            body.reply_to_name.clone().unwrap_or_default().into(),
            created_by.into(),
        ],
    ))
    .one(db)
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("unique") || msg.contains("duplicate") {
            ApiError::conflict("An email template with this name already exists")
        } else {
            ApiError::Database(e)
        }
    })?
    .ok_or_else(|| ApiError::internal("Failed to create email template"))?;

    Ok(result)
}

/// Get an email template by ID.
pub async fn get_email_template(
    db: &DatabaseConnection,
    id: &str,
) -> Result<Option<serde_json::Value>, ApiError> {
    let sql = r#"SELECT * FROM "email_template" WHERE id = $1"#;

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

/// List email templates with pagination and optional filters.
pub async fn list_email_templates(
    db: &DatabaseConnection,
    offset: u64,
    limit: u64,
    query: &ListEmailTemplatesQuery,
) -> Result<(Vec<serde_json::Value>, u64), ApiError> {
    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<sea_orm::Value> = Vec::new();
    let mut idx = 1u32;

    if let Some(ref status) = query.status {
        conditions.push(format!("status = ${}", idx));
        params.push(status.clone().into());
        idx += 1;
    }

    if let Some(ref q) = query.q {
        let pattern = format!("%{}%", q);
        conditions.push(format!("(name ILIKE ${} OR description ILIKE ${})", idx, idx));
        params.push(pattern.into());
        idx += 1;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Count
    let count_sql = format!(
        r#"SELECT COUNT(*) as count FROM "email_template" {}"#,
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
        r#"SELECT * FROM "email_template" {} ORDER BY created_at DESC LIMIT ${} OFFSET ${}"#,
        where_clause,
        idx,
        idx + 1,
    );

    params.push((limit as i64).into());
    params.push((offset as i64).into());

    let data = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        &data_sql,
        params,
    ))
    .all(db)
    .await
    .map_err(ApiError::Database)?;

    Ok((data, total_count))
}

/// Update an email template.
pub async fn update_email_template(
    db: &DatabaseConnection,
    id: &str,
    updated_by: &str,
    body: &UpdateEmailTemplateBody,
) -> Result<serde_json::Value, ApiError> {
    let mut set_clauses = vec!["updated_at = NOW()".to_string()];
    let mut params: Vec<sea_orm::Value> = vec![];
    let mut idx = 1u32;

    macro_rules! maybe_set_text {
        ($field:ident, $col:expr) => {
            if let Some(ref val) = body.$field {
                set_clauses.push(format!("{} = ${}", $col, idx));
                params.push(val.clone().into());
                idx += 1;
            }
        };
    }

    macro_rules! maybe_set_jsonb {
        ($field:ident, $col:expr) => {
            if let Some(ref val) = body.$field {
                set_clauses.push(format!("{} = ${}::jsonb", $col, idx));
                params.push(val.to_string().into());
                idx += 1;
            }
        };
    }

    maybe_set_text!(name, "name");
    maybe_set_text!(description, "description");
    maybe_set_text!(subject, "subject");
    maybe_set_text!(body_html, "body_html");
    maybe_set_text!(body_text, "body_text");
    maybe_set_text!(from_name, "from_name");
    maybe_set_text!(from_email, "from_email");
    maybe_set_text!(reply_to_email, "reply_to_email");
    maybe_set_text!(reply_to_name, "reply_to_name");
    maybe_set_text!(status, "status");
    maybe_set_jsonb!(tags, "tags");
    maybe_set_jsonb!(variables, "variables");

    set_clauses.push(format!("updated_by = ${}", idx));
    params.push(updated_by.into());
    idx += 1;

    set_clauses.push("version = version + 1".to_string());

    params.push(id.into());

    let sql = format!(
        r#"UPDATE "email_template" SET {} WHERE id = ${} RETURNING *"#,
        set_clauses.join(", "),
        idx,
    );

    let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        &sql,
        params,
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::not_found("Email template not found"))?;

    Ok(result)
}

// ---------------------------------------------------------------------------
// Email queue queries
// ---------------------------------------------------------------------------

/// Get an email queue item by ID.
pub async fn get_email_queue_item(
    db: &DatabaseConnection,
    id: &str,
) -> Result<Option<serde_json::Value>, ApiError> {
    let sql = r#"SELECT * FROM "email_queue" WHERE id = $1"#;

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

/// List email queue items with pagination and optional filters.
pub async fn list_email_queue_items(
    db: &DatabaseConnection,
    offset: u64,
    limit: u64,
    query: &ListEmailQueueQuery,
) -> Result<(Vec<serde_json::Value>, u64), ApiError> {
    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<sea_orm::Value> = Vec::new();
    let mut idx = 1u32;

    if let Some(ref status) = query.status {
        conditions.push(format!("status = ${}", idx));
        params.push(status.clone().into());
        idx += 1;
    }

    if let Some(ref template_id) = query.template_id {
        conditions.push(format!("template_id = ${}", idx));
        params.push(template_id.clone().into());
        idx += 1;
    }

    if let Some(ref recipient_email) = query.recipient_email {
        let pattern = format!("%{}%", recipient_email);
        conditions.push(format!("recipient_email ILIKE ${}", idx));
        params.push(pattern.into());
        idx += 1;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Count
    let count_sql = format!(
        r#"SELECT COUNT(*) as count FROM "email_queue" {}"#,
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

    // Data — ordered by priority desc (lower number = higher priority), then created_at
    let data_sql = format!(
        r#"SELECT * FROM "email_queue" {} ORDER BY priority ASC, created_at ASC LIMIT ${} OFFSET ${}"#,
        where_clause,
        idx,
        idx + 1,
    );

    params.push((limit as i64).into());
    params.push((offset as i64).into());

    let data = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        &data_sql,
        params,
    ))
    .all(db)
    .await
    .map_err(ApiError::Database)?;

    Ok((data, total_count))
}

/// Cancel a pending email queue item.
pub async fn cancel_email_queue_item(
    db: &DatabaseConnection,
    id: &str,
    cancelled_by: &str,
    body: &CancelEmailQueueItemBody,
) -> Result<serde_json::Value, ApiError> {
    let sql = r#"
        UPDATE "email_queue"
        SET
            status = 'cancelled',
            cancelled_at = NOW(),
            cancelled_by = $1,
            cancellation_reason = $2,
            updated_by = $1,
            updated_at = NOW(),
            version = version + 1
        WHERE id = $3
        RETURNING *
    "#;

    let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![
            cancelled_by.into(),
            body.reason.clone().unwrap_or_default().into(),
            id.into(),
        ],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::not_found("Email queue item not found"))?;

    Ok(result)
}

/// Retry a failed email queue item by resetting it to pending.
pub async fn retry_email_queue_item(
    db: &DatabaseConnection,
    id: &str,
    updated_by: &str,
) -> Result<serde_json::Value, ApiError> {
    let sql = r#"
        UPDATE "email_queue"
        SET
            status = 'pending',
            last_error = NULL,
            next_retry_at = NULL,
            updated_by = $1,
            updated_at = NOW(),
            version = version + 1
        WHERE id = $2
        RETURNING *
    "#;

    let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![updated_by.into(), id.into()],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::not_found("Email queue item not found"))?;

    Ok(result)
}
