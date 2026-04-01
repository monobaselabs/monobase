mod repo;

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;

use crate::error::{ApiError, FieldError};
use crate::handlers::AppState;
use crate::middleware::auth as auth_mw;
use crate::model::{PaginatedResult, PaginationMeta};

// ---------------------------------------------------------------------------
// Request / query types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateChatRoomBody {
    /// List of person IDs that are participants in this room.
    pub participants: Vec<String>,
    /// List of person IDs that have admin rights in this room.
    pub admins: Vec<String>,
    /// Optional external context identifier (e.g. booking ID).
    pub context_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListChatRoomsQuery {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    /// Filter by room status (e.g. "active", "archived").
    pub status: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListMessagesQuery {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendChatMessageBody {
    /// Message type: "text", "system", or "video_call".
    pub message_type: String,
    /// Message text content (required for type "text", max 5000 chars).
    pub message: Option<String>,
    /// Optional video call payload for type "video_call".
    pub video_call_data: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateVideoCallParticipantBody {
    /// The participant's person ID.
    pub participant_id: String,
    /// Action to perform: "add" or "remove".
    pub action: String,
    /// Optional extra metadata for the participant slot.
    pub metadata: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

fn validate_send_message(body: &SendChatMessageBody) -> Result<(), ApiError> {
    let mut field_errors: Vec<FieldError> = Vec::new();

    let valid_types = ["text", "system", "video_call"];
    if !valid_types.contains(&body.message_type.as_str()) {
        field_errors.push(FieldError {
            field: "messageType".to_string(),
            code: "INVALID".to_string(),
            message: "messageType must be one of: text, system, video_call".to_string(),
            value: Some(serde_json::json!(body.message_type)),
        });
    }

    if body.message_type == "text" {
        match &body.message {
            None => {
                field_errors.push(FieldError {
                    field: "message".to_string(),
                    code: "REQUIRED".to_string(),
                    message: "message is required for messageType 'text'".to_string(),
                    value: None,
                });
            }
            Some(text) if text.len() > 5000 => {
                field_errors.push(FieldError {
                    field: "message".to_string(),
                    code: "MAX_LENGTH".to_string(),
                    message: "message must not exceed 5000 characters".to_string(),
                    value: None,
                });
            }
            _ => {}
        }
    }

    if !field_errors.is_empty() {
        return Err(ApiError::validation("Validation failed", field_errors));
    }

    Ok(())
}

fn validate_create_room(body: &CreateChatRoomBody) -> Result<(), ApiError> {
    let mut field_errors: Vec<FieldError> = Vec::new();

    if body.participants.is_empty() {
        field_errors.push(FieldError {
            field: "participants".to_string(),
            code: "REQUIRED".to_string(),
            message: "participants must contain at least one person ID".to_string(),
            value: None,
        });
    }

    if body.admins.is_empty() {
        field_errors.push(FieldError {
            field: "admins".to_string(),
            code: "REQUIRED".to_string(),
            message: "admins must contain at least one person ID".to_string(),
            value: None,
        });
    }

    if !field_errors.is_empty() {
        return Err(ApiError::validation("Validation failed", field_errors));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /comms/chat-rooms
pub async fn create_chat_room(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateChatRoomBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    validate_create_room(&body)?;

    let room = repo::create_chat_room(&state.db, &user.id, &body).await?;

    Ok((StatusCode::CREATED, Json(room)))
}

/// GET /comms/chat-rooms
pub async fn list_chat_rooms(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListChatRoomsQuery>,
) -> Result<Json<PaginatedResult<serde_json::Value>>, ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(20).min(100);

    let (data, total_count) =
        repo::list_chat_rooms(&state.db, &user.id, offset, limit, &query).await?;
    let count = data.len() as u64;

    Ok(Json(PaginatedResult {
        data,
        pagination: PaginationMeta::new(offset, limit, count, total_count),
    }))
}

/// GET /comms/chat-rooms/:room
pub async fn get_chat_room(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(room_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    let room = repo::get_chat_room(&state.db, &room_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Chat room not found".to_string(),
            resource_type: Some("ChatRoom".to_string()),
            resource: Some(room_id.clone()),
        })?;

    // Ensure the caller is a participant
    let is_participant = room
        .get("participants")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().any(|p| p.as_str() == Some(&user.id)))
        .unwrap_or(false);

    if !is_participant && !user.has_any_role(&["admin"]) {
        return Err(ApiError::forbidden("Access denied"));
    }

    Ok(Json(room))
}

/// GET /comms/chat-rooms/:room/messages
pub async fn get_chat_messages(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(room_id): Path<String>,
    Query(query): Query<ListMessagesQuery>,
) -> Result<Json<PaginatedResult<serde_json::Value>>, ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    // Ensure room exists and user is a participant
    let room = repo::get_chat_room(&state.db, &room_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Chat room not found".to_string(),
            resource_type: Some("ChatRoom".to_string()),
            resource: Some(room_id.clone()),
        })?;

    let is_participant = room
        .get("participants")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().any(|p| p.as_str() == Some(&user.id)))
        .unwrap_or(false);

    if !is_participant && !user.has_any_role(&["admin"]) {
        return Err(ApiError::forbidden("Access denied"));
    }

    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(50).min(200);

    let (data, total_count) = repo::get_chat_messages(&state.db, &room_id, offset, limit).await?;
    let count = data.len() as u64;

    Ok(Json(PaginatedResult {
        data,
        pagination: PaginationMeta::new(offset, limit, count, total_count),
    }))
}

/// POST /comms/chat-rooms/:room/messages
pub async fn send_chat_message(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(room_id): Path<String>,
    Json(body): Json<SendChatMessageBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    validate_send_message(&body)?;

    // Ensure room exists and user is a participant
    let room = repo::get_chat_room(&state.db, &room_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Chat room not found".to_string(),
            resource_type: Some("ChatRoom".to_string()),
            resource: Some(room_id.clone()),
        })?;

    let is_participant = room
        .get("participants")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().any(|p| p.as_str() == Some(&user.id)))
        .unwrap_or(false);

    if !is_participant {
        return Err(ApiError::forbidden("You are not a participant in this chat room"));
    }

    let message =
        repo::send_chat_message(&state.db, &room_id, &user.id, &user.id, &body).await?;

    Ok((StatusCode::CREATED, Json(message)))
}

/// POST /comms/chat-rooms/:room/video-call/join
pub async fn join_video_call(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(room_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    // Ensure room exists and user is a participant
    let room = repo::get_chat_room(&state.db, &room_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Chat room not found".to_string(),
            resource_type: Some("ChatRoom".to_string()),
            resource: Some(room_id.clone()),
        })?;

    let is_participant = room
        .get("participants")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().any(|p| p.as_str() == Some(&user.id)))
        .unwrap_or(false);

    if !is_participant {
        return Err(ApiError::forbidden("You are not a participant in this chat room"));
    }

    // Get or create the active video call message
    let existing_msg = repo::get_active_video_call_message(&state.db, &room_id).await?;

    let call_message = match existing_msg {
        Some(msg) => {
            // Add user to participants list in video_call_data
            let mut call_data = msg
                .get("video_call_data")
                .cloned()
                .unwrap_or(serde_json::json!({}));

            let participants = call_data
                .get("participants")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            let already_in = participants.iter().any(|p| p.as_str() == Some(&user.id));
            if !already_in {
                let mut updated_participants = participants;
                updated_participants.push(serde_json::json!(user.id));
                call_data["participants"] = serde_json::json!(updated_participants);
            }

            let msg_id = msg
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            repo::update_video_call_message(&state.db, &msg_id, &user.id, call_data).await?
        }
        None => {
            // Start a new video call
            repo::start_video_call(&state.db, &room_id, &user.id, &user.id).await?
        }
    };

    Ok(Json(call_message))
}

/// POST /comms/chat-rooms/:room/video-call/leave
pub async fn leave_video_call(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(room_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    let msg = repo::get_active_video_call_message(&state.db, &room_id)
        .await?
        .ok_or_else(|| ApiError::not_found("No active video call in this room"))?;

    let mut call_data = msg
        .get("video_call_data")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    // Remove user from participants
    let participants: Vec<serde_json::Value> = call_data
        .get("participants")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|p| p.as_str() != Some(&user.id))
        .collect();

    call_data["participants"] = serde_json::json!(participants);

    let msg_id = msg
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let updated =
        repo::update_video_call_message(&state.db, &msg_id, &user.id, call_data).await?;

    Ok(Json(updated))
}

/// POST /comms/chat-rooms/:room/video-call/end
pub async fn end_video_call(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(room_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    let msg = repo::get_active_video_call_message(&state.db, &room_id)
        .await?
        .ok_or_else(|| ApiError::not_found("No active video call in this room"))?;

    let mut call_data = msg
        .get("video_call_data")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    call_data["status"] = serde_json::json!("ended");
    call_data["ended_at"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
    call_data["participants"] = serde_json::json!([]);

    let msg_id = msg
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let updated =
        repo::update_video_call_message(&state.db, &msg_id, &user.id, call_data).await?;

    // Clear the active call reference from the room
    repo::clear_active_video_call(&state.db, &room_id).await?;

    Ok(Json(updated))
}

/// PATCH /comms/chat-rooms/:room/video-call/participant
pub async fn update_video_call_participant(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(room_id): Path<String>,
    Json(body): Json<UpdateVideoCallParticipantBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    if body.action != "add" && body.action != "remove" {
        return Err(ApiError::validation(
            "Validation failed",
            vec![FieldError {
                field: "action".to_string(),
                code: "INVALID".to_string(),
                message: "action must be 'add' or 'remove'".to_string(),
                value: Some(serde_json::json!(body.action)),
            }],
        ));
    }

    let msg = repo::get_active_video_call_message(&state.db, &room_id)
        .await?
        .ok_or_else(|| ApiError::not_found("No active video call in this room"))?;

    let mut call_data = msg
        .get("video_call_data")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    let mut participants: Vec<serde_json::Value> = call_data
        .get("participants")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    if body.action == "add" {
        let already_in = participants
            .iter()
            .any(|p| p.as_str() == Some(&body.participant_id));
        if !already_in {
            participants.push(serde_json::json!(body.participant_id));
        }
    } else {
        participants.retain(|p| p.as_str() != Some(&body.participant_id));
    }

    call_data["participants"] = serde_json::json!(participants);

    if let Some(meta) = body.metadata {
        call_data["metadata"] = meta;
    }

    let msg_id = msg
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let updated =
        repo::update_video_call_message(&state.db, &msg_id, &user.id, call_data).await?;

    Ok(Json(updated))
}

/// GET /comms/ice-servers
pub async fn get_ice_servers(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    let servers: Vec<serde_json::Value> = state
        .config
        .webrtc_ice_servers
        .iter()
        .map(|url| serde_json::json!({ "urls": url }))
        .collect();

    Ok(Json(serde_json::json!({ "iceServers": servers })))
}
