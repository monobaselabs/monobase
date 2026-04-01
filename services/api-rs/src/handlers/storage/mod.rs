mod repo;

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;

use crate::error::ApiError;
use crate::handlers::AppState;
use crate::middleware::auth as auth_mw;
use crate::model::{PaginatedResult, PaginationMeta};

// ---------------------------------------------------------------------------
// Request / query types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadFileBody {
    pub filename: String,
    pub mime_type: String,
    pub size: u64,
    pub owner_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListFilesQuery {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    pub owner_id: Option<String>,
    pub status: Option<String>,
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

fn validate_upload_file(body: &UploadFileBody) -> Result<(), ApiError> {
    use crate::error::FieldError;
    let mut field_errors: Vec<FieldError> = Vec::new();

    if body.filename.is_empty() {
        field_errors.push(FieldError {
            field: "filename".to_string(),
            code: "REQUIRED".to_string(),
            message: "filename is required".to_string(),
            value: None,
        });
    }

    if body.filename.len() > 255 {
        field_errors.push(FieldError {
            field: "filename".to_string(),
            code: "MAX_LENGTH".to_string(),
            message: "filename must not exceed 255 characters".to_string(),
            value: None,
        });
    }

    if body.mime_type.is_empty() {
        field_errors.push(FieldError {
            field: "mimeType".to_string(),
            code: "REQUIRED".to_string(),
            message: "mimeType is required".to_string(),
            value: None,
        });
    }

    if body.mime_type.len() > 100 {
        field_errors.push(FieldError {
            field: "mimeType".to_string(),
            code: "MAX_LENGTH".to_string(),
            message: "mimeType must not exceed 100 characters".to_string(),
            value: None,
        });
    }

    if body.size == 0 {
        field_errors.push(FieldError {
            field: "size".to_string(),
            code: "MIN_VALUE".to_string(),
            message: "size must be greater than 0".to_string(),
            value: Some(serde_json::json!(0)),
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

    if !field_errors.is_empty() {
        return Err(ApiError::validation("Validation failed", field_errors));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /storage/files
/// Auth: authenticated (any role)
pub async fn list_files(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListFilesQuery>,
) -> Result<Json<PaginatedResult<serde_json::Value>>, ApiError> {
    auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(20).min(100);

    let (data, total_count) = repo::list_stored_files(&state.db, offset, limit, &query).await?;
    let count = data.len() as u64;

    Ok(Json(PaginatedResult {
        data,
        pagination: PaginationMeta::new(offset, limit, count, total_count),
    }))
}

/// POST /storage/files/upload
/// Auth: user
/// Creates a stored_file record with status='uploading' and returns a presigned upload URL.
pub async fn upload_file(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<UploadFileBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    validate_upload_file(&body)?;

    let file = repo::create_stored_file(&state.db, &user.id, &body).await?;

    let file_id = file
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Placeholder presigned upload URL — actual S3 presigning is performed in the service layer.
    let upload_url = format!("/storage/internal/upload/{}", file_id);

    let response = serde_json::json!({
        "file": file,
        "uploadUrl": upload_url,
    });

    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /storage/files/:file
/// Auth: admin or owner
pub async fn get_file(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(file_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = auth_mw::require_roles(
        &state.db,
        &headers,
        &state.config.auth_secret,
        &["admin", "user"],
    )
    .await?;

    let file = repo::get_stored_file(&state.db, &file_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "File not found".to_string(),
            resource_type: Some("StoredFile".to_string()),
            resource: Some(file_id.clone()),
        })?;

    // Non-admin users may only access files they own.
    if !user.has_any_role(&["admin"]) {
        let owner_id = file
            .get("owner_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if owner_id != user.id {
            return Err(ApiError::forbidden("Access denied"));
        }
    }

    Ok(Json(file))
}

/// DELETE /storage/files/:file
/// Auth: owner only
pub async fn delete_file(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(file_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    let file = repo::get_stored_file(&state.db, &file_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "File not found".to_string(),
            resource_type: Some("StoredFile".to_string()),
            resource: Some(file_id.clone()),
        })?;

    let owner_id = file
        .get("owner_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if owner_id != user.id && !user.has_any_role(&["admin"]) {
        return Err(ApiError::forbidden(
            "Only the file owner may delete this file",
        ));
    }

    repo::delete_stored_file(&state.db, &file_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /storage/files/:file/complete
/// Auth: owner only
/// Transitions the file from 'uploading' to 'available'.
pub async fn complete_file_upload(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(file_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["user"]).await?;

    let file = repo::get_stored_file(&state.db, &file_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "File not found".to_string(),
            resource_type: Some("StoredFile".to_string()),
            resource: Some(file_id.clone()),
        })?;

    let owner_id = file
        .get("owner_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if owner_id != user.id && !user.has_any_role(&["admin"]) {
        return Err(ApiError::forbidden(
            "Only the file owner may complete this upload",
        ));
    }

    let current_status = file
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if current_status != "uploading" {
        return Err(ApiError::BusinessLogic {
            message: format!(
                "Cannot complete upload: file is in '{}' state, expected 'uploading'",
                current_status
            ),
            code: "INVALID_FILE_STATUS".to_string(),
        });
    }

    let updated = repo::complete_stored_file(&state.db, &file_id, &user.id).await?;

    Ok(Json(updated))
}

/// GET /storage/files/:file/download
/// Auth: admin or owner
/// Returns a presigned download URL for the file.
pub async fn get_file_download(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(file_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = auth_mw::require_roles(
        &state.db,
        &headers,
        &state.config.auth_secret,
        &["admin", "user"],
    )
    .await?;

    let file = repo::get_stored_file(&state.db, &file_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "File not found".to_string(),
            resource_type: Some("StoredFile".to_string()),
            resource: Some(file_id.clone()),
        })?;

    // Non-admin users may only download files they own.
    if !user.has_any_role(&["admin"]) {
        let owner_id = file
            .get("owner_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if owner_id != user.id {
            return Err(ApiError::forbidden("Access denied"));
        }
    }

    let current_status = file
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if current_status != "available" {
        return Err(ApiError::BusinessLogic {
            message: format!(
                "File is not available for download: status is '{}'",
                current_status
            ),
            code: "FILE_NOT_AVAILABLE".to_string(),
        });
    }

    // Placeholder presigned download URL — actual S3 presigning is performed in the service layer.
    let download_url = format!("/storage/internal/download/{}", file_id);

    Ok(Json(serde_json::json!({
        "fileId": file_id,
        "downloadUrl": download_url,
        "expiresIn": 3600,
    })))
}
