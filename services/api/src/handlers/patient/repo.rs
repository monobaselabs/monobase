use sea_orm::{ConnectionTrait, DatabaseConnection, FromQueryResult, JsonValue, Statement};

use crate::error::ApiError;

use super::{CreatePatientBody, UpdatePatientBody};

/// Create a new patient record.
pub async fn create_patient(
    db: &DatabaseConnection,
    created_by: &str,
    body: &CreatePatientBody,
) -> Result<serde_json::Value, ApiError> {
    let primary_provider = body
        .primary_provider
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string());

    let primary_pharmacy = body
        .primary_pharmacy
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string());

    let sql = r#"
        INSERT INTO "patient" (
            id, person_id, primary_provider, primary_pharmacy,
            created_by, updated_by, created_at, updated_at, version
        )
        VALUES (gen_random_uuid(), $1, $2::jsonb, $3::jsonb, $4, $4, NOW(), NOW(), 1)
        RETURNING *
    "#;

    let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![
            body.person_id.clone().into(),
            primary_provider.into(),
            primary_pharmacy.into(),
            created_by.into(),
        ],
    ))
    .one(db)
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("unique") || msg.contains("duplicate") {
            ApiError::conflict("A patient profile already exists for this person")
        } else if msg.contains("foreign key") || msg.contains("violates") {
            ApiError::not_found("The referenced person does not exist")
        } else {
            ApiError::Database(e)
        }
    })?
    .ok_or_else(|| ApiError::internal("Failed to create patient"))?;

    Ok(result)
}

/// Get a patient by ID.
pub async fn get_patient(
    db: &DatabaseConnection,
    id: &str,
) -> Result<Option<serde_json::Value>, ApiError> {
    let sql = r#"SELECT * FROM "patient" WHERE id = $1"#;

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

/// List patients with pagination and optional search by person name.
pub async fn list_patients(
    db: &DatabaseConnection,
    offset: u64,
    limit: u64,
    search: Option<&str>,
) -> Result<(Vec<serde_json::Value>, u64), ApiError> {
    let (where_clause, params) = if let Some(q) = search {
        let pattern = format!("%{}%", q);
        (
            "WHERE p.first_name ILIKE $1 OR p.last_name ILIKE $1".to_string(),
            vec![pattern.into()],
        )
    } else {
        (String::new(), vec![])
    };

    // Count
    let count_sql = format!(
        r#"SELECT COUNT(*) as count FROM "patient" pt JOIN "person" p ON pt.person_id = p.id {}"#,
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
        r#"SELECT pt.* FROM "patient" pt JOIN "person" p ON pt.person_id = p.id {} ORDER BY pt.created_at DESC LIMIT ${} OFFSET ${}"#,
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

/// Update a patient by ID.
pub async fn update_patient(
    db: &DatabaseConnection,
    id: &str,
    updated_by: &str,
    body: &UpdatePatientBody,
) -> Result<serde_json::Value, ApiError> {
    let mut set_clauses = vec!["updated_at = NOW()".to_string()];
    let mut params: Vec<sea_orm::Value> = vec![];
    let mut idx = 1u32;

    if let Some(ref val) = body.primary_provider {
        set_clauses.push(format!("primary_provider = ${}::jsonb", idx));
        params.push(val.to_string().into());
        idx += 1;
    }
    if let Some(ref val) = body.primary_pharmacy {
        set_clauses.push(format!("primary_pharmacy = ${}::jsonb", idx));
        params.push(val.to_string().into());
        idx += 1;
    }

    set_clauses.push(format!("updated_by = ${}", idx));
    params.push(updated_by.into());
    idx += 1;

    // version bump (optimistic locking)
    set_clauses.push("version = version + 1".to_string());

    params.push(id.into());

    let sql = format!(
        r#"UPDATE "patient" SET {} WHERE id = ${} RETURNING *"#,
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
    .ok_or_else(|| ApiError::not_found("Patient not found"))?;

    Ok(result)
}

/// Hard-delete a patient by ID.
pub async fn delete_patient(
    db: &DatabaseConnection,
    id: &str,
) -> Result<(), ApiError> {
    let sql = r#"DELETE FROM "patient" WHERE id = $1"#;

    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![id.into()],
    ))
    .await
    .map_err(ApiError::Database)?;

    Ok(())
}
