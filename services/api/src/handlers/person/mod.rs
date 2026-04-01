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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePersonBody {
    pub first_name: String,
    pub last_name: Option<String>,
    pub middle_name: Option<String>,
    pub date_of_birth: Option<String>,
    pub gender: Option<String>,
    pub primary_address: Option<serde_json::Value>,
    pub contact_info: Option<serde_json::Value>,
    pub avatar: Option<serde_json::Value>,
    pub languages_spoken: Option<Vec<String>>,
    pub timezone: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePersonBody {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub middle_name: Option<String>,
    pub date_of_birth: Option<String>,
    pub gender: Option<String>,
    pub primary_address: Option<serde_json::Value>,
    pub contact_info: Option<serde_json::Value>,
    pub avatar: Option<serde_json::Value>,
    pub languages_spoken: Option<Vec<String>>,
    pub timezone: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPersonsQuery {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    pub q: Option<String>,
}

/// POST /persons
pub async fn create_person(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreatePersonBody>,
) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), ApiError> {
    let user = auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    let person = repo::create_person(&state.db, &user.id, &body).await?;

    Ok((axum::http::StatusCode::CREATED, Json(person)))
}

/// GET /persons
pub async fn list_persons(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListPersonsQuery>,
) -> Result<Json<PaginatedResult<serde_json::Value>>, ApiError> {
    auth_mw::require_roles(
        &state.db,
        &headers,
        &state.config.auth_secret,
        &["admin", "support"],
    )
    .await?;

    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(20).min(100);

    let (data, total_count) =
        repo::list_persons(&state.db, offset, limit, query.q.as_deref()).await?;
    let count = data.len() as u64;

    Ok(Json(PaginatedResult {
        data,
        pagination: PaginationMeta::new(offset, limit, count, total_count),
    }))
}

/// GET /persons/:person
pub async fn get_person(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(person_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = auth_mw::require_roles(
        &state.db,
        &headers,
        &state.config.auth_secret,
        &["admin", "support", "user"],
    )
    .await?;

    let person = repo::get_person(&state.db, &person_id).await?.ok_or_else(|| {
        ApiError::NotFound {
            message: "Person not found".to_string(),
            resource_type: Some("Person".to_string()),
            resource: Some(person_id.clone()),
        }
    })?;

    // Owner check for non-admin users
    if !user.has_any_role(&["admin", "support"]) {
        let person_created_by = person
            .get("createdBy")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if person_created_by != user.id {
            return Err(ApiError::forbidden("Access denied"));
        }
    }

    Ok(Json(person))
}

/// PATCH /persons/:person
pub async fn update_person(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(person_id): Path<String>,
    Json(body): Json<UpdatePersonBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

    // Verify ownership
    let existing = repo::get_person(&state.db, &person_id).await?.ok_or_else(|| {
        ApiError::not_found("Person not found")
    })?;

    let created_by = existing
        .get("createdBy")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if created_by != user.id && !user.has_any_role(&["admin"]) {
        return Err(ApiError::forbidden("Access denied"));
    }

    let updated = repo::update_person(&state.db, &person_id, &user.id, &body).await?;

    Ok(Json(updated))
}
