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
pub struct CreateConsultationBody {
    /// Patient record ID.
    pub patient_id: String,
    /// Provider record ID.
    pub provider_id: String,
    /// Optional unique external context identifier (e.g. booking ID).
    pub context: Option<String>,
    /// Chief complaint — max 500 characters.
    pub chief_complaint: Option<String>,
    /// Clinical assessment — max 2000 characters.
    pub assessment: Option<String>,
    /// Treatment plan — max 2000 characters.
    pub plan: Option<String>,
    /// Vitals as free-form JSON.
    pub vitals: Option<serde_json::Value>,
    /// Symptoms list as JSON.
    pub symptoms: Option<serde_json::Value>,
    /// Prescriptions as JSON.
    pub prescriptions: Option<serde_json::Value>,
    /// Follow-up instructions as JSON.
    pub follow_up: Option<serde_json::Value>,
    /// External documentation references as JSON.
    pub external_documentation: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConsultationBody {
    pub chief_complaint: Option<String>,
    pub assessment: Option<String>,
    pub plan: Option<String>,
    pub vitals: Option<serde_json::Value>,
    pub symptoms: Option<serde_json::Value>,
    pub prescriptions: Option<serde_json::Value>,
    pub follow_up: Option<serde_json::Value>,
    pub external_documentation: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListConsultationsQuery {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    /// Filter by patient ID.
    pub patient_id: Option<String>,
    /// Filter by provider ID.
    pub provider_id: Option<String>,
    /// Filter by status: "draft", "finalized", or "amended".
    pub status: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListEMRPatientsQuery {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    pub q: Option<String>,
    /// Optionally scope to a specific provider.
    pub provider_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

fn validate_create_consultation(body: &CreateConsultationBody) -> Result<(), ApiError> {
    let mut field_errors: Vec<FieldError> = Vec::new();

    if body.patient_id.is_empty() {
        field_errors.push(FieldError {
            field: "patientId".to_string(),
            code: "REQUIRED".to_string(),
            message: "patientId is required".to_string(),
            value: None,
        });
    }

    if body.provider_id.is_empty() {
        field_errors.push(FieldError {
            field: "providerId".to_string(),
            code: "REQUIRED".to_string(),
            message: "providerId is required".to_string(),
            value: None,
        });
    }

    if let Some(ref cc) = body.chief_complaint {
        if cc.len() > 500 {
            field_errors.push(FieldError {
                field: "chiefComplaint".to_string(),
                code: "MAX_LENGTH".to_string(),
                message: "chiefComplaint must not exceed 500 characters".to_string(),
                value: None,
            });
        }
    }

    if let Some(ref assessment) = body.assessment {
        if assessment.len() > 2000 {
            field_errors.push(FieldError {
                field: "assessment".to_string(),
                code: "MAX_LENGTH".to_string(),
                message: "assessment must not exceed 2000 characters".to_string(),
                value: None,
            });
        }
    }

    if let Some(ref plan) = body.plan {
        if plan.len() > 2000 {
            field_errors.push(FieldError {
                field: "plan".to_string(),
                code: "MAX_LENGTH".to_string(),
                message: "plan must not exceed 2000 characters".to_string(),
                value: None,
            });
        }
    }

    if !field_errors.is_empty() {
        return Err(ApiError::validation("Validation failed", field_errors));
    }

    Ok(())
}

fn validate_update_consultation(body: &UpdateConsultationBody) -> Result<(), ApiError> {
    let mut field_errors: Vec<FieldError> = Vec::new();

    if let Some(ref cc) = body.chief_complaint {
        if cc.len() > 500 {
            field_errors.push(FieldError {
                field: "chiefComplaint".to_string(),
                code: "MAX_LENGTH".to_string(),
                message: "chiefComplaint must not exceed 500 characters".to_string(),
                value: None,
            });
        }
    }

    if let Some(ref assessment) = body.assessment {
        if assessment.len() > 2000 {
            field_errors.push(FieldError {
                field: "assessment".to_string(),
                code: "MAX_LENGTH".to_string(),
                message: "assessment must not exceed 2000 characters".to_string(),
                value: None,
            });
        }
    }

    if let Some(ref plan) = body.plan {
        if plan.len() > 2000 {
            field_errors.push(FieldError {
                field: "plan".to_string(),
                code: "MAX_LENGTH".to_string(),
                message: "plan must not exceed 2000 characters".to_string(),
                value: None,
            });
        }
    }

    if !field_errors.is_empty() {
        return Err(ApiError::validation("Validation failed", field_errors));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /emr/consultations
pub async fn create_consultation(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateConsultationBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    auth_mw::require_roles(
        &state.db,
        &headers,
        &state.config.auth_secret,
        &["provider"],
    )
    .await?;

    // require_roles returns user; we re-authenticate to get user for created_by
    let user = auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

    validate_create_consultation(&body)?;

    let consultation = repo::create_consultation(&state.db, &user.id, &body).await?;

    Ok((StatusCode::CREATED, Json(consultation)))
}

/// GET /emr/consultations
pub async fn list_consultations(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListConsultationsQuery>,
) -> Result<Json<PaginatedResult<serde_json::Value>>, ApiError> {
    auth_mw::require_roles(
        &state.db,
        &headers,
        &state.config.auth_secret,
        &["provider", "admin", "patient"],
    )
    .await?;

    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(20).min(100);

    let (data, total_count) =
        repo::list_consultations(&state.db, offset, limit, &query).await?;
    let count = data.len() as u64;

    Ok(Json(PaginatedResult {
        data,
        pagination: PaginationMeta::new(offset, limit, count, total_count),
    }))
}

/// GET /emr/consultations/:consultation
pub async fn get_consultation(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(consultation_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    let consultation = repo::get_consultation(&state.db, &consultation_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Consultation not found".to_string(),
            resource_type: Some("ConsultationNote".to_string()),
            resource: Some(consultation_id.clone()),
        })?;

    Ok(Json(consultation))
}

/// PATCH /emr/consultations/:consultation
pub async fn update_consultation(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(consultation_id): Path<String>,
    Json(body): Json<UpdateConsultationBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = auth_mw::require_roles(
        &state.db,
        &headers,
        &state.config.auth_secret,
        &["provider"],
    )
    .await?;

    validate_update_consultation(&body)?;

    let existing = repo::get_consultation(&state.db, &consultation_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Consultation not found"))?;

    // Only the creating provider may update
    let created_by = existing
        .get("createdBy")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if created_by != user.id && !user.has_any_role(&["admin"]) {
        return Err(ApiError::forbidden("Only the creating provider may update this consultation"));
    }

    // Only draft consultations may be updated
    let status = existing
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if status != "draft" {
        return Err(ApiError::BusinessLogic {
            message: "Only draft consultations can be updated".to_string(),
            code: "CONSULTATION_NOT_DRAFT".to_string(),
        });
    }

    let updated =
        repo::update_consultation(&state.db, &consultation_id, &user.id, &body).await?;

    Ok(Json(updated))
}

/// POST /emr/consultations/:consultation/finalize
pub async fn finalize_consultation(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(consultation_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = auth_mw::require_roles(
        &state.db,
        &headers,
        &state.config.auth_secret,
        &["provider"],
    )
    .await?;

    let existing = repo::get_consultation(&state.db, &consultation_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Consultation not found"))?;

    // Only the creating provider may finalize
    let created_by = existing
        .get("createdBy")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if created_by != user.id && !user.has_any_role(&["admin"]) {
        return Err(ApiError::forbidden(
            "Only the creating provider may finalize this consultation",
        ));
    }

    // Must be in draft status
    let status = existing
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if status != "draft" {
        return Err(ApiError::BusinessLogic {
            message: "Only draft consultations can be finalized".to_string(),
            code: "CONSULTATION_NOT_DRAFT".to_string(),
        });
    }

    let finalized =
        repo::finalize_consultation(&state.db, &consultation_id, &user.id).await?;

    Ok(Json(finalized))
}

/// GET /emr/patients
pub async fn list_emr_patients(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListEMRPatientsQuery>,
) -> Result<Json<PaginatedResult<serde_json::Value>>, ApiError> {
    auth_mw::require_roles(
        &state.db,
        &headers,
        &state.config.auth_secret,
        &["provider", "admin"],
    )
    .await?;

    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(20).min(100);

    let (data, total_count) = repo::list_emr_patients(
        &state.db,
        query.provider_id.as_deref(),
        offset,
        limit,
        query.q.as_deref(),
    )
    .await?;
    let count = data.len() as u64;

    Ok(Json(PaginatedResult {
        data,
        pagination: PaginationMeta::new(offset, limit, count, total_count),
    }))
}
