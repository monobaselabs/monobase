use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::ApiError;

/// IPC request - simple struct, no HTTP dependencies
#[derive(Debug, Clone, Deserialize)]
pub struct IpcRequest {
    pub method: String,
    pub path: String,
    pub body: Option<serde_json::Value>,
    pub headers: HashMap<String, String>,
}

/// IPC response - simple struct, no HTTP dependencies
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

    pub fn error(e: ApiError) -> Self {
        Self {
            status: e.status_code(),
            body: serde_json::json!({ "error": e.message() }),
        }
    }

    pub fn not_found() -> Self {
        Self {
            status: 404,
            body: serde_json::json!({ "error": "Not found" }),
        }
    }
}
