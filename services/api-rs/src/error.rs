use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// API error type — all handlers return `Result<T, ApiError>`.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{message}")]
    NotFound {
        message: String,
        resource_type: Option<String>,
        resource: Option<String>,
    },

    #[error("{message}")]
    Unauthorized { message: String },

    #[error("{message}")]
    Forbidden { message: String },

    #[error("{message}")]
    Validation {
        message: String,
        field_errors: Vec<FieldError>,
        global_errors: Vec<String>,
    },

    #[error("{message}")]
    BusinessLogic { message: String, code: String },

    #[error("{message}")]
    Conflict { message: String },

    #[error("{message}")]
    RateLimit { message: String },

    #[error("{message}")]
    Timeout {
        message: String,
        timeout_ms: u64,
        retryable: bool,
    },

    #[error("{message}")]
    ExternalService {
        message: String,
        service: String,
        retryable: bool,
    },

    #[error("{message}")]
    Internal { message: String },

    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FieldError {
    pub field: String,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorResponse {
    message: String,
    code: String,
    status_code: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<String>,
    timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    field_errors: Option<Vec<FieldError>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    global_errors: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resource_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resource: Option<String>,
}

impl ApiError {
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound {
            message: message.into(),
            resource_type: None,
            resource: None,
        }
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::Unauthorized {
            message: message.into(),
        }
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::Forbidden {
            message: message.into(),
        }
    }

    pub fn validation(message: impl Into<String>, field_errors: Vec<FieldError>) -> Self {
        Self::Validation {
            message: message.into(),
            field_errors,
            global_errors: vec![],
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict {
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound { .. } => StatusCode::NOT_FOUND,
            Self::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
            Self::Forbidden { .. } => StatusCode::FORBIDDEN,
            Self::Validation { .. } => StatusCode::BAD_REQUEST,
            Self::BusinessLogic { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Conflict { .. } => StatusCode::CONFLICT,
            Self::RateLimit { .. } => StatusCode::TOO_MANY_REQUESTS,
            Self::Timeout { .. } => StatusCode::REQUEST_TIMEOUT,
            Self::ExternalService { .. } => StatusCode::SERVICE_UNAVAILABLE,
            Self::Internal { .. } | Self::Database(_) | Self::Anyhow(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    fn error_code(&self) -> &str {
        match self {
            Self::NotFound { .. } => "NOT_FOUND",
            Self::Unauthorized { .. } => "UNAUTHORIZED",
            Self::Forbidden { .. } => "FORBIDDEN",
            Self::Validation { .. } => "VALIDATION_ERROR",
            Self::BusinessLogic { code, .. } => code.as_str(),
            Self::Conflict { .. } => "CONFLICT",
            Self::RateLimit { .. } => "RATE_LIMIT",
            Self::Timeout { .. } => "TIMEOUT_ERROR",
            Self::ExternalService { .. } => "EXTERNAL_SERVICE_ERROR",
            Self::Internal { .. } | Self::Database(_) | Self::Anyhow(_) => "INTERNAL_ERROR",
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();

        // Log server errors
        if status.is_server_error() {
            tracing::error!(error = %self, "Internal server error");
        }

        let (field_errors, global_errors, resource_type, resource) = match &self {
            ApiError::Validation {
                field_errors,
                global_errors,
                ..
            } => (
                Some(field_errors.clone()),
                if global_errors.is_empty() {
                    None
                } else {
                    Some(global_errors.clone())
                },
                None,
                None,
            ),
            ApiError::NotFound {
                resource_type,
                resource,
                ..
            } => (None, None, resource_type.clone(), resource.clone()),
            _ => (None, None, None, None),
        };

        let body = ErrorResponse {
            message: self.to_string(),
            code: self.error_code().to_string(),
            status_code: status.as_u16(),
            request_id: None, // TODO: extract from request extensions
            timestamp: chrono::Utc::now().to_rfc3339(),
            field_errors,
            global_errors,
            resource_type,
            resource,
        };

        (status, Json(body)).into_response()
    }
}
