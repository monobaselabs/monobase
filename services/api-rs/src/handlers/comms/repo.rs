use sea_orm::{ConnectionTrait, DatabaseConnection, FromQueryResult, JsonValue, Statement};

use crate::error::ApiError;

use super::{CreateChatRoomBody, ListChatRoomsQuery, SendChatMessageBody};

// ---------------------------------------------------------------------------
// Chat room queries
// ---------------------------------------------------------------------------

/// Create a new chat room.
pub async fn create_chat_room(
    db: &DatabaseConnection,
    created_by: &str,
    body: &CreateChatRoomBody,
) -> Result<serde_json::Value, ApiError> {
    let participants = serde_json::to_string(&body.participants)
        .unwrap_or_else(|_| "[]".to_string());
    let admins = serde_json::to_string(&body.admins)
        .unwrap_or_else(|_| "[]".to_string());

    let sql = r#"
        INSERT INTO "chat_room" (
            id, participants, admins, context_id, status,
            message_count, created_by, updated_by, created_at, updated_at, version
        )
        VALUES (
            gen_random_uuid(), $1::jsonb, $2::jsonb, $3, 'active',
            0, $4, $4, NOW(), NOW(), 1
        )
        RETURNING *
    "#;

    let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![
            participants.into(),
            admins.into(),
            body.context_id.clone().unwrap_or_default().into(),
            created_by.into(),
        ],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::internal("Failed to create chat room"))?;

    Ok(result)
}

/// Get a chat room by ID.
pub async fn get_chat_room(
    db: &DatabaseConnection,
    room_id: &str,
) -> Result<Option<serde_json::Value>, ApiError> {
    let sql = r#"SELECT * FROM "chat_room" WHERE id = $1"#;

    let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![room_id.into()],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?;

    Ok(result)
}

/// List chat rooms where the given user is a participant.
pub async fn list_chat_rooms(
    db: &DatabaseConnection,
    user_id: &str,
    offset: u64,
    limit: u64,
    query: &ListChatRoomsQuery,
) -> Result<(Vec<serde_json::Value>, u64), ApiError> {
    let mut conditions = vec![
        "participants @> $1::jsonb".to_string(),
    ];
    let mut params: Vec<sea_orm::Value> = vec![
        serde_json::json!([user_id]).to_string().into(),
    ];
    let mut idx = 2u32;

    if let Some(ref status) = query.status {
        conditions.push(format!("status = ${}", idx));
        params.push(status.clone().into());
        idx += 1;
    }

    let where_clause = format!("WHERE {}", conditions.join(" AND "));

    // Count
    let count_sql = format!(
        r#"SELECT COUNT(*) as count FROM "chat_room" {}"#,
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
        r#"SELECT * FROM "chat_room" {} ORDER BY COALESCE(last_message_at, created_at) DESC LIMIT ${} OFFSET ${}"#,
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

// ---------------------------------------------------------------------------
// Chat message queries
// ---------------------------------------------------------------------------

/// Get messages for a chat room with pagination.
pub async fn get_chat_messages(
    db: &DatabaseConnection,
    room_id: &str,
    offset: u64,
    limit: u64,
) -> Result<(Vec<serde_json::Value>, u64), ApiError> {
    // Count
    let count_sql = r#"SELECT COUNT(*) as count FROM "chat_message" WHERE chat_room_id = $1"#;
    let count_result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        count_sql,
        vec![room_id.into()],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?;

    let total_count = count_result
        .and_then(|v| v.get("count").and_then(|c| c.as_u64()))
        .unwrap_or(0);

    // Data
    let data_sql = r#"
        SELECT * FROM "chat_message"
        WHERE chat_room_id = $1
        ORDER BY timestamp ASC
        LIMIT $2 OFFSET $3
    "#;

    let data = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        data_sql,
        vec![
            room_id.into(),
            (limit as i64).into(),
            (offset as i64).into(),
        ],
    ))
    .all(db)
    .await
    .map_err(ApiError::Database)?;

    Ok((data, total_count))
}

/// Send a message and update the chat room counters atomically.
pub async fn send_chat_message(
    db: &DatabaseConnection,
    room_id: &str,
    sender_id: &str,
    created_by: &str,
    body: &SendChatMessageBody,
) -> Result<serde_json::Value, ApiError> {
    let video_call_data = body
        .video_call_data
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string());

    let insert_sql = r#"
        INSERT INTO "chat_message" (
            id, chat_room_id, sender_id, timestamp,
            message_type, message, video_call_data,
            created_by, updated_by, created_at, updated_at, version
        )
        VALUES (
            gen_random_uuid(), $1, $2, NOW(),
            $3, $4, $5::jsonb,
            $6, $6, NOW(), NOW(), 1
        )
        RETURNING *
    "#;

    let message = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        insert_sql,
        vec![
            room_id.into(),
            sender_id.into(),
            body.message_type.clone().into(),
            body.message.clone().unwrap_or_default().into(),
            video_call_data.into(),
            created_by.into(),
        ],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::internal("Failed to send message"))?;

    // Update chat_room counters
    let update_sql = r#"
        UPDATE "chat_room"
        SET
            message_count = message_count + 1,
            last_message_at = NOW(),
            updated_at = NOW(),
            version = version + 1
        WHERE id = $1
    "#;

    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        update_sql,
        vec![room_id.into()],
    ))
    .await
    .map_err(ApiError::Database)?;

    Ok(message)
}

// ---------------------------------------------------------------------------
// Video call queries
// ---------------------------------------------------------------------------

/// Find the active video call message for a room, or create one if absent.
/// Returns the video call message row.
pub async fn get_active_video_call_message(
    db: &DatabaseConnection,
    room_id: &str,
) -> Result<Option<serde_json::Value>, ApiError> {
    // Get room to find active_video_call_message_id
    let room = get_chat_room(db, room_id).await?;
    let Some(room) = room else {
        return Ok(None);
    };

    let msg_id = room
        .get("active_video_call_message_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let Some(msg_id) = msg_id else {
        return Ok(None);
    };

    let sql = r#"SELECT * FROM "chat_message" WHERE id = $1"#;
    let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![msg_id.into()],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?;

    Ok(result)
}

/// Create a video call message and mark it as active on the room.
pub async fn start_video_call(
    db: &DatabaseConnection,
    room_id: &str,
    initiator_id: &str,
    created_by: &str,
) -> Result<serde_json::Value, ApiError> {
    let video_call_data = serde_json::json!({
        "status": "active",
        "initiator_id": initiator_id,
        "participants": [initiator_id],
        "started_at": chrono::Utc::now().to_rfc3339(),
        "ended_at": null,
    });

    let insert_sql = r#"
        INSERT INTO "chat_message" (
            id, chat_room_id, sender_id, timestamp,
            message_type, message, video_call_data,
            created_by, updated_by, created_at, updated_at, version
        )
        VALUES (
            gen_random_uuid(), $1, $2, NOW(),
            'video_call', null, $3::jsonb,
            $4, $4, NOW(), NOW(), 1
        )
        RETURNING *
    "#;

    let message = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        insert_sql,
        vec![
            room_id.into(),
            initiator_id.into(),
            video_call_data.to_string().into(),
            created_by.into(),
        ],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::internal("Failed to create video call message"))?;

    let msg_id = message
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    // Mark the room as having an active video call
    let update_sql = r#"
        UPDATE "chat_room"
        SET
            active_video_call_message_id = $1,
            updated_at = NOW(),
            version = version + 1
        WHERE id = $2
    "#;

    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        update_sql,
        vec![msg_id.into(), room_id.into()],
    ))
    .await
    .map_err(ApiError::Database)?;

    Ok(message)
}

/// Update the video_call_data JSONB on a video call message.
pub async fn update_video_call_message(
    db: &DatabaseConnection,
    message_id: &str,
    updated_by: &str,
    video_call_data: serde_json::Value,
) -> Result<serde_json::Value, ApiError> {
    let sql = r#"
        UPDATE "chat_message"
        SET
            video_call_data = $1::jsonb,
            updated_by = $2,
            updated_at = NOW(),
            version = version + 1
        WHERE id = $3
        RETURNING *
    "#;

    let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![
            video_call_data.to_string().into(),
            updated_by.into(),
            message_id.into(),
        ],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::not_found("Video call message not found"))?;

    Ok(result)
}

/// Clear the active_video_call_message_id from the room.
pub async fn clear_active_video_call(
    db: &DatabaseConnection,
    room_id: &str,
) -> Result<(), ApiError> {
    let sql = r#"
        UPDATE "chat_room"
        SET
            active_video_call_message_id = NULL,
            updated_at = NOW(),
            version = version + 1
        WHERE id = $1
    "#;

    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![room_id.into()],
    ))
    .await
    .map_err(ApiError::Database)?;

    Ok(())
}
