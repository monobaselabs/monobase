use sea_orm::{ConnectionTrait, DatabaseConnection, FromQueryResult, JsonValue, Statement};

use crate::error::ApiError;

use super::{
    CreateBookingBody, CreateBookingEventBody, CreateScheduleExceptionBody,
    ListBookingEventsQuery, ListBookingsQuery, ListScheduleExceptionsQuery, ListSlotsQuery,
    UpdateBookingEventBody,
};

// ---------------------------------------------------------------------------
// Booking Event
// ---------------------------------------------------------------------------

pub async fn create_booking_event(
    db: &DatabaseConnection,
    created_by: &str,
    body: &CreateBookingEventBody,
) -> Result<serde_json::Value, ApiError> {
    let id = uuid::Uuid::new_v4().to_string();

    let keywords = serde_json::to_string(&body.keywords.as_deref().unwrap_or(&[]))
        .unwrap_or_else(|_| "[]".to_string());
    let tags = serde_json::to_string(&body.tags.as_deref().unwrap_or(&[]))
        .unwrap_or_else(|_| "[]".to_string());

    let location_types = body
        .location_types
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| r#"["video","phone","in-person"]"#.to_string());

    let form_config = body
        .form_config
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string());

    let billing_config = body
        .billing_config
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string());

    let daily_configs = serde_json::to_string(&body.daily_configs)
        .unwrap_or_else(|_| "[]".to_string());

    let sql = r#"
        INSERT INTO "booking_event" (
            id, owner_id, context_id, title, description,
            keywords, tags, timezone,
            location_types, max_booking_days, min_booking_minutes,
            form_config, billing_config, status,
            effective_from, effective_to, daily_configs,
            created_by, updated_by, created_at, updated_at, version
        )
        VALUES (
            $1, $2, $3, $4, $5,
            $6::jsonb, $7::jsonb, $8,
            $9::jsonb, $10, $11,
            $12::jsonb, $13::jsonb, $14,
            COALESCE($15::timestamptz, NOW()), $16::timestamptz, $17::jsonb,
            $18, $18, NOW(), NOW(), 1
        )
        RETURNING *
    "#;

    let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![
            id.into(),
            body.owner_id.clone().into(),
            body.context_id.clone().into(),
            body.title.clone().into(),
            body.description.clone().into(),
            keywords.into(),
            tags.into(),
            body.timezone
                .clone()
                .unwrap_or_else(|| "America/New_York".to_string())
                .into(),
            location_types.into(),
            (body.max_booking_days.unwrap_or(30) as i32).into(),
            (body.min_booking_minutes.unwrap_or(1440) as i32).into(),
            form_config.into(),
            billing_config.into(),
            body.status
                .clone()
                .unwrap_or_else(|| "active".to_string())
                .into(),
            body.effective_from.clone().into(),
            body.effective_to.clone().into(),
            daily_configs.into(),
            created_by.into(),
        ],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::internal("Failed to create booking event"))?;

    Ok(result)
}

pub async fn get_booking_event(
    db: &DatabaseConnection,
    id: &str,
) -> Result<Option<serde_json::Value>, ApiError> {
    let sql = r#"SELECT * FROM "booking_event" WHERE id = $1"#;

    JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![id.into()],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)
}

pub async fn list_booking_events(
    db: &DatabaseConnection,
    offset: u64,
    limit: u64,
    query: &ListBookingEventsQuery,
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

    let count_sql = format!(
        r#"SELECT COUNT(*) as count FROM "booking_event" {}"#,
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

    let data_sql = format!(
        r#"SELECT * FROM "booking_event" {} ORDER BY created_at DESC LIMIT ${} OFFSET ${}"#,
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

pub async fn update_booking_event(
    db: &DatabaseConnection,
    id: &str,
    updated_by: &str,
    body: &UpdateBookingEventBody,
) -> Result<serde_json::Value, ApiError> {
    let mut set_clauses = vec!["updated_at = NOW()".to_string(), "version = version + 1".to_string()];
    let mut params: Vec<sea_orm::Value> = Vec::new();
    let mut idx = 1u32;

    macro_rules! maybe_set_str {
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

    maybe_set_str!(title, "title");
    maybe_set_str!(description, "description");
    maybe_set_str!(timezone, "timezone");
    maybe_set_str!(status, "status");
    maybe_set_str!(context_id, "context_id");
    maybe_set_str!(effective_from, "effective_from");
    maybe_set_str!(effective_to, "effective_to");
    maybe_set_jsonb!(location_types, "location_types");
    maybe_set_jsonb!(form_config, "form_config");
    maybe_set_jsonb!(billing_config, "billing_config");
    maybe_set_jsonb!(daily_configs, "daily_configs");

    if let Some(ref v) = body.keywords {
        let s = serde_json::to_string(v).unwrap_or_else(|_| "[]".to_string());
        set_clauses.push(format!("keywords = ${}::jsonb", idx));
        params.push(s.into());
        idx += 1;
    }
    if let Some(ref v) = body.tags {
        let s = serde_json::to_string(v).unwrap_or_else(|_| "[]".to_string());
        set_clauses.push(format!("tags = ${}::jsonb", idx));
        params.push(s.into());
        idx += 1;
    }
    if let Some(v) = body.max_booking_days {
        set_clauses.push(format!("max_booking_days = ${}", idx));
        params.push((v as i32).into());
        idx += 1;
    }
    if let Some(v) = body.min_booking_minutes {
        set_clauses.push(format!("min_booking_minutes = ${}", idx));
        params.push((v as i32).into());
        idx += 1;
    }

    set_clauses.push(format!("updated_by = ${}", idx));
    params.push(updated_by.into());
    idx += 1;

    params.push(id.into());

    let sql = format!(
        r#"UPDATE "booking_event" SET {} WHERE id = ${} RETURNING *"#,
        set_clauses.join(", "),
        idx,
    );

    JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        &sql,
        params,
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::not_found("Booking event not found"))
}

pub async fn delete_booking_event(db: &DatabaseConnection, id: &str) -> Result<(), ApiError> {
    let sql = r#"DELETE FROM "booking_event" WHERE id = $1"#;

    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![id.into()],
    ))
    .await
    .map_err(ApiError::Database)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Booking
// ---------------------------------------------------------------------------

pub async fn create_booking(
    db: &DatabaseConnection,
    created_by: &str,
    body: &CreateBookingBody,
) -> Result<serde_json::Value, ApiError> {
    // Fetch and validate the slot
    let slot = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        r#"SELECT * FROM "time_slot" WHERE id = $1"#,
        vec![body.slot_id.clone().into()],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound {
        message: "Time slot not found".to_string(),
        resource_type: Some("TimeSlot".to_string()),
        resource: Some(body.slot_id.clone()),
    })?;

    let slot_status = slot
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if slot_status != "available" {
        return Err(ApiError::BusinessLogic {
            message: format!("Slot is not available (current status: {})", slot_status),
            code: "SLOT_NOT_AVAILABLE".to_string(),
        });
    }

    let scheduled_at = slot
        .get("start_time")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let booking_id = uuid::Uuid::new_v4().to_string();

    let form_responses = body
        .form_responses
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string());

    let sql = r#"
        INSERT INTO "booking" (
            id, client_id, provider_id, slot_id,
            location_type, reason, status,
            booked_at, scheduled_at, duration_minutes,
            form_responses,
            created_by, updated_by, created_at, updated_at, version
        )
        VALUES (
            $1, $2, $3, $4,
            $5, $6, 'pending',
            NOW(), $7::timestamptz, $8,
            $9::jsonb,
            $10, $10, NOW(), NOW(), 1
        )
        RETURNING *
    "#;

    let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![
            booking_id.clone().into(),
            body.client_id.clone().into(),
            body.provider_id.clone().into(),
            body.slot_id.clone().into(),
            body.location_type.clone().into(),
            body.reason.clone().into(),
            scheduled_at.into(),
            (body.duration_minutes as i32).into(),
            form_responses.into(),
            created_by.into(),
        ],
    ))
    .one(db)
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("unique") || msg.contains("duplicate") || msg.contains("23505") {
            ApiError::conflict("A booking already exists for this slot")
        } else {
            ApiError::Database(e)
        }
    })?
    .ok_or_else(|| ApiError::internal("Failed to create booking"))?;

    // Mark the slot as booked
    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        r#"UPDATE "time_slot" SET status = 'booked', booking_id = $1, updated_at = NOW(), version = version + 1 WHERE id = $2"#,
        vec![booking_id.into(), body.slot_id.clone().into()],
    ))
    .await
    .map_err(ApiError::Database)?;

    Ok(result)
}

pub async fn get_booking(
    db: &DatabaseConnection,
    id: &str,
) -> Result<Option<serde_json::Value>, ApiError> {
    let sql = r#"SELECT * FROM "booking" WHERE id = $1"#;

    JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![id.into()],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)
}

pub async fn list_bookings(
    db: &DatabaseConnection,
    offset: u64,
    limit: u64,
    query: &ListBookingsQuery,
) -> Result<(Vec<serde_json::Value>, u64), ApiError> {
    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<sea_orm::Value> = Vec::new();
    let mut idx = 1u32;

    if let Some(ref client_id) = query.client_id {
        conditions.push(format!("client_id = ${}", idx));
        params.push(client_id.clone().into());
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

    let count_sql = format!(
        r#"SELECT COUNT(*) as count FROM "booking" {}"#,
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

    let data_sql = format!(
        r#"SELECT * FROM "booking" {} ORDER BY created_at DESC LIMIT ${} OFFSET ${}"#,
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

pub async fn cancel_booking(
    db: &DatabaseConnection,
    id: &str,
    cancelled_by: &str,
    reason: Option<&str>,
) -> Result<serde_json::Value, ApiError> {
    let booking = get_booking(db, id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Booking not found".to_string(),
            resource_type: Some("Booking".to_string()),
            resource: Some(id.to_string()),
        })?;

    let status = booking
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if status != "pending" && status != "confirmed" {
        return Err(ApiError::BusinessLogic {
            message: format!("Cannot cancel a booking with status '{}'", status),
            code: "INVALID_STATUS_TRANSITION".to_string(),
        });
    }

    let slot_id = booking
        .get("slot_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        r#"
            UPDATE "booking"
            SET status = 'cancelled',
                cancelled_by = $1,
                cancellation_reason = $2,
                cancelled_at = NOW(),
                updated_at = NOW(),
                updated_by = $1,
                version = version + 1
            WHERE id = $3
            RETURNING *
        "#,
        vec![
            cancelled_by.into(),
            reason.unwrap_or("").into(),
            id.into(),
        ],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::internal("Failed to cancel booking"))?;

    // Release the slot back to available
    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        r#"UPDATE "time_slot" SET status = 'available', booking_id = NULL, updated_at = NOW(), version = version + 1 WHERE id = $1"#,
        vec![slot_id.into()],
    ))
    .await
    .map_err(ApiError::Database)?;

    Ok(result)
}

pub async fn confirm_booking(
    db: &DatabaseConnection,
    id: &str,
    confirmed_by: &str,
) -> Result<serde_json::Value, ApiError> {
    let booking = get_booking(db, id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Booking not found".to_string(),
            resource_type: Some("Booking".to_string()),
            resource: Some(id.to_string()),
        })?;

    let status = booking
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if status != "pending" {
        return Err(ApiError::BusinessLogic {
            message: format!("Cannot confirm a booking with status '{}'", status),
            code: "INVALID_STATUS_TRANSITION".to_string(),
        });
    }

    JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        r#"
            UPDATE "booking"
            SET status = 'confirmed',
                confirmation_timestamp = NOW(),
                updated_at = NOW(),
                updated_by = $1,
                version = version + 1
            WHERE id = $2
            RETURNING *
        "#,
        vec![confirmed_by.into(), id.into()],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::internal("Failed to confirm booking"))
}

pub async fn reject_booking(
    db: &DatabaseConnection,
    id: &str,
    rejected_by: &str,
    reason: Option<&str>,
) -> Result<serde_json::Value, ApiError> {
    let booking = get_booking(db, id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Booking not found".to_string(),
            resource_type: Some("Booking".to_string()),
            resource: Some(id.to_string()),
        })?;

    let status = booking
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if status != "pending" {
        return Err(ApiError::BusinessLogic {
            message: format!("Cannot reject a booking with status '{}'", status),
            code: "INVALID_STATUS_TRANSITION".to_string(),
        });
    }

    let slot_id = booking
        .get("slot_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        r#"
            UPDATE "booking"
            SET status = 'rejected',
                cancellation_reason = $1,
                cancelled_by = $2,
                cancelled_at = NOW(),
                updated_at = NOW(),
                updated_by = $2,
                version = version + 1
            WHERE id = $3
            RETURNING *
        "#,
        vec![
            reason.unwrap_or("").into(),
            rejected_by.into(),
            id.into(),
        ],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::internal("Failed to reject booking"))?;

    // Release the slot
    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        r#"UPDATE "time_slot" SET status = 'available', booking_id = NULL, updated_at = NOW(), version = version + 1 WHERE id = $1"#,
        vec![slot_id.into()],
    ))
    .await
    .map_err(ApiError::Database)?;

    Ok(result)
}

pub async fn mark_no_show(
    db: &DatabaseConnection,
    id: &str,
    marked_by: &str,
) -> Result<serde_json::Value, ApiError> {
    let booking = get_booking(db, id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Booking not found".to_string(),
            resource_type: Some("Booking".to_string()),
            resource: Some(id.to_string()),
        })?;

    let status = booking
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if status != "confirmed" {
        return Err(ApiError::BusinessLogic {
            message: format!("Cannot mark no-show for a booking with status '{}'", status),
            code: "INVALID_STATUS_TRANSITION".to_string(),
        });
    }

    JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        r#"
            UPDATE "booking"
            SET status = 'no_show',
                no_show_marked_by = $1,
                no_show_marked_at = NOW(),
                updated_at = NOW(),
                updated_by = $1,
                version = version + 1
            WHERE id = $2
            RETURNING *
        "#,
        vec![marked_by.into(), id.into()],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::internal("Failed to mark no-show"))
}

// ---------------------------------------------------------------------------
// Time Slots
// ---------------------------------------------------------------------------

pub async fn get_time_slot(
    db: &DatabaseConnection,
    id: &str,
) -> Result<Option<serde_json::Value>, ApiError> {
    let sql = r#"SELECT * FROM "time_slot" WHERE id = $1"#;

    JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![id.into()],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)
}

pub async fn list_event_slots(
    db: &DatabaseConnection,
    event_id: &str,
    offset: u64,
    limit: u64,
    query: &ListSlotsQuery,
) -> Result<(Vec<serde_json::Value>, u64), ApiError> {
    let mut conditions = vec![format!("event_id = $1")];
    let mut params: Vec<sea_orm::Value> = vec![event_id.into()];
    let mut idx = 2u32;

    if let Some(ref status) = query.status {
        conditions.push(format!("status = ${}", idx));
        params.push(status.clone().into());
        idx += 1;
    }

    let where_clause = format!("WHERE {}", conditions.join(" AND "));

    let count_sql = format!(
        r#"SELECT COUNT(*) as count FROM "time_slot" {}"#,
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

    let data_sql = format!(
        r#"SELECT * FROM "time_slot" {} ORDER BY start_time ASC LIMIT ${} OFFSET ${}"#,
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

// ---------------------------------------------------------------------------
// Schedule Exceptions
// ---------------------------------------------------------------------------

pub async fn create_schedule_exception(
    db: &DatabaseConnection,
    event_id: &str,
    created_by: &str,
    body: &CreateScheduleExceptionBody,
) -> Result<serde_json::Value, ApiError> {
    let id = uuid::Uuid::new_v4().to_string();

    let recurrence_pattern = body
        .recurrence_pattern
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string());

    let sql = r#"
        INSERT INTO "schedule_exception" (
            id, event_id, owner_id, context_id, timezone,
            start_datetime, end_datetime, reason,
            recurring, recurrence_pattern,
            created_by, updated_by, created_at, updated_at, version
        )
        VALUES (
            $1, $2, $3, $4, $5,
            $6::timestamptz, $7::timestamptz, $8,
            $9, $10::jsonb,
            $11, $11, NOW(), NOW(), 1
        )
        RETURNING *
    "#;

    JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![
            id.into(),
            event_id.into(),
            body.owner_id.clone().into(),
            body.context_id.clone().into(),
            body.timezone
                .clone()
                .unwrap_or_else(|| "America/New_York".to_string())
                .into(),
            body.start_datetime.clone().into(),
            body.end_datetime.clone().into(),
            body.reason.clone().into(),
            body.recurring.unwrap_or(false).into(),
            recurrence_pattern.into(),
            created_by.into(),
        ],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::internal("Failed to create schedule exception"))
}

pub async fn get_schedule_exception(
    db: &DatabaseConnection,
    event_id: &str,
    exception_id: &str,
) -> Result<Option<serde_json::Value>, ApiError> {
    let sql = r#"SELECT * FROM "schedule_exception" WHERE id = $1 AND event_id = $2"#;

    JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![exception_id.into(), event_id.into()],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)
}

pub async fn list_schedule_exceptions(
    db: &DatabaseConnection,
    event_id: &str,
    offset: u64,
    limit: u64,
    query: &ListScheduleExceptionsQuery,
) -> Result<(Vec<serde_json::Value>, u64), ApiError> {
    let mut conditions = vec![format!("event_id = $1")];
    let mut params: Vec<sea_orm::Value> = vec![event_id.into()];
    let mut idx = 2u32;

    if let Some(recurring) = query.recurring {
        conditions.push(format!("recurring = ${}", idx));
        params.push(recurring.into());
        idx += 1;
    }

    let where_clause = format!("WHERE {}", conditions.join(" AND "));

    let count_sql = format!(
        r#"SELECT COUNT(*) as count FROM "schedule_exception" {}"#,
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

    let data_sql = format!(
        r#"SELECT * FROM "schedule_exception" {} ORDER BY start_datetime ASC LIMIT ${} OFFSET ${}"#,
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

pub async fn delete_schedule_exception(
    db: &DatabaseConnection,
    event_id: &str,
    exception_id: &str,
) -> Result<(), ApiError> {
    let sql = r#"DELETE FROM "schedule_exception" WHERE id = $1 AND event_id = $2"#;

    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![exception_id.into(), event_id.into()],
    ))
    .await
    .map_err(ApiError::Database)?;

    Ok(())
}
