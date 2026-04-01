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
pub struct CreateProviderBody {
    pub person_id: String,
    pub provider_type: String,
    pub years_of_experience: Option<i32>,
    pub biography: Option<String>,
    pub minor_ailments_specialties: Option<Vec<String>>,
    pub minor_ailments_practice_locations: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProviderBody {
    pub provider_type: Option<String>,
    pub years_of_experience: Option<i32>,
    pub biography: Option<String>,
    pub minor_ailments_specialties: Option<Vec<String>>,
    pub minor_ailments_practice_locations: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListProvidersQuery {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    pub q: Option<String>,
}

/// POST /providers
pub async fn create_provider(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateProviderBody>,
) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), ApiError> {
    let user = auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    let provider = repo::create_provider(&state.db, &user.id, &body).await?;

    Ok((axum::http::StatusCode::CREATED, Json(provider)))
}

/// GET /providers
pub async fn list_providers(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListProvidersQuery>,
) -> Result<Json<PaginatedResult<serde_json::Value>>, ApiError> {
    auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(20).min(100);

    let (data, total_count) =
        repo::list_providers(&state.db, offset, limit, query.q.as_deref()).await?;
    let count = data.len() as u64;

    Ok(Json(PaginatedResult {
        data,
        pagination: PaginationMeta::new(offset, limit, count, total_count),
    }))
}

/// GET /providers/:provider
pub async fn get_provider(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

    let provider = repo::get_provider(&state.db, &provider_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Provider not found".to_string(),
            resource_type: Some("Provider".to_string()),
            resource: Some(provider_id.clone()),
        })?;

    Ok(Json(provider))
}

/// PATCH /providers/:provider
pub async fn update_provider(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
    Json(body): Json<UpdateProviderBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

    // Verify ownership
    let existing = repo::get_provider(&state.db, &provider_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Provider not found"))?;

    let created_by = existing
        .get("createdBy")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if created_by != user.id {
        return Err(ApiError::forbidden("Access denied"));
    }

    let updated = repo::update_provider(&state.db, &provider_id, &user.id, &body).await?;

    Ok(Json(updated))
}

/// DELETE /providers/:provider
pub async fn delete_provider(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    let user = auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

    // Verify ownership
    let existing = repo::get_provider(&state.db, &provider_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Provider not found"))?;

    let created_by = existing
        .get("createdBy")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if created_by != user.id {
        return Err(ApiError::forbidden("Access denied"));
    }

    repo::delete_provider(&state.db, &provider_id).await?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}
