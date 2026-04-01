use axum::http::{HeaderMap, header};
use sea_orm::DatabaseConnection;

use crate::auth::{session, AuthUser};
use crate::error::ApiError;

/// Extract authenticated user from request headers.
/// Checks `Authorization: Bearer {token}` header first, then
/// `better-auth.session_token` cookie.
pub async fn extract_auth(
    db: &DatabaseConnection,
    headers: &HeaderMap,
    auth_secret: &str,
) -> Result<Option<AuthUser>, ApiError> {
    // Try Authorization header first
    if let Some(auth_header) = headers.get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                let raw_token = session::extract_bearer_token(auth_str, auth_secret)?;
                return session::get_user_by_token(db, &raw_token).await;
            }
        }
    }

    // Try cookie
    if let Some(cookie_header) = headers.get(header::COOKIE) {
        if let Ok(cookies) = cookie_header.to_str() {
            for cookie in cookies.split(';') {
                let cookie = cookie.trim();
                // Better-Auth cookie names
                for name in &[
                    "better-auth.session_token",
                    "__Secure-better-auth.session_token",
                ] {
                    if let Some(value) = cookie.strip_prefix(&format!("{}=", name)) {
                        let raw_token = session::verify_signed_token(value, auth_secret)?;
                        return session::get_user_by_token(db, &raw_token).await;
                    }
                }
            }
        }
    }

    Ok(None)
}

/// Require authentication — returns error if not authenticated.
pub async fn require_auth(
    db: &DatabaseConnection,
    headers: &HeaderMap,
    auth_secret: &str,
) -> Result<AuthUser, ApiError> {
    extract_auth(db, headers, auth_secret)
        .await?
        .ok_or_else(|| ApiError::unauthorized("Authentication required"))
}

/// Require authentication with specific roles (OR logic).
pub async fn require_roles(
    db: &DatabaseConnection,
    headers: &HeaderMap,
    auth_secret: &str,
    roles: &[&str],
) -> Result<AuthUser, ApiError> {
    let user = require_auth(db, headers, auth_secret).await?;
    if user.has_any_role(roles) {
        Ok(user)
    } else {
        Err(ApiError::forbidden("Insufficient permissions"))
    }
}
