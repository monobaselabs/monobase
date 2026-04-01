use sea_orm::{ConnectionTrait, DatabaseConnection, FromQueryResult, JsonValue, Statement};

use crate::error::ApiError;

use super::{ListFilesQuery, UploadFileBody};

/// Create a new stored_file record with status='uploading'.
pub async fn create_stored_file(
    db: &DatabaseConnection,
    created_by: &str,
    body: &UploadFileBody,
) -> Result<serde_json::Value, ApiError> {
    let id = uuid::Uuid::new_v4().to_string();

    let sql = r#"
        INSERT INTO "stored_file" (
            id, filename, mime_type, size, status, owner_id,
            created_by, updated_by, created_at, updated_at, uploaded_at, version
        )
        VALUES ($1, $2, $3, $4, 'uploading', $5, $6, $6, NOW(), NOW(), NOW(), 1)
        RETURNING *
    "#;

    let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![
            id.into(),
            body.filename.clone().into(),
            body.mime_type.clone().into(),
            (body.size as i64).into(),
            body.owner_id.clone().into(),
            created_by.into(),
        ],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::internal("Failed to create stored file"))?;

    Ok(result)
}

/// Get a stored_file by ID.
pub async fn get_stored_file(
    db: &DatabaseConnection,
    id: &str,
) -> Result<Option<serde_json::Value>, ApiError> {
    let sql = r#"SELECT * FROM "stored_file" WHERE id = $1"#;

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

/// List stored_files with pagination and optional filters.
pub async fn list_stored_files(
    db: &DatabaseConnection,
    offset: u64,
    limit: u64,
    query: &ListFilesQuery,
) -> Result<(Vec<serde_json::Value>, u64), ApiError> {
    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<sea_orm::Value> = Vec::new();
    let mut idx = 1u32;

    if let Some(ref owner_id) = query.owner_id {
        conditions.push(format!("owner_id = ${}", idx));
        params.push(owner_id.clone().into());
        idx += 1;
    }

    if let Some(ref status) = query.status {
        conditions.push(format!("status = ${}", idx));
        params.push(status.clone().into());
        idx += 1;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Count
    let count_sql = format!(
        r#"SELECT COUNT(*) as count FROM "stored_file" {}"#,
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
        r#"SELECT * FROM "stored_file" {} ORDER BY uploaded_at DESC LIMIT ${} OFFSET ${}"#,
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

/// Transition a stored_file from 'uploading' to 'available'.
pub async fn complete_stored_file(
    db: &DatabaseConnection,
    id: &str,
    updated_by: &str,
) -> Result<serde_json::Value, ApiError> {
    let sql = r#"
        UPDATE "stored_file"
        SET status = 'available', updated_at = NOW(), updated_by = $2, version = version + 1
        WHERE id = $1
        RETURNING *
    "#;

    let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![id.into(), updated_by.into()],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::not_found("Stored file not found"))?;

    Ok(result)
}

/// Hard-delete a stored_file by ID.
pub async fn delete_stored_file(
    db: &DatabaseConnection,
    id: &str,
) -> Result<(), ApiError> {
    let sql = r#"DELETE FROM "stored_file" WHERE id = $1"#;

    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![id.into()],
    ))
    .await
    .map_err(ApiError::Database)?;

    Ok(())
}
