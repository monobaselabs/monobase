use axum::{
    extract::{FromRequestParts, State},
    http::request::Parts,
};
use std::sync::Arc;

use super::backend::{AuthUser, get_user_by_token};
use crate::handlers::AppState;

/// Extract authenticated user from request headers
/// Looks for `Authorization: Bearer <token>` header
#[derive(Debug, Clone)]
pub struct CurrentUser(pub AuthUser);

impl CurrentUser {
    pub fn user(&self) -> &AuthUser {
        &self.0
    }
}

/// Optional user extraction (doesn't fail for anonymous requests)
#[derive(Debug, Clone)]
pub struct OptionalUser(pub Option<AuthUser>);

/// Extract bearer token from authorization header
fn extract_bearer_token(parts: &Parts) -> Option<String> {
    parts
        .headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|t| t.to_string())
}
