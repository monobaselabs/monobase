use sea_orm::{ConnectionTrait, DatabaseConnection, FromQueryResult, JsonValue, Statement};

use crate::error::ApiError;

use super::{CreateProviderBody, UpdateProviderBody};

/// Create a new provider record.
pub async fn create_provider(
    db: &DatabaseConnection,
    created_by: &str,
    body: &CreateProviderBody,
) -> Result<serde_json::Value, ApiError> {
    let id = uuid::Uuid::new_v4().to_string();

    let minor_ailments_specialties = body
        .minor_ailments_specialties
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "[]".to_string()))
        .unwrap_or_else(|| "null".to_string());

    let minor_ailments_practice_locations = body
        .minor_ailments_practice_locations
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "[]".to_string()))
        .unwrap_or_else(|| "null".to_string());

    let sql = r#"
        INSERT INTO "provider" (
            id, person_id, provider_type, years_of_experience, biography,
            minor_ailments_specialties, minor_ailments_practice_locations,
            created_by, updated_by, created_at, updated_at, version
        )
        VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7::jsonb, $8, $8, NOW(), NOW(), 1)
        RETURNING *
    "#;

    let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![
            id.into(),
            body.person_id.clone().into(),
            body.provider_type.clone().into(),
            body.years_of_experience.map(|v| v as i64).into(),
            body.biography.clone().unwrap_or_default().into(),
            minor_ailments_specialties.into(),
            minor_ailments_practice_locations.into(),
            created_by.into(),
        ],
    ))
    .one(db)
    .await
    .map_err(|e| {
        // Detect unique constraint violation on person_id
        let msg = e.to_string();
        if msg.contains("unique") || msg.contains("duplicate") {
            ApiError::conflict("A provider profile already exists for this person")
        } else {
            ApiError::Database(e)
        }
    })?
    .ok_or_else(|| ApiError::internal("Failed to create provider"))?;

    Ok(result)
}

/// Get a provider by ID.
pub async fn get_provider(
    db: &DatabaseConnection,
    id: &str,
) -> Result<Option<serde_json::Value>, ApiError> {
    let sql = r#"SELECT * FROM "provider" WHERE id = $1"#;

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

/// List providers with pagination and optional search.
pub async fn list_providers(
    db: &DatabaseConnection,
    offset: u64,
    limit: u64,
    search: Option<&str>,
) -> Result<(Vec<serde_json::Value>, u64), ApiError> {
    let (where_clause, params) = if let Some(q) = search {
        let pattern = format!("%{}%", q);
        (
            "WHERE provider_type ILIKE $1 OR biography ILIKE $1".to_string(),
            vec![pattern.into()],
        )
    } else {
        (String::new(), vec![])
    };

    // Count
    let count_sql = format!(
        r#"SELECT COUNT(*) as count FROM "provider" {}"#,
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
    let param_offset = if search.is_some() { 2 } else { 1 };
    let data_sql = format!(
        r#"SELECT * FROM "provider" {} ORDER BY created_at DESC LIMIT ${} OFFSET ${}"#,
        where_clause,
        param_offset,
        param_offset + 1,
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

/// Update a provider by ID.
pub async fn update_provider(
    db: &DatabaseConnection,
    id: &str,
    updated_by: &str,
    body: &UpdateProviderBody,
) -> Result<serde_json::Value, ApiError> {
    let mut set_clauses = vec!["updated_at = NOW()".to_string()];
    let mut params: Vec<sea_orm::Value> = vec![];
    let mut idx = 1u32;

    macro_rules! maybe_set {
        ($field:ident, $col:expr) => {
            if let Some(ref val) = body.$field {
                set_clauses.push(format!("{} = ${}", $col, idx));
                params.push(val.clone().into());
                idx += 1;
            }
        };
    }

    maybe_set!(provider_type, "provider_type");
    maybe_set!(biography, "biography");

    if let Some(val) = body.years_of_experience {
        set_clauses.push(format!("years_of_experience = ${}", idx));
        params.push((val as i64).into());
        idx += 1;
    }

    if let Some(ref val) = body.minor_ailments_specialties {
        set_clauses.push(format!("minor_ailments_specialties = ${}::jsonb", idx));
        params.push(serde_json::to_string(val).unwrap_or_else(|_| "[]".to_string()).into());
        idx += 1;
    }

    if let Some(ref val) = body.minor_ailments_practice_locations {
        set_clauses.push(format!("minor_ailments_practice_locations = ${}::jsonb", idx));
        params.push(serde_json::to_string(val).unwrap_or_else(|_| "[]".to_string()).into());
        idx += 1;
    }

    set_clauses.push(format!("updated_by = ${}", idx));
    params.push(updated_by.into());
    idx += 1;

    // version bump (optimistic locking)
    set_clauses.push("version = version + 1".to_string());

    params.push(id.into());

    let sql = format!(
        r#"UPDATE "provider" SET {} WHERE id = ${} RETURNING *"#,
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
    .ok_or_else(|| ApiError::not_found("Provider not found"))?;

    Ok(result)
}

/// Delete a provider by ID.
pub async fn delete_provider(
    db: &DatabaseConnection,
    id: &str,
) -> Result<(), ApiError> {
    let sql = r#"DELETE FROM "provider" WHERE id = $1"#;

    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![id.into()],
    ))
    .await
    .map_err(ApiError::Database)?;

    Ok(())
}
