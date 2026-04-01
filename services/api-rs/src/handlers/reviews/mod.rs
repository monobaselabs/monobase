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
pub struct CreateReviewBody {
    /// Identifies the entity being reviewed (e.g. booking ID).
    pub context_id: String,
    /// The person submitting the review.
    pub reviewer_id: String,
    /// Discriminator for the kind of review (e.g. "booking", "provider").
    pub review_type: String,
    /// Optional FK to the person entity that is the subject of the review.
    pub reviewed_entity_id: Option<String>,
    /// Net Promoter Score — must be in the range [0, 10].
    pub nps_score: u8,
    /// Optional free-text comment, max 1 000 characters.
    pub comment: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListReviewsQuery {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    pub reviewer_id: Option<String>,
    pub review_type: Option<String>,
    pub context_id: Option<String>,
    pub reviewed_entity_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

fn validate_create_body(body: &CreateReviewBody) -> Result<(), ApiError> {
    let mut field_errors: Vec<FieldError> = Vec::new();

    if body.nps_score > 10 {
        field_errors.push(FieldError {
            field: "npsScore".to_string(),
            code: "RANGE".to_string(),
            message: "npsScore must be between 0 and 10".to_string(),
            value: Some(serde_json::json!(body.nps_score)),
        });
    }

    if let Some(ref comment) = body.comment {
        if comment.len() > 1000 {
            field_errors.push(FieldError {
                field: "comment".to_string(),
                code: "MAX_LENGTH".to_string(),
                message: "comment must not exceed 1000 characters".to_string(),
                value: None,
            });
        }
    }

    if body.context_id.is_empty() {
        field_errors.push(FieldError {
            field: "contextId".to_string(),
            code: "REQUIRED".to_string(),
            message: "contextId is required".to_string(),
            value: None,
        });
    }

    if body.reviewer_id.is_empty() {
        field_errors.push(FieldError {
            field: "reviewerId".to_string(),
            code: "REQUIRED".to_string(),
            message: "reviewerId is required".to_string(),
            value: None,
        });
    }

    if body.review_type.is_empty() {
        field_errors.push(FieldError {
            field: "reviewType".to_string(),
            code: "REQUIRED".to_string(),
            message: "reviewType is required".to_string(),
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

/// POST /reviews
pub async fn create_review(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateReviewBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let user = auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    validate_create_body(&body)?;

    let review = repo::create_review(&state.db, &user.id, &body).await?;

    Ok((StatusCode::CREATED, Json(review)))
}

/// GET /reviews
pub async fn list_reviews(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListReviewsQuery>,
) -> Result<Json<PaginatedResult<serde_json::Value>>, ApiError> {
    auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(20).min(100);

    let (data, total_count) = repo::list_reviews(&state.db, offset, limit, &query).await?;
    let count = data.len() as u64;

    Ok(Json(PaginatedResult {
        data,
        pagination: PaginationMeta::new(offset, limit, count, total_count),
    }))
}

/// GET /reviews/:review
pub async fn get_review(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(review_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    let review = repo::get_review(&state.db, &review_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Review not found".to_string(),
            resource_type: Some("Review".to_string()),
            resource: Some(review_id.clone()),
        })?;

    Ok(Json(review))
}

/// DELETE /reviews/:review
///
/// Only the owner (the user identified by `reviewer_id`) or an admin may
/// delete a review.
pub async fn delete_review(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(review_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user", "admin"])
            .await?;

    let review = repo::get_review(&state.db, &review_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Review not found".to_string(),
            resource_type: Some("Review".to_string()),
            resource: Some(review_id.clone()),
        })?;

    // Enforce owner-or-admin rule
    if !user.has_any_role(&["admin"]) {
        let reviewer_id = review
            .get("reviewer_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if reviewer_id != user.id {
            return Err(ApiError::forbidden(
                "Only the review owner or an admin may delete this review",
            ));
        }
    }

    repo::delete_review(&state.db, &review_id).await?;

    Ok(StatusCode::NO_CONTENT)
}
