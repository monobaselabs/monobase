use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

/// Application error type that converts to HTTP responses
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Not found")]
    NotFound,

    #[error("Not authenticated")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error(transparent)]
    Database(#[from] sea_orm::DbErr),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

impl ApiError {
    pub fn status_code(&self) -> u16 {
        match self {
            Self::NotFound => 404,
            Self::Unauthorized => 401,
            Self::Forbidden => 403,
            Self::BadRequest(_) => 400,
            Self::Conflict(_) => 409,
            Self::Validation(_) => 422,
            Self::Internal(_) | Self::Database(_) | Self::Anyhow(_) => 500,
        }
    }

    pub fn message(&self) -> String {
        self.to_string()
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        // Log internal errors
        if self.status_code() >= 500 {
            tracing::error!("Internal error: {}", self);
        }

        let body = json!({
            "message": self.message(),
            "statusCode": self.status_code(),
        });

        (status, axum::Json(body)).into_response()
    }
}
