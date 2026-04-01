mod repo;

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use serde::Deserialize;

use crate::error::ApiError;
use crate::handlers::AppState;
use crate::middleware::auth as auth_mw;
use crate::model::{PaginatedResult, PaginationMeta};

// ---------------------------------------------------------------------------
// Request / query types
// ---------------------------------------------------------------------------

/// Used internally by the service layer to create notifications programmatically.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNotificationBody {
    pub recipient_id: String,
    pub r#type: String,
    pub channel: String,
    pub title: String,
    pub message: String,
    pub scheduled_at: Option<String>,
    pub related_entity_type: Option<String>,
    pub related_entity_id: Option<String>,
    pub consent_validated: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListNotificationsQuery {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    pub r#type: Option<String>,
    pub channel: Option<String>,
    pub status: Option<String>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /notifs
/// Auth: user (own notifications only) or admin (all)
pub async fn list_notifications(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListNotificationsQuery>,
) -> Result<Json<PaginatedResult<serde_json::Value>>, ApiError> {
    let user = auth_mw::require_roles(
        &state.db,
        &headers,
        &state.config.auth_secret,
        &["admin", "user"],
    )
    .await?;

    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(20).min(100);

    // Non-admin users are scoped to their own recipient_id.
    let recipient_scope = if user.has_any_role(&["admin"]) {
        None
    } else {
        Some(user.id.as_str())
    };

    let (data, total_count) =
        repo::list_notifications(&state.db, offset, limit, &query, recipient_scope).await?;
    let count = data.len() as u64;

    Ok(Json(PaginatedResult {
        data,
        pagination: PaginationMeta::new(offset, limit, count, total_count),
    }))
}

/// POST /notifs/read-all
/// Auth: user — marks all of the caller's unread notifications as read.
pub async fn mark_all_notifications_as_read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    let updated_count =
        repo::mark_all_notifications_as_read(&state.db, &user.id, &user.id).await?;

    Ok(Json(serde_json::json!({
        "updatedCount": updated_count,
    })))
}

/// GET /notifs/:notif
/// Auth: user (own) or admin (any)
pub async fn get_notification(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(notif_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = auth_mw::require_roles(
        &state.db,
        &headers,
        &state.config.auth_secret,
        &["admin", "user"],
    )
    .await?;

    let notification = repo::get_notification(&state.db, &notif_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Notification not found".to_string(),
            resource_type: Some("Notification".to_string()),
            resource: Some(notif_id.clone()),
        })?;

    // Non-admin users may only view their own notifications.
    if !user.has_any_role(&["admin"]) {
        let recipient_id = notification
            .get("recipient_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if recipient_id != user.id {
            return Err(ApiError::forbidden("Access denied"));
        }
    }

    Ok(Json(notification))
}

/// POST /notifs/:notif/read
/// Auth: user — marks a single notification as read for the calling user.
pub async fn mark_notification_as_read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(notif_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    // Verify the notification exists before attempting the update.
    let notification = repo::get_notification(&state.db, &notif_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Notification not found".to_string(),
            resource_type: Some("Notification".to_string()),
            resource: Some(notif_id.clone()),
        })?;

    let recipient_id = notification
        .get("recipient_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if recipient_id != user.id {
        return Err(ApiError::forbidden("Access denied"));
    }

    let updated =
        repo::mark_notification_as_read(&state.db, &notif_id, &user.id, &user.id).await?;

    Ok(Json(updated))
}
