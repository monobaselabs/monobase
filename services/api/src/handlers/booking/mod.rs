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
// Request / query types — Booking Events
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBookingEventBody {
    pub owner_id: String,
    pub context_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub timezone: Option<String>,
    pub location_types: Option<serde_json::Value>,
    pub max_booking_days: Option<u32>,
    pub min_booking_minutes: Option<u32>,
    pub form_config: Option<serde_json::Value>,
    pub billing_config: Option<serde_json::Value>,
    pub status: Option<String>,
    pub effective_from: Option<String>,
    pub effective_to: Option<String>,
    pub daily_configs: serde_json::Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBookingEventBody {
    pub context_id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub timezone: Option<String>,
    pub location_types: Option<serde_json::Value>,
    pub max_booking_days: Option<u32>,
    pub min_booking_minutes: Option<u32>,
    pub form_config: Option<serde_json::Value>,
    pub billing_config: Option<serde_json::Value>,
    pub status: Option<String>,
    pub effective_from: Option<String>,
    pub effective_to: Option<String>,
    pub daily_configs: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListBookingEventsQuery {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    pub owner_id: Option<String>,
    pub status: Option<String>,
}

// ---------------------------------------------------------------------------
// Request / query types — Bookings
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBookingBody {
    pub client_id: String,
    pub provider_id: String,
    pub slot_id: String,
    pub location_type: String,
    pub reason: Option<String>,
    pub duration_minutes: u32,
    pub form_responses: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListBookingsQuery {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    pub client_id: Option<String>,
    pub provider_id: Option<String>,
    pub status: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelBookingBody {
    pub reason: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectBookingBody {
    pub reason: Option<String>,
}

// ---------------------------------------------------------------------------
// Request / query types — Schedule Exceptions
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateScheduleExceptionBody {
    pub owner_id: String,
    pub context_id: Option<String>,
    pub timezone: Option<String>,
    pub start_datetime: String,
    pub end_datetime: String,
    pub reason: String,
    pub recurring: Option<bool>,
    pub recurrence_pattern: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListScheduleExceptionsQuery {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    pub recurring: Option<bool>,
}

// ---------------------------------------------------------------------------
// Request / query types — Slots
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSlotsQuery {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    pub status: Option<String>,
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

fn validate_create_booking(body: &CreateBookingBody) -> Result<(), ApiError> {
    let mut field_errors: Vec<FieldError> = Vec::new();

    if body.duration_minutes < 15 || body.duration_minutes > 480 {
        field_errors.push(FieldError {
            field: "durationMinutes".to_string(),
            code: "RANGE".to_string(),
            message: "durationMinutes must be between 15 and 480".to_string(),
            value: Some(serde_json::json!(body.duration_minutes)),
        });
    }

    if let Some(ref reason) = body.reason {
        if reason.len() > 500 {
            field_errors.push(FieldError {
                field: "reason".to_string(),
                code: "MAX_LENGTH".to_string(),
                message: "reason must not exceed 500 characters".to_string(),
                value: None,
            });
        }
    }

    let valid_location_types = ["video", "phone", "in-person"];
    if !valid_location_types.contains(&body.location_type.as_str()) {
        field_errors.push(FieldError {
            field: "locationType".to_string(),
            code: "INVALID_VALUE".to_string(),
            message: "locationType must be one of: video, phone, in-person".to_string(),
            value: Some(serde_json::json!(body.location_type)),
        });
    }

    if body.slot_id.is_empty() {
        field_errors.push(FieldError {
            field: "slotId".to_string(),
            code: "REQUIRED".to_string(),
            message: "slotId is required".to_string(),
            value: None,
        });
    }

    if !field_errors.is_empty() {
        return Err(ApiError::validation("Validation failed", field_errors));
    }

    Ok(())
}

fn validate_create_booking_event(body: &CreateBookingEventBody) -> Result<(), ApiError> {
    let mut field_errors: Vec<FieldError> = Vec::new();

    if body.title.is_empty() {
        field_errors.push(FieldError {
            field: "title".to_string(),
            code: "REQUIRED".to_string(),
            message: "title is required".to_string(),
            value: None,
        });
    }

    if body.owner_id.is_empty() {
        field_errors.push(FieldError {
            field: "ownerId".to_string(),
            code: "REQUIRED".to_string(),
            message: "ownerId is required".to_string(),
            value: None,
        });
    }

    if let Some(days) = body.max_booking_days {
        if days > 365 {
            field_errors.push(FieldError {
                field: "maxBookingDays".to_string(),
                code: "RANGE".to_string(),
                message: "maxBookingDays must be between 0 and 365".to_string(),
                value: Some(serde_json::json!(days)),
            });
        }
    }

    if let Some(minutes) = body.min_booking_minutes {
        if minutes > 4320 {
            field_errors.push(FieldError {
                field: "minBookingMinutes".to_string(),
                code: "RANGE".to_string(),
                message: "minBookingMinutes must be between 0 and 4320".to_string(),
                value: Some(serde_json::json!(minutes)),
            });
        }
    }

    if !field_errors.is_empty() {
        return Err(ApiError::validation("Validation failed", field_errors));
    }

    Ok(())
}

fn validate_create_schedule_exception(body: &CreateScheduleExceptionBody) -> Result<(), ApiError> {
    let mut field_errors: Vec<FieldError> = Vec::new();

    if body.reason.is_empty() {
        field_errors.push(FieldError {
            field: "reason".to_string(),
            code: "REQUIRED".to_string(),
            message: "reason is required".to_string(),
            value: None,
        });
    }

    if body.reason.len() > 500 {
        field_errors.push(FieldError {
            field: "reason".to_string(),
            code: "MAX_LENGTH".to_string(),
            message: "reason must not exceed 500 characters".to_string(),
            value: None,
        });
    }

    if body.start_datetime.is_empty() {
        field_errors.push(FieldError {
            field: "startDatetime".to_string(),
            code: "REQUIRED".to_string(),
            message: "startDatetime is required".to_string(),
            value: None,
        });
    }

    if body.end_datetime.is_empty() {
        field_errors.push(FieldError {
            field: "endDatetime".to_string(),
            code: "REQUIRED".to_string(),
            message: "endDatetime is required".to_string(),
            value: None,
        });
    }

    if !field_errors.is_empty() {
        return Err(ApiError::validation("Validation failed", field_errors));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Booking Event handlers
// ---------------------------------------------------------------------------

/// POST /booking/events
pub async fn create_booking_event(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateBookingEventBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    validate_create_booking_event(&body)?;

    let event = repo::create_booking_event(&state.db, &user.id, &body).await?;

    Ok((StatusCode::CREATED, Json(event)))
}

/// GET /booking/events
pub async fn list_booking_events(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListBookingEventsQuery>,
) -> Result<Json<PaginatedResult<serde_json::Value>>, ApiError> {
    // Optional auth — public endpoint, but auth enriches access when present
    let _user = auth_mw::extract_auth(&state.db, &headers, &state.config.auth_secret).await?;

    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(20).min(100);

    let (data, total_count) =
        repo::list_booking_events(&state.db, offset, limit, &query).await?;
    let count = data.len() as u64;

    Ok(Json(PaginatedResult {
        data,
        pagination: PaginationMeta::new(offset, limit, count, total_count),
    }))
}

/// GET /booking/events/:event
pub async fn get_booking_event(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(event_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Optional auth
    let _user = auth_mw::extract_auth(&state.db, &headers, &state.config.auth_secret).await?;

    let event = repo::get_booking_event(&state.db, &event_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Booking event not found".to_string(),
            resource_type: Some("BookingEvent".to_string()),
            resource: Some(event_id.clone()),
        })?;

    Ok(Json(event))
}

/// PATCH /booking/events/:event
pub async fn update_booking_event(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(event_id): Path<String>,
    Json(body): Json<UpdateBookingEventBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    let event = repo::get_booking_event(&state.db, &event_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Booking event not found".to_string(),
            resource_type: Some("BookingEvent".to_string()),
            resource: Some(event_id.clone()),
        })?;

    // Enforce owner-or-admin
    if !user.has_any_role(&["admin"]) {
        let owner_id = event
            .get("owner_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if owner_id != user.id {
            return Err(ApiError::forbidden(
                "Only the event owner or an admin may update this event",
            ));
        }
    }

    let updated = repo::update_booking_event(&state.db, &event_id, &user.id, &body).await?;

    Ok(Json(updated))
}

/// DELETE /booking/events/:event
pub async fn delete_booking_event(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(event_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    let event = repo::get_booking_event(&state.db, &event_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Booking event not found".to_string(),
            resource_type: Some("BookingEvent".to_string()),
            resource: Some(event_id.clone()),
        })?;

    if !user.has_any_role(&["admin"]) {
        let owner_id = event
            .get("owner_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if owner_id != user.id {
            return Err(ApiError::forbidden(
                "Only the event owner or an admin may delete this event",
            ));
        }
    }

    repo::delete_booking_event(&state.db, &event_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Booking handlers
// ---------------------------------------------------------------------------

/// POST /booking/bookings
pub async fn create_booking(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateBookingBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    validate_create_booking(&body)?;

    let booking = repo::create_booking(&state.db, &user.id, &body).await?;

    Ok((StatusCode::CREATED, Json(booking)))
}

/// GET /booking/bookings
pub async fn list_bookings(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListBookingsQuery>,
) -> Result<Json<PaginatedResult<serde_json::Value>>, ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(20).min(100);

    // Non-admin/support users can only see their own bookings
    let effective_query = if user.has_any_role(&["admin", "support"]) {
        query
    } else {
        // Restrict to the authenticated user's own bookings as client or provider
        // We keep the query as-is but the caller is expected to supply client_id or provider_id.
        // If neither is provided and the user is not admin/support, scope to user's own records
        // by injecting the user ID as client_id filter when no ownership filter is present.
        let no_owner_filter = query.client_id.is_none() && query.provider_id.is_none();
        if no_owner_filter {
            ListBookingsQuery {
                client_id: Some(user.id.clone()),
                ..query
            }
        } else {
            // Validate that the provided filters are for the authenticated user
            let client_ok = query
                .client_id
                .as_deref()
                .map(|id| id == user.id)
                .unwrap_or(true);
            let provider_ok = query
                .provider_id
                .as_deref()
                .map(|id| id == user.id)
                .unwrap_or(true);

            if !client_ok && !provider_ok {
                return Err(ApiError::forbidden(
                    "You may only list your own bookings",
                ));
            }
            query
        }
    };

    let (data, total_count) =
        repo::list_bookings(&state.db, offset, limit, &effective_query).await?;
    let count = data.len() as u64;

    Ok(Json(PaginatedResult {
        data,
        pagination: PaginationMeta::new(offset, limit, count, total_count),
    }))
}

/// GET /booking/bookings/:booking
pub async fn get_booking(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(booking_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    let booking = repo::get_booking(&state.db, &booking_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Booking not found".to_string(),
            resource_type: Some("Booking".to_string()),
            resource: Some(booking_id.clone()),
        })?;

    // Enforce ownership for non-admin/support users
    if !user.has_any_role(&["admin", "support"]) {
        let client_id = booking
            .get("client_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let provider_id = booking
            .get("provider_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if client_id != user.id && provider_id != user.id {
            return Err(ApiError::forbidden("Access denied"));
        }
    }

    Ok(Json(booking))
}

/// POST /booking/bookings/:booking/cancel
pub async fn cancel_booking(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(booking_id): Path<String>,
    Json(body): Json<CancelBookingBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    let booking = repo::get_booking(&state.db, &booking_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Booking not found".to_string(),
            resource_type: Some("Booking".to_string()),
            resource: Some(booking_id.clone()),
        })?;

    if !user.has_any_role(&["admin"]) {
        let client_id = booking
            .get("client_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let provider_id = booking
            .get("provider_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if client_id != user.id && provider_id != user.id {
            return Err(ApiError::forbidden("Access denied"));
        }
    }

    let updated =
        repo::cancel_booking(&state.db, &booking_id, &user.id, body.reason.as_deref()).await?;

    Ok(Json(updated))
}

/// POST /booking/bookings/:booking/confirm
pub async fn confirm_booking(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(booking_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    let booking = repo::get_booking(&state.db, &booking_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Booking not found".to_string(),
            resource_type: Some("Booking".to_string()),
            resource: Some(booking_id.clone()),
        })?;

    // Only the provider of the booking or an admin may confirm
    if !user.has_any_role(&["admin"]) {
        let provider_id = booking
            .get("provider_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if provider_id != user.id {
            return Err(ApiError::forbidden(
                "Only the booking provider or an admin may confirm this booking",
            ));
        }
    }

    let updated = repo::confirm_booking(&state.db, &booking_id, &user.id).await?;

    Ok(Json(updated))
}

/// POST /booking/bookings/:booking/no-show
pub async fn mark_no_show(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(booking_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    let updated = repo::mark_no_show(&state.db, &booking_id, &user.id).await?;

    Ok(Json(updated))
}

/// POST /booking/bookings/:booking/reject
pub async fn reject_booking(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(booking_id): Path<String>,
    Json(body): Json<RejectBookingBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    let booking = repo::get_booking(&state.db, &booking_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Booking not found".to_string(),
            resource_type: Some("Booking".to_string()),
            resource: Some(booking_id.clone()),
        })?;

    // Only the provider or an admin may reject
    if !user.has_any_role(&["admin"]) {
        let provider_id = booking
            .get("provider_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if provider_id != user.id {
            return Err(ApiError::forbidden(
                "Only the booking provider or an admin may reject this booking",
            ));
        }
    }

    let updated =
        repo::reject_booking(&state.db, &booking_id, &user.id, body.reason.as_deref()).await?;

    Ok(Json(updated))
}

// ---------------------------------------------------------------------------
// Schedule Exception handlers
// ---------------------------------------------------------------------------

/// POST /booking/events/:event/exceptions
pub async fn create_schedule_exception(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(event_id): Path<String>,
    Json(body): Json<CreateScheduleExceptionBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    let event = repo::get_booking_event(&state.db, &event_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Booking event not found".to_string(),
            resource_type: Some("BookingEvent".to_string()),
            resource: Some(event_id.clone()),
        })?;

    if !user.has_any_role(&["admin"]) {
        let owner_id = event
            .get("owner_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if owner_id != user.id {
            return Err(ApiError::forbidden(
                "Only the event owner or an admin may add exceptions to this event",
            ));
        }
    }

    validate_create_schedule_exception(&body)?;

    let exception =
        repo::create_schedule_exception(&state.db, &event_id, &user.id, &body).await?;

    Ok((StatusCode::CREATED, Json(exception)))
}

/// GET /booking/events/:event/exceptions
pub async fn list_schedule_exceptions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(event_id): Path<String>,
    Query(query): Query<ListScheduleExceptionsQuery>,
) -> Result<Json<PaginatedResult<serde_json::Value>>, ApiError> {
    auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(20).min(100);

    let (data, total_count) =
        repo::list_schedule_exceptions(&state.db, &event_id, offset, limit, &query).await?;
    let count = data.len() as u64;

    Ok(Json(PaginatedResult {
        data,
        pagination: PaginationMeta::new(offset, limit, count, total_count),
    }))
}

/// GET /booking/events/:event/exceptions/:exception
pub async fn get_schedule_exception(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((event_id, exception_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    let exception = repo::get_schedule_exception(&state.db, &event_id, &exception_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Schedule exception not found".to_string(),
            resource_type: Some("ScheduleException".to_string()),
            resource: Some(exception_id.clone()),
        })?;

    Ok(Json(exception))
}

/// DELETE /booking/events/:event/exceptions/:exception
pub async fn delete_schedule_exception(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((event_id, exception_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    let exception = repo::get_schedule_exception(&state.db, &event_id, &exception_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Schedule exception not found".to_string(),
            resource_type: Some("ScheduleException".to_string()),
            resource: Some(exception_id.clone()),
        })?;

    if !user.has_any_role(&["admin"]) {
        let owner_id = exception
            .get("owner_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if owner_id != user.id {
            return Err(ApiError::forbidden(
                "Only the exception owner or an admin may delete this exception",
            ));
        }
    }

    repo::delete_schedule_exception(&state.db, &event_id, &exception_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Slot handlers
// ---------------------------------------------------------------------------

/// GET /booking/events/:event/slots
pub async fn list_event_slots(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(event_id): Path<String>,
    Query(query): Query<ListSlotsQuery>,
) -> Result<Json<PaginatedResult<serde_json::Value>>, ApiError> {
    // Optional auth
    let _user = auth_mw::extract_auth(&state.db, &headers, &state.config.auth_secret).await?;

    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(50).min(200);

    let (data, total_count) =
        repo::list_event_slots(&state.db, &event_id, offset, limit, &query).await?;
    let count = data.len() as u64;

    Ok(Json(PaginatedResult {
        data,
        pagination: PaginationMeta::new(offset, limit, count, total_count),
    }))
}

/// GET /booking/slots/:slotId
pub async fn get_time_slot(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(slot_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

    let slot = repo::get_time_slot(&state.db, &slot_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Time slot not found".to_string(),
            resource_type: Some("TimeSlot".to_string()),
            resource: Some(slot_id.clone()),
        })?;

    Ok(Json(slot))
}
