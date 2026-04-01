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
pub struct CreateEmailTemplateBody {
    /// Template display name (unique per system).
    pub name: String,
    pub description: Option<String>,
    /// Email subject line — max 500 characters.
    pub subject: String,
    /// Full HTML body.
    pub body_html: String,
    /// Optional plain-text fallback.
    pub body_text: Option<String>,
    /// Arbitrary tag metadata for categorisation.
    pub tags: Option<serde_json::Value>,
    /// Variable definitions used in the template (e.g. `{"name": "string"}`).
    pub variables: serde_json::Value,
    pub from_name: Option<String>,
    pub from_email: Option<String>,
    pub reply_to_email: Option<String>,
    pub reply_to_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEmailTemplateBody {
    pub name: Option<String>,
    pub description: Option<String>,
    pub subject: Option<String>,
    pub body_html: Option<String>,
    pub body_text: Option<String>,
    pub tags: Option<serde_json::Value>,
    pub variables: Option<serde_json::Value>,
    pub from_name: Option<String>,
    pub from_email: Option<String>,
    pub reply_to_email: Option<String>,
    pub reply_to_name: Option<String>,
    /// Allowed values: "draft", "active", "archived".
    pub status: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestEmailTemplateBody {
    /// Sample variable values to use when rendering the preview.
    pub variables: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListEmailTemplatesQuery {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    /// Filter by status: "draft", "active", or "archived".
    pub status: Option<String>,
    pub q: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListEmailQueueQuery {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    /// Filter by queue item status.
    pub status: Option<String>,
    pub template_id: Option<String>,
    pub recipient_email: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelEmailQueueItemBody {
    /// Human-readable cancellation reason.
    pub reason: Option<String>,
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

fn validate_create_template(body: &CreateEmailTemplateBody) -> Result<(), ApiError> {
    let mut field_errors: Vec<FieldError> = Vec::new();

    if body.name.is_empty() {
        field_errors.push(FieldError {
            field: "name".to_string(),
            code: "REQUIRED".to_string(),
            message: "name is required".to_string(),
            value: None,
        });
    } else if body.name.len() > 255 {
        field_errors.push(FieldError {
            field: "name".to_string(),
            code: "MAX_LENGTH".to_string(),
            message: "name must not exceed 255 characters".to_string(),
            value: None,
        });
    }

    if body.subject.is_empty() {
        field_errors.push(FieldError {
            field: "subject".to_string(),
            code: "REQUIRED".to_string(),
            message: "subject is required".to_string(),
            value: None,
        });
    } else if body.subject.len() > 500 {
        field_errors.push(FieldError {
            field: "subject".to_string(),
            code: "MAX_LENGTH".to_string(),
            message: "subject must not exceed 500 characters".to_string(),
            value: None,
        });
    }

    if body.body_html.is_empty() {
        field_errors.push(FieldError {
            field: "bodyHtml".to_string(),
            code: "REQUIRED".to_string(),
            message: "bodyHtml is required".to_string(),
            value: None,
        });
    }

    if !field_errors.is_empty() {
        return Err(ApiError::validation("Validation failed", field_errors));
    }

    Ok(())
}

fn validate_update_template(body: &UpdateEmailTemplateBody) -> Result<(), ApiError> {
    let mut field_errors: Vec<FieldError> = Vec::new();

    if let Some(ref name) = body.name {
        if name.is_empty() {
            field_errors.push(FieldError {
                field: "name".to_string(),
                code: "REQUIRED".to_string(),
                message: "name must not be empty".to_string(),
                value: None,
            });
        } else if name.len() > 255 {
            field_errors.push(FieldError {
                field: "name".to_string(),
                code: "MAX_LENGTH".to_string(),
                message: "name must not exceed 255 characters".to_string(),
                value: None,
            });
        }
    }

    if let Some(ref subject) = body.subject {
        if subject.len() > 500 {
            field_errors.push(FieldError {
                field: "subject".to_string(),
                code: "MAX_LENGTH".to_string(),
                message: "subject must not exceed 500 characters".to_string(),
                value: None,
            });
        }
    }

    let valid_statuses = ["draft", "active", "archived"];
    if let Some(ref status) = body.status {
        if !valid_statuses.contains(&status.as_str()) {
            field_errors.push(FieldError {
                field: "status".to_string(),
                code: "INVALID".to_string(),
                message: "status must be one of: draft, active, archived".to_string(),
                value: Some(serde_json::json!(status)),
            });
        }
    }

    if !field_errors.is_empty() {
        return Err(ApiError::validation("Validation failed", field_errors));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Handlers — email queue
// ---------------------------------------------------------------------------

/// GET /email/queue
pub async fn list_email_queue_items(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListEmailQueueQuery>,
) -> Result<Json<PaginatedResult<serde_json::Value>>, ApiError> {
    auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["admin"]).await?;

    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(20).min(100);

    let (data, total_count) =
        repo::list_email_queue_items(&state.db, offset, limit, &query).await?;
    let count = data.len() as u64;

    Ok(Json(PaginatedResult {
        data,
        pagination: PaginationMeta::new(offset, limit, count, total_count),
    }))
}

/// GET /email/queue/:queue
pub async fn get_email_queue_item(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(queue_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["admin"]).await?;

    let item = repo::get_email_queue_item(&state.db, &queue_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Email queue item not found".to_string(),
            resource_type: Some("EmailQueueItem".to_string()),
            resource: Some(queue_id.clone()),
        })?;

    Ok(Json(item))
}

/// POST /email/queue/:queue/cancel
pub async fn cancel_email_queue_item(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(queue_id): Path<String>,
    Json(body): Json<CancelEmailQueueItemBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["admin"]).await?;

    let existing = repo::get_email_queue_item(&state.db, &queue_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Email queue item not found"))?;

    let status = existing
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if status != "pending" {
        return Err(ApiError::BusinessLogic {
            message: "Only pending email queue items can be cancelled".to_string(),
            code: "EMAIL_QUEUE_NOT_PENDING".to_string(),
        });
    }

    let cancelled = repo::cancel_email_queue_item(&state.db, &queue_id, &user.id, &body).await?;

    Ok(Json(cancelled))
}

/// POST /email/queue/:queue/retry
pub async fn retry_email_queue_item(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(queue_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["admin"]).await?;

    let existing = repo::get_email_queue_item(&state.db, &queue_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Email queue item not found"))?;

    let status = existing
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if status != "failed" {
        return Err(ApiError::BusinessLogic {
            message: "Only failed email queue items can be retried".to_string(),
            code: "EMAIL_QUEUE_NOT_FAILED".to_string(),
        });
    }

    let retried = repo::retry_email_queue_item(&state.db, &queue_id, &user.id).await?;

    Ok(Json(retried))
}

// ---------------------------------------------------------------------------
// Handlers — email templates
// ---------------------------------------------------------------------------

/// GET /email/templates
pub async fn list_email_templates(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListEmailTemplatesQuery>,
) -> Result<Json<PaginatedResult<serde_json::Value>>, ApiError> {
    auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["admin"]).await?;

    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(20).min(100);

    let (data, total_count) =
        repo::list_email_templates(&state.db, offset, limit, &query).await?;
    let count = data.len() as u64;

    Ok(Json(PaginatedResult {
        data,
        pagination: PaginationMeta::new(offset, limit, count, total_count),
    }))
}

/// POST /email/templates
pub async fn create_email_template(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateEmailTemplateBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["admin"]).await?;

    validate_create_template(&body)?;

    let template = repo::create_email_template(&state.db, &user.id, &body).await?;

    Ok((StatusCode::CREATED, Json(template)))
}

/// GET /email/templates/:template
pub async fn get_email_template(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(template_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["admin"]).await?;

    let template = repo::get_email_template(&state.db, &template_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Email template not found".to_string(),
            resource_type: Some("EmailTemplate".to_string()),
            resource: Some(template_id.clone()),
        })?;

    Ok(Json(template))
}

/// PATCH /email/templates/:template
pub async fn update_email_template(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(template_id): Path<String>,
    Json(body): Json<UpdateEmailTemplateBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user =
        auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["admin"]).await?;

    validate_update_template(&body)?;

    // Ensure template exists
    repo::get_email_template(&state.db, &template_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Email template not found"))?;

    let updated =
        repo::update_email_template(&state.db, &template_id, &user.id, &body).await?;

    Ok(Json(updated))
}

/// POST /email/templates/:template/test
///
/// Returns a preview render of the template with sample variable substitution.
/// This endpoint does not send any email; it only returns the rendered preview.
pub async fn test_email_template(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(template_id): Path<String>,
    Json(body): Json<TestEmailTemplateBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth_mw::require_roles(&state.db, &headers, &state.config.auth_secret, &["admin"]).await?;

    let template = repo::get_email_template(&state.db, &template_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Email template not found".to_string(),
            resource_type: Some("EmailTemplate".to_string()),
            resource: Some(template_id.clone()),
        })?;

    // Build sample variable map from the template's variable definitions and
    // any caller-supplied override values.
    let template_vars = template
        .get("variables")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    let sample_vars = match body.variables {
        Some(caller_vars) => {
            // Merge: caller values override template defaults
            let mut merged = template_vars.clone();
            if let (Some(m), Some(c)) = (merged.as_object_mut(), caller_vars.as_object()) {
                for (k, v) in c {
                    m.insert(k.clone(), v.clone());
                }
            }
            merged
        }
        None => template_vars,
    };

    // Perform naive variable substitution: replace `{{key}}` placeholders in
    // subject and body_html with sample values.
    let subject = template
        .get("subject")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let body_html = template
        .get("body_html")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let body_text = template
        .get("body_text")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let (rendered_subject, rendered_html, rendered_text) =
        apply_template_variables(subject, body_html, body_text, &sample_vars);

    Ok(Json(serde_json::json!({
        "templateId": template_id,
        "renderedSubject": rendered_subject,
        "renderedBodyHtml": rendered_html,
        "renderedBodyText": rendered_text,
        "sampleVariables": sample_vars,
    })))
}

// ---------------------------------------------------------------------------
// Template rendering helpers
// ---------------------------------------------------------------------------

/// Replace `{{variable_name}}` placeholders using the provided variable map.
fn apply_template_variables(
    subject: String,
    body_html: String,
    body_text: Option<String>,
    variables: &serde_json::Value,
) -> (String, String, Option<String>) {
    let replacer = |mut text: String| -> String {
        if let Some(map) = variables.as_object() {
            for (key, value) in map {
                let placeholder = format!("{{{{{}}}}}", key);
                let replacement = match value {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                text = text.replace(&placeholder, &replacement);
            }
        }
        text
    };

    let rendered_subject = replacer(subject);
    let rendered_html = replacer(body_html);
    let rendered_text = body_text.map(replacer);

    (rendered_subject, rendered_html, rendered_text)
}
