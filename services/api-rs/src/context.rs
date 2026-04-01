use std::collections::HashMap;

use crate::auth::AuthUser;

/// Transport-agnostic request context.
/// Shared between HTTP (Axum) and IPC (embedded) transports.
#[derive(Debug, Clone)]
pub struct ServiceContext {
    /// Authenticated user, if any.
    pub user: Option<AuthUser>,
    /// Query parameters as JSON.
    pub query: serde_json::Value,
    /// Path parameters.
    pub params: HashMap<String, String>,
    /// Transport provider: "rest" or "ipc".
    pub provider: String,
    /// Request ID for tracing.
    pub request_id: String,
}

impl ServiceContext {
    pub fn new_rest(user: Option<AuthUser>, request_id: String) -> Self {
        Self {
            user,
            query: serde_json::Value::Object(serde_json::Map::new()),
            params: HashMap::new(),
            provider: "rest".to_string(),
            request_id,
        }
    }

    pub fn new_ipc(user: Option<AuthUser>) -> Self {
        Self {
            user,
            query: serde_json::Value::Object(serde_json::Map::new()),
            params: HashMap::new(),
            provider: "ipc".to_string(),
            request_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    /// Require an authenticated user, or return Unauthorized.
    pub fn require_user(&self) -> Result<&AuthUser, crate::error::ApiError> {
        self.user
            .as_ref()
            .ok_or_else(|| crate::error::ApiError::unauthorized("Authentication required"))
    }

    /// Require the user to have one of the given roles.
    pub fn require_role(&self, allowed_roles: &[&str]) -> Result<&AuthUser, crate::error::ApiError> {
        let user = self.require_user()?;
        if user.has_any_role(allowed_roles) {
            Ok(user)
        } else {
            Err(crate::error::ApiError::forbidden(
                "Insufficient permissions",
            ))
        }
    }
}
