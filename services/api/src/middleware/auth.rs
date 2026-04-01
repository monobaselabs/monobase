//! Authentication middleware.
//! Extracts bearer token from Authorization header and resolves AuthUser.

use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;

use crate::auth::backend::{AuthUser, get_user_by_token};
use crate::handlers::AppState;

/// Helper to extract auth user from headers using AppState.
pub async fn extract_auth(state: &AppState, headers: &axum::http::HeaderMap) -> Option<AuthUser> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))?;
    get_user_by_token(&state.db, token).await.ok()?
}

/// Axum middleware layer that requires authentication.
/// Returns 401 if no valid bearer token is present.
pub async fn require_auth(
    State(state): State<Arc<AppState>>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let token = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    match token {
        Some(token) => {
            match get_user_by_token(&state.db, token).await {
                Ok(Some(_user)) => {
                    // User is authenticated — proceed
                    next.run(request).await
                }
                _ => {
                    // Invalid or expired token
                    (StatusCode::UNAUTHORIZED, Json(json!({"message": "Unauthorized"}))).into_response()
                }
            }
        }
        None => {
            // No authorization header
            (StatusCode::UNAUTHORIZED, Json(json!({"message": "Unauthorized"}))).into_response()
        }
    }
}
