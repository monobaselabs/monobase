use sea_orm::{ConnectionTrait, DatabaseConnection, FromQueryResult, JsonValue, Statement};

use crate::error::ApiError;

use super::{CreateReviewBody, ListReviewsQuery};

/// Create a new review record.
pub async fn create_review(
    db: &DatabaseConnection,
    created_by: &str,
    body: &CreateReviewBody,
) -> Result<serde_json::Value, ApiError> {
    let id = uuid::Uuid::new_v4().to_string();

    let sql = r#"
        INSERT INTO "review" (
            id, context_id, reviewer_id, review_type,
            reviewed_entity_id, nps_score, comment,
            created_by, updated_by, created_at, updated_at, version
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8, NOW(), NOW(), 1)
        RETURNING *
    "#;

    let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![
            id.into(),
            body.context_id.clone().into(),
            body.reviewer_id.clone().into(),
            body.review_type.clone().into(),
            body.reviewed_entity_id.clone().into(),
            (body.nps_score as i32).into(),
            body.comment.clone().into(),
            created_by.into(),
        ],
    ))
    .one(db)
    .await
    .map_err(|e| {
        // Detect unique constraint violation on (context_id, reviewer_id, review_type)
        let msg = e.to_string();
        if msg.contains("unique") || msg.contains("duplicate") || msg.contains("23505") {
            ApiError::conflict("A review already exists for this context, reviewer and type")
        } else {
            ApiError::Database(e)
        }
    })?
    .ok_or_else(|| ApiError::internal("Failed to create review"))?;

    Ok(result)
}

/// Get a single review by ID.
pub async fn get_review(
    db: &DatabaseConnection,
    id: &str,
) -> Result<Option<serde_json::Value>, ApiError> {
    let sql = r#"SELECT * FROM "review" WHERE id = $1"#;

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

/// List reviews with pagination and optional filters.
pub async fn list_reviews(
    db: &DatabaseConnection,
    offset: u64,
    limit: u64,
    query: &ListReviewsQuery,
) -> Result<(Vec<serde_json::Value>, u64), ApiError> {
    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<sea_orm::Value> = Vec::new();
    let mut idx = 1u32;

    if let Some(ref reviewer_id) = query.reviewer_id {
        conditions.push(format!("reviewer_id = ${}", idx));
        params.push(reviewer_id.clone().into());
        idx += 1;
    }

    if let Some(ref review_type) = query.review_type {
        conditions.push(format!("review_type = ${}", idx));
        params.push(review_type.clone().into());
        idx += 1;
    }

    if let Some(ref context_id) = query.context_id {
        conditions.push(format!("context_id = ${}", idx));
        params.push(context_id.clone().into());
        idx += 1;
    }

    if let Some(ref reviewed_entity_id) = query.reviewed_entity_id {
        conditions.push(format!("reviewed_entity_id = ${}", idx));
        params.push(reviewed_entity_id.clone().into());
        idx += 1;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Count query
    let count_sql = format!(
        r#"SELECT COUNT(*) as count FROM "review" {}"#,
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
        r#"SELECT * FROM "review" {} ORDER BY created_at DESC LIMIT ${} OFFSET ${}"#,
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

/// Delete a review by ID.
pub async fn delete_review(db: &DatabaseConnection, id: &str) -> Result<(), ApiError> {
    let sql = r#"DELETE FROM "review" WHERE id = $1"#;

    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![id.into()],
    ))
    .await
    .map_err(ApiError::Database)?;

    Ok(())
}
