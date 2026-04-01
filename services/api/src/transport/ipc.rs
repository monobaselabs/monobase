use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::ApiError;

/// IPC request — simple struct, no HTTP dependencies.
/// Used by embedded mode (Tauri) for direct function calls.
#[derive(Debug, Clone, Deserialize)]
pub struct IpcRequest {
    pub method: String,
    pub path: String,
    pub body: Option<serde_json::Value>,
    pub headers: HashMap<String, String>,
}

/// IPC response — simple struct, no HTTP dependencies.
#[derive(Debug, Clone, Serialize)]
pub struct IpcResponse {
    pub status: u16,
    pub body: serde_json::Value,
}

impl IpcResponse {
    pub fn success<T: Serialize>(data: T) -> Self {
        Self {
            status: 200,
            body: serde_json::to_value(data).unwrap_or_default(),
        }
    }

    pub fn created<T: Serialize>(data: T) -> Self {
        Self {
            status: 201,
            body: serde_json::to_value(data).unwrap_or_default(),
        }
    }

    pub fn error(e: &ApiError) -> Self {
        let (status, code, message) = match e {
            ApiError::NotFound { message, .. } => (404, "NOT_FOUND", message.clone()),
            ApiError::Unauthorized { message } => (401, "UNAUTHORIZED", message.clone()),
            ApiError::Forbidden { message } => (403, "FORBIDDEN", message.clone()),
            ApiError::Validation { message, .. } => (400, "VALIDATION_ERROR", message.clone()),
            ApiError::BusinessLogic { message, code } => (422, code.as_str(), message.clone()),
            ApiError::Conflict { message } => (409, "CONFLICT", message.clone()),
            _ => (500, "INTERNAL_ERROR", "Internal server error".to_string()),
        };
        Self {
            status,
            body: serde_json::json!({ "message": message, "code": code }),
        }
    }

    pub fn not_found() -> Self {
        Self {
            status: 404,
            body: serde_json::json!({ "message": "Not found", "code": "NOT_FOUND" }),
        }
    }
}
