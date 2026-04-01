mod repo;

use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    http::HeaderMap,
};
use serde::Deserialize;

use crate::error::ApiError;
use crate::handlers::AppState;
use crate::middleware::auth as auth_mw;
use crate::model::{PaginatedResult, PaginationMeta};

// ---------------------------------------------------------------------------
// Valid enumeration values (validated at the handler layer)
// ---------------------------------------------------------------------------

const VALID_EVENT_TYPES: &[&str] = &[
    "authentication",
    "data-access",
    "data-modification",
    "system-config",
    "security",
    "compliance",
];

const VALID_CATEGORIES: &[&str] = &[
    "hipaa",
    "security",
    "privacy",
    "administrative",
    "clinical",
    "financial",
];

const VALID_ACTIONS: &[&str] = &[
    "create", "read", "update", "delete", "login", "logout",
];

const VALID_OUTCOMES: &[&str] = &["success", "failure", "partial", "denied"];

// ---------------------------------------------------------------------------
// Query type
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListAuditLogsQuery {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    /// Filter by event type (authentication, data-access, data-modification,
    /// system-config, security, compliance).
    pub event_type: Option<String>,
    /// Filter by category (hipaa, security, privacy, administrative,
    /// clinical, financial).
    pub category: Option<String>,
    /// Filter by action (create, read, update, delete, login, logout).
    pub action: Option<String>,
    /// Filter by outcome (success, failure, partial, denied).
    pub outcome: Option<String>,
    /// Filter by the user who triggered the event.
    pub user_id: Option<String>,
    /// Filter by the resource type that was acted upon.
    pub resource_type: Option<String>,
    /// ISO 8601 lower bound for `created_at`.
    pub date_from: Option<String>,
    /// ISO 8601 upper bound for `created_at`.
    pub date_to: Option<String>,
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

fn validate_enum_filter(
    value: &Option<String>,
    valid: &[&str],
    field: &str,
) -> Result<(), ApiError> {
    if let Some(ref v) = value {
        if !valid.contains(&v.as_str()) {
            return Err(ApiError::validation(
                format!("Invalid value for {}", field),
                vec![crate::error::FieldError {
                    field: field.to_string(),
                    code: "INVALID_VALUE".to_string(),
                    message: format!(
                        "Must be one of: {}",
                        valid.join(", ")
                    ),
                    value: Some(serde_json::json!(v)),
                }],
            ));
        }
    }
    Ok(())
}

fn validate_query(query: &ListAuditLogsQuery) -> Result<(), ApiError> {
    validate_enum_filter(&query.event_type, VALID_EVENT_TYPES, "eventType")?;
    validate_enum_filter(&query.category, VALID_CATEGORIES, "category")?;
    validate_enum_filter(&query.action, VALID_ACTIONS, "action")?;
    validate_enum_filter(&query.outcome, VALID_OUTCOMES, "outcome")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// GET /audit/logs
///
/// Returns a paginated list of audit log entries. Restricted to admin and
/// support roles.
pub async fn list_audit_logs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListAuditLogsQuery>,
) -> Result<Json<PaginatedResult<serde_json::Value>>, ApiError> {
    auth_mw::require_roles(
        &state.db,
        &headers,
        &state.config.auth_secret,
        &["admin", "support"],
    )
    .await?;

    validate_query(&query)?;

    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(50).min(200);

    let (data, total_count) = repo::list_audit_logs(&state.db, offset, limit, &query).await?;
    let count = data.len() as u64;

    Ok(Json(PaginatedResult {
        data,
        pagination: PaginationMeta::new(offset, limit, count, total_count),
    }))
}
