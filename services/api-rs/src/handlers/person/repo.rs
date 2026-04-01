use sea_orm::{DatabaseConnection, FromQueryResult, JsonValue, Statement};

use crate::error::ApiError;

use super::{CreatePersonBody, UpdatePersonBody};

/// Create a new person record.
pub async fn create_person(
    db: &DatabaseConnection,
    created_by: &str,
    body: &CreatePersonBody,
) -> Result<serde_json::Value, ApiError> {
    let id = uuid::Uuid::new_v4().to_string();

    let languages = body
        .languages_spoken
        .as_ref()
        .map(|l| serde_json::to_string(l).unwrap_or_else(|_| "[]".to_string()))
        .unwrap_or_else(|| "null".to_string());

    let primary_address = body
        .primary_address
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string());

    let contact_info = body
        .contact_info
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string());

    let avatar = body
        .avatar
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string());

    let sql = r#"
        INSERT INTO "person" (
            id, first_name, last_name, middle_name, date_of_birth, gender,
            primary_address, contact_info, avatar, languages_spoken, timezone,
            created_by, updated_by, created_at, updated_at, version
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb, $8::jsonb, $9::jsonb, $10::jsonb, $11, $12, $12, NOW(), NOW(), 1)
        RETURNING *
    "#;

    let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![
            id.into(),
            body.first_name.clone().into(),
            body.last_name.clone().unwrap_or_default().into(),
            body.middle_name.clone().unwrap_or_default().into(),
            body.date_of_birth.clone().unwrap_or_default().into(),
            body.gender.clone().unwrap_or_default().into(),
            primary_address.into(),
            contact_info.into(),
            avatar.into(),
            languages.into(),
            body.timezone.clone().unwrap_or_default().into(),
            created_by.into(),
        ],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::internal("Failed to create person"))?;

    Ok(result)
}

/// Get a person by ID.
pub async fn get_person(
    db: &DatabaseConnection,
    id: &str,
) -> Result<Option<serde_json::Value>, ApiError> {
    let sql = r#"SELECT * FROM "person" WHERE id = $1"#;

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

/// List persons with pagination and optional search.
pub async fn list_persons(
    db: &DatabaseConnection,
    offset: u64,
    limit: u64,
    search: Option<&str>,
) -> Result<(Vec<serde_json::Value>, u64), ApiError> {
    let (where_clause, params) = if let Some(q) = search {
        let pattern = format!("%{}%", q);
        (
            "WHERE first_name ILIKE $1 OR last_name ILIKE $1".to_string(),
            vec![pattern.into()],
        )
    } else {
        (String::new(), vec![])
    };

    // Count
    let count_sql = format!(
        r#"SELECT COUNT(*) as count FROM "person" {}"#,
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
        r#"SELECT * FROM "person" {} ORDER BY created_at DESC LIMIT ${} OFFSET ${}"#,
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

/// Update a person by ID.
pub async fn update_person(
    db: &DatabaseConnection,
    id: &str,
    updated_by: &str,
    body: &UpdatePersonBody,
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

    maybe_set!(first_name, "first_name");
    maybe_set!(last_name, "last_name");
    maybe_set!(middle_name, "middle_name");
    maybe_set!(date_of_birth, "date_of_birth");
    maybe_set!(gender, "gender");
    maybe_set!(timezone, "timezone");

    if let Some(ref val) = body.primary_address {
        set_clauses.push(format!("primary_address = ${}::jsonb", idx));
        params.push(val.to_string().into());
        idx += 1;
    }
    if let Some(ref val) = body.contact_info {
        set_clauses.push(format!("contact_info = ${}::jsonb", idx));
        params.push(val.to_string().into());
        idx += 1;
    }
    if let Some(ref val) = body.avatar {
        set_clauses.push(format!("avatar = ${}::jsonb", idx));
        params.push(val.to_string().into());
        idx += 1;
    }
    if let Some(ref val) = body.languages_spoken {
        set_clauses.push(format!("languages_spoken = ${}::jsonb", idx));
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
        r#"UPDATE "person" SET {} WHERE id = ${} RETURNING *"#,
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
    .ok_or_else(|| ApiError::not_found("Person not found"))?;

    Ok(result)
}
