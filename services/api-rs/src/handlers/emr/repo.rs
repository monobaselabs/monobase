use sea_orm::{DatabaseConnection, FromQueryResult, JsonValue, Statement};

use crate::error::ApiError;

use super::{CreateConsultationBody, ListConsultationsQuery, UpdateConsultationBody};

// ---------------------------------------------------------------------------
// Consultation queries
// ---------------------------------------------------------------------------

/// Create a new consultation note.
pub async fn create_consultation(
    db: &DatabaseConnection,
    created_by: &str,
    body: &CreateConsultationBody,
) -> Result<serde_json::Value, ApiError> {
    let vitals = body
        .vitals
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string());

    let symptoms = body
        .symptoms
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string());

    let prescriptions = body
        .prescriptions
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string());

    let follow_up = body
        .follow_up
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string());

    let external_documentation = body
        .external_documentation
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string());

    let sql = r#"
        INSERT INTO "consultation_note" (
            id, patient_id, provider_id, context,
            chief_complaint, assessment, plan,
            vitals, symptoms, prescriptions, follow_up, external_documentation,
            status, created_by, updated_by, created_at, updated_at, version
        )
        VALUES (
            gen_random_uuid(), $1, $2, $3,
            $4, $5, $6,
            $7::jsonb, $8::jsonb, $9::jsonb, $10::jsonb, $11::jsonb,
            'draft', $12, $12, NOW(), NOW(), 1
        )
        RETURNING *
    "#;

    let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![
            body.patient_id.clone().into(),
            body.provider_id.clone().into(),
            body.context.clone().unwrap_or_default().into(),
            body.chief_complaint.clone().unwrap_or_default().into(),
            body.assessment.clone().unwrap_or_default().into(),
            body.plan.clone().unwrap_or_default().into(),
            vitals.into(),
            symptoms.into(),
            prescriptions.into(),
            follow_up.into(),
            external_documentation.into(),
            created_by.into(),
        ],
    ))
    .one(db)
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("unique") || msg.contains("duplicate") {
            ApiError::conflict("A consultation with this context already exists")
        } else if msg.contains("foreign key") || msg.contains("violates") {
            ApiError::not_found("The referenced patient or provider does not exist")
        } else {
            ApiError::Database(e)
        }
    })?
    .ok_or_else(|| ApiError::internal("Failed to create consultation"))?;

    Ok(result)
}

/// Get a consultation by ID.
pub async fn get_consultation(
    db: &DatabaseConnection,
    id: &str,
) -> Result<Option<serde_json::Value>, ApiError> {
    let sql = r#"SELECT * FROM "consultation_note" WHERE id = $1"#;

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

/// List consultations with pagination and optional filters.
pub async fn list_consultations(
    db: &DatabaseConnection,
    offset: u64,
    limit: u64,
    query: &ListConsultationsQuery,
) -> Result<(Vec<serde_json::Value>, u64), ApiError> {
    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<sea_orm::Value> = Vec::new();
    let mut idx = 1u32;

    if let Some(ref patient_id) = query.patient_id {
        conditions.push(format!("patient_id = ${}", idx));
        params.push(patient_id.clone().into());
        idx += 1;
    }

    if let Some(ref provider_id) = query.provider_id {
        conditions.push(format!("provider_id = ${}", idx));
        params.push(provider_id.clone().into());
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
        r#"SELECT COUNT(*) as count FROM "consultation_note" {}"#,
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
        r#"SELECT * FROM "consultation_note" {} ORDER BY created_at DESC LIMIT ${} OFFSET ${}"#,
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

/// Update a draft consultation.
pub async fn update_consultation(
    db: &DatabaseConnection,
    id: &str,
    updated_by: &str,
    body: &UpdateConsultationBody,
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

    maybe_set_text!(chief_complaint, "chief_complaint");
    maybe_set_text!(assessment, "assessment");
    maybe_set_text!(plan, "plan");
    maybe_set_jsonb!(vitals, "vitals");
    maybe_set_jsonb!(symptoms, "symptoms");
    maybe_set_jsonb!(prescriptions, "prescriptions");
    maybe_set_jsonb!(follow_up, "follow_up");
    maybe_set_jsonb!(external_documentation, "external_documentation");

    set_clauses.push(format!("updated_by = ${}", idx));
    params.push(updated_by.into());
    idx += 1;

    set_clauses.push("version = version + 1".to_string());

    params.push(id.into());

    let sql = format!(
        r#"UPDATE "consultation_note" SET {} WHERE id = ${} RETURNING *"#,
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
    .ok_or_else(|| ApiError::not_found("Consultation not found"))?;

    Ok(result)
}

/// Finalize a consultation: set status to 'finalized', record who finalized and when.
pub async fn finalize_consultation(
    db: &DatabaseConnection,
    id: &str,
    finalized_by: &str,
) -> Result<serde_json::Value, ApiError> {
    let sql = r#"
        UPDATE "consultation_note"
        SET
            status = 'finalized',
            finalized_at = NOW(),
            finalized_by = $1,
            updated_by = $1,
            updated_at = NOW(),
            version = version + 1
        WHERE id = $2
        RETURNING *
    "#;

    let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![finalized_by.into(), id.into()],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::not_found("Consultation not found"))?;

    Ok(result)
}

/// List distinct patients a provider has consultations with (joined with person data).
pub async fn list_emr_patients(
    db: &DatabaseConnection,
    provider_id: Option<&str>,
    offset: u64,
    limit: u64,
    search: Option<&str>,
) -> Result<(Vec<serde_json::Value>, u64), ApiError> {
    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<sea_orm::Value> = Vec::new();
    let mut idx = 1u32;

    if let Some(pid) = provider_id {
        conditions.push(format!("cn.provider_id = ${}", idx));
        params.push(pid.into());
        idx += 1;
    }

    if let Some(q) = search {
        let pattern = format!("%{}%", q);
        conditions.push(format!(
            "(per.first_name ILIKE ${} OR per.last_name ILIKE ${})",
            idx, idx
        ));
        params.push(pattern.into());
        idx += 1;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let base_query = r#"
        FROM "consultation_note" cn
        JOIN "patient" pt ON cn.patient_id = pt.id
        JOIN "person" per ON pt.person_id = per.id
    "#;

    // Count distinct patients
    let count_sql = format!(
        "SELECT COUNT(DISTINCT cn.patient_id) as count {} {}",
        base_query, where_clause
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

    // Data: distinct patients with person information
    let data_sql = format!(
        r#"
        SELECT DISTINCT ON (cn.patient_id)
            pt.id as patient_id,
            pt.person_id,
            per.first_name,
            per.last_name,
            per.date_of_birth,
            per.gender,
            per.contact_info,
            MAX(cn.created_at) OVER (PARTITION BY cn.patient_id) as last_consultation_at
        {}
        {}
        ORDER BY cn.patient_id, cn.created_at DESC
        LIMIT ${} OFFSET ${}
        "#,
        base_query, where_clause, idx, idx + 1,
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
