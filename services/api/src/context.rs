use std::collections::HashMap;

/// Transport-agnostic request context.
/// Created from HTTP requests OR IPC messages.
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub method: String,
    pub path: String,
    pub path_params: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
    pub body: Option<serde_json::Value>,
    pub headers: HashMap<String, String>,
    pub auth: Option<AuthContext>,
}

/// Authentication context extracted from request
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: String,
    pub email: String,
    pub organization_id: Option<String>,
    pub roles: Vec<String>,
}

/// Service response wrapper
#[derive(Debug, Clone, serde::Serialize)]
pub struct ServiceResponse {
    pub status: u16,
    pub body: serde_json::Value,
}

impl ServiceResponse {
    pub fn ok(body: serde_json::Value) -> Self {
        Self { status: 200, body }
    }

    pub fn created(body: serde_json::Value) -> Self {
        Self { status: 201, body }
    }

    pub fn no_content() -> Self {
        Self {
            status: 204,
            body: serde_json::Value::Null,
        }
    }
}
