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
pub struct CreatePatientBody {
    pub person_id: String,
    pub primary_provider: Option<serde_json::Value>,
    pub primary_pharmacy: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePatientBody {
    pub primary_provider: Option<serde_json::Value>,
    pub primary_pharmacy: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPatientsQuery {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    pub q: Option<String>,
}

/// POST /patients
pub async fn create_patient(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreatePatientBody>,
) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), ApiError> {
    let user = auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    let patient = repo::create_patient(&state.db, &user.id, &body).await?;

    Ok((axum::http::StatusCode::CREATED, Json(patient)))
}

/// GET /patients
pub async fn list_patients(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListPatientsQuery>,
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
        repo::list_patients(&state.db, offset, limit, query.q.as_deref()).await?;
    let count = data.len() as u64;

    Ok(Json(PaginatedResult {
        data,
        pagination: PaginationMeta::new(offset, limit, count, total_count),
    }))
}

/// GET /patients/:patient
pub async fn get_patient(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(patient_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = auth_mw::require_roles(
        &state.db,
        &headers,
        &state.config.auth_secret,
        &["admin", "support", "user"],
    )
    .await?;

    let patient = repo::get_patient(&state.db, &patient_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Patient not found".to_string(),
            resource_type: Some("Patient".to_string()),
            resource: Some(patient_id.clone()),
        })?;

    // Owner check for non-admin/support users
    if !user.has_any_role(&["admin", "support"]) {
        let created_by = patient
            .get("createdBy")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if created_by != user.id {
            return Err(ApiError::forbidden("Access denied"));
        }
    }

    Ok(Json(patient))
}

/// PATCH /patients/:patient
pub async fn update_patient(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(patient_id): Path<String>,
    Json(body): Json<UpdatePatientBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

    // Verify ownership
    let existing = repo::get_patient(&state.db, &patient_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Patient not found"))?;

    let created_by = existing
        .get("createdBy")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if created_by != user.id {
        return Err(ApiError::forbidden("Access denied"));
    }

    let updated = repo::update_patient(&state.db, &patient_id, &user.id, &body).await?;

    Ok(Json(updated))
}

/// DELETE /patients/:patient
pub async fn delete_patient(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(patient_id): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    let user = auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

    // Verify ownership
    let existing = repo::get_patient(&state.db, &patient_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Patient not found"))?;

    let created_by = existing
        .get("createdBy")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if created_by != user.id {
        return Err(ApiError::forbidden("Access denied"));
    }

    repo::delete_patient(&state.db, &patient_id).await?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}
